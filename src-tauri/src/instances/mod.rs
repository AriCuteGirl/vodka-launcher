//! Running games. Each one is a Mocktail process with its own account and profile.
//!
//! Mocktail allows one process per executable path, and its Flatpak ships the
//! same binary at two paths. Vodka uses one path per slot, so two games can run
//! at once. Mocktail's authors do not support this; see the warning in the UI.

pub mod proc;
pub mod profiles;

use crate::accounts;
use crate::error::{AppError, Result};
use crate::mocktail::detect::APP_ID;
use crate::mocktail::launch::{ExitReport, classify_exit, is_running_marker, launch_uri, redact};
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

/// The path Mocktail is started from, per slot. `None` is the Flatpak's default command.
pub const SLOT_COMMANDS: [Option<&str>; 2] = [None, Some("/app/lib/mocktail/mocktail")];
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
    log: VecDeque<String>,
    profile: ProfilePaths,
    sample: Option<(Instant, u64)>,
    cpu_percent: f64,
}

#[derive(Default)]
pub struct Manager {
    next_id: u32,
    instances: Vec<Instance>,
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
}

/// Which slot a new game may take, or why it can't start.
pub fn pick_slot(used: &[(usize, u64)], account_id: u64, account_name: &str) -> Result<usize> {
    if used.iter().any(|(_, id)| *id == account_id) {
        return Err(AppError::Launch(format!(
            "{account_name} is already in a game. Close it, or switch to another account."
        )));
    }
    (0..SLOT_COMMANDS.len())
        .find(|slot| !used.iter().any(|(s, _)| s == slot))
        .ok_or_else(|| AppError::Launch(format!("Vodka can run {} games at once, and all are in use.", SLOT_COMMANDS.len())))
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn flatpak_data() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or_else(|| AppError::Other("HOME is not set.".into()))?;
    Ok(profiles::flatpak_data(&PathBuf::from(home)))
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
                if !inst.playing && is_running_marker(&line) {
                    inst.playing = true;
                    inst.started_at_ms = now_ms();
                    became_playing = true;
                }
            }
            if became_playing {
                let _ = app.emit("instances://changed", ());
            }
        }
    });
}

/// Starts `game` as `account` in a free slot. Returns the new instance id.
pub async fn launch(app: AppHandle, manager: SharedManager, account: SavedAccount, game: Game) -> Result<u32> {
    let used: Vec<(usize, u64)> = manager.lock().unwrap().instances.iter().map(|i| (i.slot, i.account.id)).collect();
    pick_slot(&used, account.id, &account.display_name)?;

    let account_id = account.id;
    let (cookie, profile) = tauri::async_runtime::spawn_blocking(move || -> Result<(String, ProfilePaths)> {
        let cookie = accounts::load_session(account_id)?.ok_or(AppError::NotSignedIn)?;
        let data = flatpak_data()?;
        let profile = profiles::profile_paths(&data, account_id);
        profiles::prepare(&data.join("mocktail"), &profile)
            .map_err(|e| AppError::Launch(format!("could not prepare this account's profile: {e}")))?;
        Ok((cookie, profile))
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))??;

    // Re-check under the lock: another launch may have taken the slot meanwhile.
    let mut m = manager.lock().unwrap();
    let used: Vec<(usize, u64)> = m.instances.iter().map(|i| (i.slot, i.account.id)).collect();
    let slot = pick_slot(&used, account.id, &account.display_name)?;

    let mut cmd = Command::new("flatpak");
    cmd.arg("run");
    if let Some(command) = SLOT_COMMANDS[slot] {
        cmd.arg(format!("--command={command}"));
    }
    cmd.args([APP_ID, &launch_uri(game.place_id)])
        .env("MOCKTAIL_DATA_ROOT", &profile.data)
        .env("MOCKTAIL_CACHE_ROOT", &profile.cache)
        .env("MOCKTAIL_STATE_ROOT", &profile.state)
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
        log: VecDeque::new(),
        profile,
        sample: None,
        cpu_percent: 0.0,
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
            Ok(st) if inst.restart => ExitReport { code: st.code(), outcome: "closed", detail: None },
            Ok(st) => classify_exit(st.code(), &log),
            Err(e) => ExitReport { code: None, outcome: "crashed", detail: Some(e.to_string()) },
        };
        let _ = app.emit(
            "instances://exit",
            ExitEvent {
                id,
                game: inst.game.clone(),
                account_name: inst.account.display_name.clone(),
                report,
                log: log.iter().rev().take(12).rev().cloned().collect(),
                restarting: inst.restart,
            },
        );
        let _ = app.emit("instances://changed", ());
        if inst.restart {
            if let Err(e) = launch_boxed(app.clone(), manager, inst.account, inst.game).await {
                let _ = app.emit("instances://error", e);
            }
        }
    });
    Ok(id)
}

type LaunchFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Result<u32>> + Send>>;

/// `launch` restarts games by calling itself; boxing gives that recursion a nameable type.
fn launch_boxed(app: AppHandle, manager: SharedManager, account: SavedAccount, game: Game) -> LaunchFuture {
    Box::pin(launch(app, manager, account, game))
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
                log: inst.log.iter().rev().take(40).rev().cloned().collect(),
            }
        })
        .collect()
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
        assert_eq!(SLOT_COMMANDS[0], None);
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
    fn no_third_game() {
        let err = pick_slot(&[(0, 1), (1, 2)], 3, "C").unwrap_err();
        assert!(err.to_string().contains("2 games at once"));
    }

    #[test]
    fn slots_use_different_executable_paths() {
        assert_ne!(SLOT_COMMANDS[0], SLOT_COMMANDS[1]);
    }
}
