//! Running games. Each one is a Mocktail process with its own account and profile.
//!
//! Mocktail allows one process per executable path. Vodka gives every running game
//! its own path (see `slots`), so several can run at once. Mocktail's authors do not
//! support this; see the warning in the UI.

pub mod proc;
pub mod profiles;
pub mod slots;

use crate::accounts;
use crate::error::{AppError, Result};
use crate::mocktail::detect::APP_ID;
use crate::mocktail::launch::{ExitReport, classify_exit, fps_between, is_running_marker, is_signin_failure, launch_uri, parse_outdated_client, parse_present_sample, redact};
use crate::roblox_api::Game;
use crate::settings::SavedAccount;
use profiles::ProfilePaths;
use serde::Serialize;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;

const LOG_LINES: usize = 400;

struct Instance {
    id: u32,
    slot: usize,
    account: SavedAccount,
    game: Game,
    pid: u32,
    started_at_ms: u64,
    playing: bool,
    restart: bool,
    /// The user (or Vodka itself) asked this game to stop, so its exit is not a crash.
    stopping: bool,
    /// Come back by itself after a crash or disconnect.
    auto_rejoin: bool,
    /// Set when Vodka stops the game because Roblox says the account is no longer in it.
    disconnected: bool,
    /// How many presence checks in a row said "not in a game".
    absent_checks: u8,
    log: VecDeque<String>,
    profile: ProfilePaths,
    sample: Option<(Instant, u64)>,
    cpu_percent: f64,
    frame_sample: Option<(u64, u64)>,
    fps: Option<f64>,
    env: Vec<(&'static str, String)>,
    uri: String,
    /// Held for as long as the game runs; dropping it frees the slot and the account for other windows.
    _claim: slots::Claim,
}

#[derive(Default)]
pub struct Manager {
    next_id: u32,
    instances: Vec<Instance>,
    /// When each account was last brought back automatically, to stop a crash loop.
    rejoins: Vec<(u64, Instant)>,
    /// When each account's launch last failed because Roblox's sign-in didn't answer.
    signin_failures: Vec<(u64, Instant)>,
}

/// A launch that fails at sign-in is tried again by itself this many times.
const SIGNIN_RETRIES: usize = 3;
const SIGNIN_WINDOW: std::time::Duration = std::time::Duration::from_secs(120);

/// How many sign-in failures this account has had recently, this one included.
pub fn signin_failures_recently(history: &[(u64, Instant)], account_id: u64, now: Instant) -> usize {
    history.iter().filter(|(id, at)| *id == account_id && now.duration_since(*at) < SIGNIN_WINDOW).count()
}

const REJOIN_LIMIT: usize = 5;
const REJOIN_WINDOW: std::time::Duration = std::time::Duration::from_secs(15 * 60);
/// A game must have been up this long before a "not in a game" answer counts as a disconnect.
const SETTLE_MS: u64 = 120_000;
const ABSENT_CHECKS_FOR_REJOIN: u8 = 2;

/// Whether another automatic rejoin is allowed for this account: at most five in fifteen minutes.
pub fn rejoin_allowed(history: &[(u64, Instant)], account_id: u64, now: Instant) -> bool {
    history.iter().filter(|(id, at)| *id == account_id && now.duration_since(*at) < REJOIN_WINDOW).count() < REJOIN_LIMIT
}

/// The link to come back with. A specific public server may be gone by then, so only
/// private-server links are kept; everything else rejoins the game on any server.
pub fn rejoin_uri(original: &str, place_id: u64) -> String {
    if original.contains("accessCode=") { original.to_owned() } else { launch_uri(place_id) }
}

pub type SharedManager = Arc<Mutex<Manager>>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceView {
    pub id: u32,
    pub slot: usize,
    pub account_id: u64,
    pub account_name: String,
    pub game: Game,
    /// "starting" or "playing".
    pub phase: &'static str,
    pub started_at_ms: u64,
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    /// Measured from Mocktail's frame counter; `None` until two samples have arrived.
    pub fps: Option<f64>,
    pub log: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExitEvent {
    id: u32,
    game: Game,
    account_name: String,
    report: ExitReport,
    log: Vec<String>,
    restarting: bool,
    /// Why Vodka is (or isn't) bringing the game back: "crash", "disconnect", or "limit" when it gave up.
    rejoin: Option<&'static str>,
}

/// Which slot a new game may take, or why it can't start.
pub fn pick_slot(used: &[(usize, u64)], account_id: u64, account_name: &str) -> Result<usize> {
    if used.iter().any(|(_, id)| *id == account_id) {
        return Err(AppError::Launch(format!(
            "{account_name} is already in a game. Close it, or switch to another account."
        )));
    }
    (0..slots::MAX_INSTANCES)
        .find(|slot| !used.iter().any(|(s, _)| s == slot))
        .ok_or_else(|| AppError::Launch(format!("Vodka runs at most {} games at once. Close one first.", slots::MAX_INSTANCES)))
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn flatpak_data() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or_else(|| AppError::Other("HOME is not set.".into()))?;
    Ok(profiles::flatpak_data(&PathBuf::from(home)))
}

/// The Mocktail binary inside the Flatpak's install folder, as seen from outside the sandbox.
fn installed_binary() -> Result<PathBuf> {
    let out = std::process::Command::new("flatpak")
        .args(["info", "--show-location", APP_ID])
        .env("LC_ALL", "C")
        .output()
        .map_err(|_| AppError::FlatpakMissing)?;
    let location = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if !out.status.success() || location.is_empty() {
        return Err(AppError::MocktailMissing);
    }
    Ok(PathBuf::from(location).join("files/bin/mocktail"))
}

fn pump<R: AsyncRead + Unpin + Send + 'static>(reader: R, app: AppHandle, manager: SharedManager, id: u32) {
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let line = redact(&line);
            let mut became_playing = false;
            {
                let mut m = manager.lock().unwrap();
                let Some(inst) = m.instances.iter_mut().find(|i| i.id == id) else { break };
                if inst.log.len() == LOG_LINES {
                    inst.log.pop_front();
                }
                inst.log.push_back(line.clone());
                if let Some(sample) = parse_present_sample(&line) {
                    if let Some(before) = inst.frame_sample {
                        inst.fps = fps_between(before, sample).or(inst.fps);
                    }
                    inst.frame_sample = Some(sample);
                }
                if !inst.playing && is_running_marker(&line) {
                    inst.playing = true;
                    inst.started_at_ms = now_ms();
                    became_playing = true;
                }
            }
            if became_playing {
                let _ = app.emit("instances://changed", ());
            }
            if let Some(outdated) = parse_outdated_client(&line) {
                let _ = app.emit("mocktail://outdated-client", outdated);
            }
        }
    });
}

/// Starts `game` as `account` in a free slot. Returns the new instance id.
pub async fn launch(
    app: AppHandle,
    manager: SharedManager,
    account: SavedAccount,
    game: Game,
    uri: String,
    env: Vec<(&'static str, String)>,
    auto_rejoin: bool,
) -> Result<u32> {
    // A quick answer for the common case; the lock files below are what actually decides.
    let used: Vec<(usize, u64)> = manager.lock().unwrap().instances.iter().map(|i| (i.slot, i.account.id)).collect();
    pick_slot(&used, account.id, &account.display_name)?;

    let account_id = account.id;
    let account_name = account.display_name.clone();
    let (cookie, profile, claim) = tauri::async_runtime::spawn_blocking(move || -> Result<(String, ProfilePaths, slots::Claim)> {
        let cookie = accounts::load_session(account_id)?.ok_or(AppError::NotSignedIn)?;
        let data = flatpak_data()?;
        // Claimed through lock files, so a second Vodka window can't pick the same slot or account.
        let claim = slots::claim(&slots::lock_dir(), account_id).map_err(|e| match e {
            slots::ClaimError::AccountBusy => AppError::Launch(format!(
                "{account_name} is already in a game, maybe from another Vodka window. Close it, or switch to another account."
            )),
            slots::ClaimError::Full => AppError::Launch(format!("Vodka runs at most {} games at once. Close one first.", slots::MAX_INSTANCES)),
            slots::ClaimError::Io(e) => AppError::Launch(format!("could not reserve a game slot: {e}")),
        })?;
        let profile = profiles::profile_paths(&data, account_id);
        profiles::prepare(&data.join("mocktail"), &profile)
            .map_err(|e| AppError::Launch(format!("could not prepare this account's profile: {e}")))?;
        // Games beyond the second run from their own copy of Mocktail's binary.
        if slots::needs_copy(claim.slot) {
            slots::prepare(&installed_binary()?, &slots::slot_dir(&data, claim.slot))
                .map_err(|e| AppError::Launch(format!("could not prepare another copy of Mocktail: {e}")))?;
        }
        Ok((cookie, profile, claim))
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))??;

    let mut m = manager.lock().unwrap();
    let slot = claim.slot;

    let mut cmd = Command::new("flatpak");
    cmd.arg("run");
    if let Some(command) = slots::command(&flatpak_data()?, slot) {
        cmd.arg(format!("--command={command}"));
    }
    cmd.args([APP_ID, &uri])
        .envs(env.iter().map(|(k, v)| (*k, v.as_str())))
        .env("MOCKTAIL_DATA_ROOT", &profile.data)
        .env("MOCKTAIL_CACHE_ROOT", &profile.cache)
        .env("MOCKTAIL_STATE_ROOT", &profile.state)
        // Vodka reports failures itself; Mocktail's own "detected a crash" window would only stack on top.
        .env("MOCKTAIL_DISABLE_FAILURE_DIALOG", "1")
        // Through the environment, so the session never shows up in a process list.
        .env("MOCKTAIL_ROBLOX_COOKIES", format!(".ROBLOSECURITY={cookie}"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound { AppError::FlatpakMissing } else { AppError::Launch(e.to_string()) }
    })?;

    m.next_id += 1;
    let id = m.next_id;
    m.instances.push(Instance {
        id,
        slot,
        account,
        game,
        pid: child.id().unwrap_or(0),
        started_at_ms: now_ms(),
        playing: false,
        restart: false,
        stopping: false,
        auto_rejoin,
        disconnected: false,
        absent_checks: 0,
        log: VecDeque::new(),
        profile,
        sample: None,
        cpu_percent: 0.0,
        frame_sample: None,
        fps: None,
        env,
        uri,
        _claim: claim,
    });
    drop(m);

    if let Some(out) = child.stdout.take() {
        pump(out, app.clone(), manager.clone(), id);
    }
    if let Some(err) = child.stderr.take() {
        pump(err, app.clone(), manager.clone(), id);
    }
    let _ = app.emit("instances://changed", ());

    tauri::async_runtime::spawn(async move {
        let status = child.wait().await;
        // Let the readers drain the last lines.
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        let finished = {
            let mut m = manager.lock().unwrap();
            m.instances.iter().position(|i| i.id == id).map(|at| m.instances.remove(at))
        };
        let Some(inst) = finished else { return };
        profiles::clear_session_copy(&inst.profile);
        let log: Vec<String> = inst.log.iter().cloned().collect();
        let report = match status {
            Ok(st) if inst.stopping => ExitReport { code: st.code(), outcome: "closed", detail: None },
            Ok(st) => classify_exit(st.code(), &log),
            Err(e) => ExitReport { code: None, outcome: "crashed", detail: Some(e.to_string()) },
        };
        let report = explain_kill(report, &inst.env);

        // A launch that died at sign-in never reached the game. Try it again quietly, whatever the
        // auto-rejoin setting says: the user asked for this game moments ago.
        let signin_failed = !inst.stopping && log.iter().any(|l| is_signin_failure(l));
        let mut report = report;
        let mut signin_attempt = 0;

        // Decide whether Vodka brings the game back by itself.
        let wants_rejoin = inst.auto_rejoin && (inst.disconnected || (!inst.stopping && report.outcome == "crashed"));
        let rejoin = if signin_failed {
            let mut m = manager.lock().unwrap();
            let now = Instant::now();
            m.signin_failures.retain(|(_, at)| now.duration_since(*at) < SIGNIN_WINDOW);
            m.signin_failures.push((inst.account.id, now));
            signin_attempt = signin_failures_recently(&m.signin_failures, inst.account.id, now);
            if signin_attempt <= SIGNIN_RETRIES {
                Some("signin")
            } else {
                report.detail = Some(format!(
                    "Roblox's sign-in service didn't answer for {} after {} tries. Wait a minute and press Play again. Starting many accounts at once makes this more likely.",
                    inst.account.display_name,
                    SIGNIN_RETRIES + 1
                ));
                Some("signin_limit")
            }
        } else if !wants_rejoin {
            None
        } else {
            let mut m = manager.lock().unwrap();
            let now = Instant::now();
            m.rejoins.retain(|(_, at)| now.duration_since(*at) < REJOIN_WINDOW);
            if rejoin_allowed(&m.rejoins, inst.account.id, now) {
                m.rejoins.push((inst.account.id, now));
                Some(if inst.disconnected { "disconnect" } else { "crash" })
            } else {
                Some("limit")
            }
        };
        let retrying_signin = rejoin == Some("signin");
        let rejoining = matches!(rejoin, Some("crash" | "disconnect")) || retrying_signin;
        let restarting = (inst.restart && !inst.disconnected) || rejoining;
        let _ = app.emit(
            "instances://exit",
            ExitEvent {
                id,
                game: inst.game.clone(),
                account_name: inst.account.display_name.clone(),
                report,
                log: log.iter().rev().take(12).rev().cloned().collect(),
                restarting,
                rejoin,
            },
        );
        let _ = app.emit("instances://changed", ());
        if restarting {
            // Let go of this game's slot and account before starting it again. The finished game's
            // record is still in scope here, and while it held them the new launch was refused
            // as "already in a game".
            drop(inst._claim);
            let uri = if retrying_signin {
                // Wait longer each time, so several accounts retrying don't hit the sign-in together again.
                tokio::time::sleep(std::time::Duration::from_secs(4 * signin_attempt as u64)).await;
                inst.uri
            } else if rejoining {
                // Give a crashed client a moment to let go of its files and its slot.
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                rejoin_uri(&inst.uri, inst.game.place_id)
            } else {
                inst.uri
            };
            if let Err(e) = launch_boxed(app.clone(), manager, inst.account, inst.game, uri, inst.env, inst.auto_rejoin).await {
                let _ = app.emit("instances://error", e);
            }
        }
    });
    Ok(id)
}

/// With a memory limit set, the kernel may kill the game outright, leaving no message in the log.
/// Say so, rather than showing a bare "stopped by a signal".
fn explain_kill(mut report: ExitReport, env: &[(&'static str, String)]) -> ExitReport {
    let limit = env.iter().find(|(k, _)| *k == "MOCKTAIL_MEMORY_LIMIT_MB").map(|(_, v)| v.as_str()).filter(|v| *v != "0");
    let killed = report.outcome == "crashed" && matches!(report.code, None | Some(137));
    if let (Some(mb), true) = (limit, killed) {
        if !report.detail.as_deref().is_some_and(|d| d.contains("memory safety limit")) {
            report.detail = Some(format!(
                "The game was killed, most likely for going over its {mb} MB memory safety limit. Raise the limit or turn it off in Settings → Performance."
            ));
        }
    }
    report
}

type LaunchFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Result<u32>> + Send>>;

/// `launch` restarts games by calling itself; boxing gives that recursion a nameable type.
fn launch_boxed(
    app: AppHandle,
    manager: SharedManager,
    account: SavedAccount,
    game: Game,
    uri: String,
    env: Vec<(&'static str, String)>,
    auto_rejoin: bool,
) -> LaunchFuture {
    Box::pin(launch(app, manager, account, game, uri, env, auto_rejoin))
}

/// Asks an instance's whole process tree to stop. With `restart`, it is started again once it has closed.
pub fn stop(manager: &SharedManager, id: u32, restart: bool) -> Result<()> {
    let pid = {
        let mut m = manager.lock().unwrap();
        let inst = m
            .instances
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or_else(|| AppError::Other("That game has already closed.".into()))?;
        inst.restart = restart;
        inst.stopping = true;
        inst.pid
    };
    if pid == 0 {
        return Err(AppError::Other("Vodka lost track of that game's process.".into()));
    }
    let pids: Vec<String> = proc::tree(&proc::snapshot(), pid).iter().map(|p| p.pid.to_string()).collect();
    if pids.is_empty() {
        return Ok(());
    }
    let status = std::process::Command::new("kill")
        .arg("-TERM")
        .args(&pids)
        .stderr(Stdio::null())
        .status()
        .map_err(|e| AppError::Other(format!("Could not stop the game: {e}")))?;
    // Some processes in the tree may already be gone; that is fine.
    let _ = status;
    Ok(())
}

/// Every running game with fresh CPU and memory figures.
pub fn list(manager: &SharedManager) -> Vec<InstanceView> {
    let all = proc::snapshot();
    let now = Instant::now();
    let mut m = manager.lock().unwrap();
    m.instances
        .iter_mut()
        .map(|inst| {
            let tree = proc::tree(&all, inst.pid);
            let ticks: u64 = tree.iter().map(|p| p.ticks).sum();
            if let Some((at, before)) = inst.sample {
                let seconds = now.duration_since(at).as_secs_f64();
                if seconds >= 0.5 {
                    inst.cpu_percent = proc::cpu_percent(before, ticks, seconds);
                    inst.sample = Some((now, ticks));
                }
            } else {
                inst.sample = Some((now, ticks));
            }
            InstanceView {
                id: inst.id,
                slot: inst.slot,
                account_id: inst.account.id,
                account_name: inst.account.display_name.clone(),
                game: inst.game.clone(),
                phase: if inst.playing { "playing" } else { "starting" },
                started_at_ms: inst.started_at_ms,
                cpu_percent: inst.cpu_percent,
                memory_bytes: tree.iter().map(|p| p.memory_bytes).sum(),
                fps: inst.fps,
                log: inst.log.iter().rev().take(40).rev().cloned().collect(),
            }
        })
        .collect()
}

/// Records one presence answer for a running game. Returns true when the account has been out of
/// the game long enough that Vodka should bring it back (and auto-rejoin is on for it).
pub fn note_presence(manager: &SharedManager, id: u32, in_game: bool) -> bool {
    let mut m = manager.lock().unwrap();
    let Some(inst) = m.instances.iter_mut().find(|i| i.id == id) else { return false };
    let settled = inst.playing && now_ms().saturating_sub(inst.started_at_ms) >= SETTLE_MS;
    if in_game || !settled || !inst.auto_rejoin || inst.stopping {
        inst.absent_checks = 0;
        return false;
    }
    inst.absent_checks = inst.absent_checks.saturating_add(1);
    inst.absent_checks >= ABSENT_CHECKS_FOR_REJOIN
}

/// Restarts a game into a different link (used to move accounts into one server together).
pub fn restart_into(manager: &SharedManager, id: u32, uri: String) -> Result<()> {
    if let Some(inst) = manager.lock().unwrap().instances.iter_mut().find(|i| i.id == id) {
        inst.uri = uri;
    }
    stop(manager, id, true)
}

/// Stops a game whose account was disconnected, so the exit handler starts it again.
pub fn stop_for_rejoin(manager: &SharedManager, id: u32) -> Result<()> {
    if let Some(inst) = manager.lock().unwrap().instances.iter_mut().find(|i| i.id == id) {
        inst.disconnected = true;
    }
    stop(manager, id, true)
}

/// (instance id, account id, universe id) of every running game, for checking where each account really is.
pub fn running_games(manager: &SharedManager) -> Vec<(u32, u64, u64)> {
    manager.lock().unwrap().instances.iter().map(|i| (i.id, i.account.id, i.game.universe_id)).collect()
}

/// Records that an instance is now in a different game (the player switched inside Roblox).
pub fn set_game(manager: &SharedManager, id: u32, game: Game) {
    if let Some(inst) = manager.lock().unwrap().instances.iter_mut().find(|i| i.id == id) {
        inst.game = game;
    }
}

pub fn running_account_ids(manager: &SharedManager) -> Vec<u64> {
    manager.lock().unwrap().instances.iter().map(|i| i.account.id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_game_takes_the_default_slot() {
        assert_eq!(pick_slot(&[], 1, "A").unwrap(), 0);
    }

    #[test]
    fn second_account_takes_the_other_slot() {
        assert_eq!(pick_slot(&[(0, 1)], 2, "B").unwrap(), 1);
        assert_eq!(pick_slot(&[(1, 1)], 2, "B").unwrap(), 0);
    }

    #[test]
    fn one_account_cannot_be_in_two_games() {
        let err = pick_slot(&[(0, 1)], 1, "Ari").unwrap_err();
        assert!(err.to_string().contains("Ari is already in a game"));
    }

    #[test]
    fn further_games_take_the_next_free_slot() {
        assert_eq!(pick_slot(&[(0, 1), (1, 2)], 3, "C").unwrap(), 2);
        assert_eq!(pick_slot(&[(0, 1), (2, 3)], 4, "D").unwrap(), 1);
    }

    #[test]
    fn a_kill_with_a_memory_limit_set_points_at_the_limit() {
        let killed = ExitReport { code: None, outcome: "crashed", detail: Some("Mocktail was stopped by a signal.".into()) };
        let with_limit = explain_kill(killed.clone(), &[("MOCKTAIL_MEMORY_LIMIT_MB", "4096".into())]);
        assert!(with_limit.detail.unwrap().contains("4096 MB memory safety limit"));
        assert_eq!(explain_kill(killed.clone(), &[("MOCKTAIL_MEMORY_LIMIT_MB", "0".into())]), killed);
        assert_eq!(explain_kill(killed.clone(), &[]), killed);
        let clean = ExitReport { code: Some(0), outcome: "closed", detail: None };
        assert_eq!(explain_kill(clean.clone(), &[("MOCKTAIL_MEMORY_LIMIT_MB", "4096".into())]), clean);
    }

    #[test]
    fn sign_in_failures_are_counted_per_account() {
        let now = Instant::now();
        let history = vec![(1, now), (1, now), (2, now)];
        assert_eq!(signin_failures_recently(&history, 1, now), 2);
        assert_eq!(signin_failures_recently(&history, 2, now), 1);
        assert_eq!(signin_failures_recently(&history, 3, now), 0);
        // The first three failures are retried, the fourth is reported.
        assert!(3 <= SIGNIN_RETRIES && 4 > SIGNIN_RETRIES);
    }

    #[test]
    fn rejoins_are_capped_per_account() {
        let now = Instant::now();
        let recent = |n: usize, id: u64| vec![(id, now); n];
        assert!(rejoin_allowed(&[], 1, now));
        assert!(rejoin_allowed(&recent(4, 1), 1, now));
        assert!(!rejoin_allowed(&recent(5, 1), 1, now));
        // Another account's rejoins don't count against this one.
        assert!(rejoin_allowed(&recent(5, 2), 1, now));
    }

    #[test]
    fn rejoining_drops_a_specific_public_server_but_keeps_a_private_one() {
        assert_eq!(rejoin_uri("roblox://experiences/start?placeId=5&gameInstanceId=abc", 5), "roblox://experiences/start?placeId=5");
        assert_eq!(rejoin_uri("roblox://experiences/start?placeId=5", 5), "roblox://experiences/start?placeId=5");
        let private = "roblox://experiences/start?placeId=5&accessCode=zz";
        assert_eq!(rejoin_uri(private, 5), private);
    }

    #[test]
    fn there_is_an_upper_limit() {
        let full: Vec<(usize, u64)> = (0..slots::MAX_INSTANCES).map(|s| (s, s as u64 + 1)).collect();
        let err = pick_slot(&full, 99, "Z").unwrap_err();
        assert!(err.to_string().contains("at most 10 games"));
    }
}
