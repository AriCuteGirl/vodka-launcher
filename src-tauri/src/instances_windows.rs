//! Running games on Windows. Here Vodka starts the official Roblox client (there is no Mocktail),
//! signed in as the chosen account with a one-time ticket.
//!
//! What's different from Linux: several games at once only after the user accepts the multi-instance
//! warning (see `singleton`), no per-game CPU or FPS, and no restart or gather. The program Vodka starts is only
//! Roblox's own launcher, which hands off and exits, so each game is matched to the `RobloxPlayerBeta.exe` process
//! that appears after its launch (see `winclient`). That process is how Vodka knows the game ended, closes it, and
//! reads its memory use; Vodka never opens or changes the process itself (only `tasklist` and `taskkill`).

use crate::accounts;
use crate::error::{AppError, Result};
use crate::roblox_api::Game;
use crate::settings::SavedAccount;
use crate::windows_player;
use crate::winclient;
use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager as _};

/// Don't flash a console window for helper commands.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// A launch that Roblox never reports as "in a game" is forgotten after this long.
const GIVE_UP_MS: u64 = 4 * 60_000;

/// Roblox lets one client run at a time by checking a named mutex, `ROBLOX_singletonEvent`. If Vodka holds that
/// mutex before the first client starts, further clients can start too. This is the same thing open-source
/// multi-instance tools do. It only creates a named object in Vodka's own process: it never opens, reads or
/// changes a Roblox process. Unofficial: Roblox may block it in any update.
mod singleton {
    use std::sync::Mutex;

    const ERROR_ALREADY_EXISTS: u32 = 183;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateMutexW(attributes: *const core::ffi::c_void, initial_owner: i32, name: *const u16) -> isize;
        fn GetLastError() -> u32;
        fn CloseHandle(handle: isize) -> i32;
    }

    static HELD: Mutex<Option<isize>> = Mutex::new(None);

    pub fn held() -> bool {
        HELD.lock().unwrap().is_some()
    }

    /// Takes the lock. Fails if a Roblox client already holds it, because then it's too late for this session.
    pub fn take(roblox_running: impl FnOnce() -> bool) -> bool {
        let mut held = HELD.lock().unwrap();
        if held.is_some() {
            return true;
        }
        let name: Vec<u16> = "ROBLOX_singletonEvent\0".encode_utf16().collect();
        // SAFETY: a null security descriptor and a NUL-terminated UTF-16 name, as CreateMutexW expects.
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle == 0 {
            return false;
        }
        // SAFETY: plain call, read right after CreateMutexW on the same thread.
        let existed = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        if existed && roblox_running() {
            // SAFETY: the handle was just returned by CreateMutexW and isn't stored anywhere.
            unsafe { CloseHandle(handle) };
            return false;
        }
        *held = Some(handle);
        true
    }

    pub fn release() {
        if let Some(handle) = HELD.lock().unwrap().take() {
            // SAFETY: the handle came from CreateMutexW and is closed once, here.
            unsafe { CloseHandle(handle) };
        }
    }
}

/// Every running Roblox client, with its memory use.
fn roblox_processes() -> Vec<winclient::Process> {
    std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq RobloxPlayerBeta.exe", "/NH", "/FO", "CSV"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| winclient::parse_tasklist(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default()
}

fn kill_pid(pid: u32) -> bool {
    std::process::Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .is_ok_and(|s| s.success())
}

/// Whether any Roblox client is running, whoever started it.
fn roblox_running() -> bool {
    std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq RobloxPlayerBeta.exe", "/NH", "/FO", "CSV"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("RobloxPlayerBeta.exe"))
        .unwrap_or(false)
}

/// Keeps the multi-instance lock in line with the user's choice. Called when the setting changes.
/// Turning it off keeps the lock while games are running, so they aren't disturbed.
pub fn set_multi_instance(manager: &SharedManager, on: bool) {
    if on {
        singleton::take(roblox_running);
    } else if manager.lock().unwrap().instances.is_empty() {
        singleton::release();
    }
}

pub mod profiles {
    //! Linux keeps a Mocktail profile per account. Windows has none; these keep the shared code compiling.
    use std::path::{Path, PathBuf};

    pub struct ProfilePaths {
        pub root: PathBuf,
    }
    pub fn flatpak_data(home: &Path) -> PathBuf {
        home.to_path_buf()
    }
    pub fn profile_paths(base: &Path, account_id: u64) -> ProfilePaths {
        ProfilePaths { root: base.join("vodka-no-profile").join(account_id.to_string()) }
    }
    pub fn client_version(_: &ProfilePaths) -> Option<String> {
        None
    }
    pub fn remove_tree(_: &Path) -> std::io::Result<()> {
        Ok(())
    }
}

struct Instance {
    id: u32,
    account: SavedAccount,
    game: Game,
    started_at_ms: u64,
    playing: bool,
    absent_checks: u8,
    /// Sent in the launch link; finds this game's Roblox process when only this one should close.
    tracker: u64,
    /// This game's Roblox process, once it appeared.
    pid: Option<u32>,
    /// Roblox processes already running at launch; never this game's.
    before: Vec<u32>,
    memory_bytes: u64,
    /// Close the game when its memory goes over this; `None` is no limit.
    memory_limit_mb: Option<u32>,
}

#[derive(Default)]
pub struct Manager {
    next_id: u32,
    instances: Vec<Instance>,
    /// Names of games given up on since the last `take_given_up`, so the user can be told.
    given_up: Vec<String>,
    /// Games Vodka closed for going over their memory limit, to tell the user.
    over_memory: Vec<String>,
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
    pub phase: &'static str,
    pub started_at_ms: u64,
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub fps: Option<f64>,
    pub log: Vec<String>,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Starts `game` in the official Roblox client as `account`.
pub async fn launch(
    app: AppHandle,
    manager: SharedManager,
    account: SavedAccount,
    game: Game,
    uri: String,
    env: Vec<(&'static str, String)>,
    _auto_rejoin: bool,
) -> Result<u32> {
    let multi = app
        .path()
        .app_config_dir()
        .map(|dir| crate::settings::load(&dir).multi_instance_accepted)
        .unwrap_or(false);
    if multi {
        // Taken before the first client starts, which is the only time it can work.
        singleton::take(roblox_running);
    }
    {
        let m = manager.lock().unwrap();
        if m.instances.iter().any(|i| i.account.id == account.id) {
            return Err(AppError::Other(format!("{} is already in a game. Close it first.", account.display_name)));
        }
        if !m.instances.is_empty() {
            if !multi {
                return Err(AppError::Other("Another game is running. Close it first, or allow several games at once in Settings.".into()));
            }
            if !singleton::held() {
                return Err(AppError::Other(
                    "Roblox was already open before Vodka could allow several games. Close every Roblox window, then start your games again from Vodka."
                        .into(),
                ));
            }
        }
    }
    let setting = |name: &str| env.iter().find(|(k, _)| *k == name).map(|(_, v)| v.clone());
    // FastFlags, FPS and the graphics API go in Roblox's own flag file, read when the client starts.
    let mut flags: std::collections::BTreeMap<String, String> =
        setting("MOCKTAIL_CLIENT_SETTINGS_OVERRIDES_JSON").and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default();
    flags.extend(winclient::option_flags(setting("MOCKTAIL_FRAME_RATE_LIMIT").as_deref(), setting("VODKA_WINDOWS_RENDERER").as_deref()));
    if let Ok(dir) = app.path().app_config_dir() {
        let flags = flags.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || write_flags(&dir, &flags)).await;
    }
    let memory_limit_mb = setting("MOCKTAIL_MEMORY_LIMIT_MB").and_then(|v| v.parse::<u32>().ok()).filter(|mb| *mb > 0);
    let before: Vec<u32> = tauri::async_runtime::spawn_blocking(roblox_processes).await.unwrap_or_default().iter().map(|p| p.pid).collect();
    let to = windows_player::destination(&uri).ok_or_else(|| AppError::Launch("Vodka couldn't work out where that link goes.".into()))?;
    let account_id = account.id;
    let cookie = tauri::async_runtime::spawn_blocking(move || accounts::load_session(account_id))
        .await
        .map_err(|e| AppError::Other(e.to_string()))??
        .ok_or(AppError::NotSignedIn)?;
    // A one-time ticket signs the client in as this account; the session cookie itself never leaves Vodka.
    let ticket = app.state::<crate::AppState>().api.auth_ticket(&cookie).await?;
    let now = now_ms();
    let tracker = now % 1_000_000_000 + 100_000_000_000;
    let link = windows_player::player_link(&ticket, &to, now, tracker)
        .ok_or_else(|| AppError::Launch("Roblox returned a sign-in ticket Vodka can't use.".into()))?;
    // Handed to Windows' protocol handler directly, with no shell in between to reinterpret it.
    std::process::Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", &link])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| AppError::Launch(format!("Windows couldn't start Roblox: {e}")))?;

    let mut m = manager.lock().unwrap();
    m.next_id += 1;
    let id = m.next_id;
    m.instances.push(Instance {
        id,
        account,
        game,
        started_at_ms: now,
        playing: false,
        absent_checks: 0,
        tracker,
        pid: None,
        before,
        memory_bytes: 0,
        memory_limit_mb,
    });
    drop(m);
    let _ = app.emit("instances://changed", ());
    Ok(id)
}

/// Closes the Roblox client. Restarting isn't available on Windows yet.
pub fn stop(manager: &SharedManager, id: u32, restart: bool) -> Result<()> {
    if restart {
        return Err(AppError::Other("Restart isn't available in the Windows version yet. Close the game and press Play again.".into()));
    }
    let (tracker, pid, others) = {
        let mut m = manager.lock().unwrap();
        let found = m.instances.iter().find(|i| i.id == id).map(|i| (i.tracker, i.pid));
        m.instances.retain(|i| i.id != id);
        (found.map(|f| f.0), found.and_then(|f| f.1), m.instances.len())
    };
    // The process Vodka matched to this game: close exactly that one.
    if let Some(pid) = pid
        && kill_pid(pid)
    {
        return Ok(());
    }
    // Close only the client started with this game's link.
    if let Some(tracker) = tracker {
        let script = format!(
            "Get-CimInstance Win32_Process -Filter \"Name='RobloxPlayerBeta.exe'\" | \
             Where-Object {{ $_.CommandLine -like '*browsertrackerid:{tracker}+*' }} | \
             ForEach-Object {{ Stop-Process -Id $_.ProcessId -Force; 'closed' }}"
        );
        let closed = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("closed"))
            .unwrap_or(false);
        if closed {
            return Ok(());
        }
    }
    if others == 0 {
        // The only game Vodka knows of, so closing every client can't hit another one of Vodka's.
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "RobloxPlayerBeta.exe"])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
        return Ok(());
    }
    Err(AppError::Other("Vodka couldn't tell which Roblox window is this game's, so it left them all open. Close that window yourself.".into()))
}

pub fn restart_into(_: &SharedManager, _: u32, _: String) -> Result<()> {
    Err(AppError::Other("Moving a running game to another server isn't available in the Windows version yet.".into()))
}

pub fn stop_for_rejoin(_: &SharedManager, _: u32) -> Result<()> {
    Ok(())
}

/// Presence is how the Windows version follows a game: first "in a game" means it started,
/// two "not in a game" answers after that mean it ended. Never asks for a rejoin.
pub fn note_presence(manager: &SharedManager, id: u32, in_game: bool) -> bool {
    let mut m = manager.lock().unwrap();
    let now = now_ms();
    let mut ended = false;
    if let Some(inst) = m.instances.iter_mut().find(|i| i.id == id) {
        if in_game {
            if !inst.playing {
                inst.playing = true;
                inst.started_at_ms = now;
            }
            inst.absent_checks = 0;
        } else if inst.pid.is_some() {
            // Its process is still there (see `sync`), so it's still running whatever presence says.
        } else if inst.playing {
            inst.absent_checks = inst.absent_checks.saturating_add(1);
            ended = inst.absent_checks >= 2;
        } else {
            ended = now.saturating_sub(inst.started_at_ms) > GIVE_UP_MS;
        }
    }
    if ended {
        if let Some(inst) = m.instances.iter().find(|i| i.id == id && !i.playing) {
            let name = inst.game.name.clone();
            m.given_up.push(name);
        }
        m.instances.retain(|i| i.id != id);
    }
    false
}

/// Matches games to Roblox processes, drops games whose process is gone, and closes any game over its memory limit.
fn sync(manager: &SharedManager) {
    let running = roblox_processes();
    let mut m = manager.lock().unwrap();
    let tracked: Vec<winclient::Tracked> = m.instances.iter().map(|i| winclient::Tracked { pid: i.pid, before: i.before.clone() }).collect();
    let outcomes = winclient::match_processes(&tracked, &running);
    let mut ended = Vec::new();
    let mut over = Vec::new();
    for (inst, outcome) in m.instances.iter_mut().zip(outcomes) {
        match outcome {
            winclient::Outcome::Ended => ended.push(inst.id),
            winclient::Outcome::Running(pid) => {
                inst.pid = pid;
                if let Some(pid) = pid {
                    inst.playing = true;
                    inst.memory_bytes = running.iter().find(|p| p.pid == pid).map(|p| p.memory_bytes).unwrap_or(0);
                    if let Some(limit) = inst.memory_limit_mb
                        && inst.memory_bytes > u64::from(limit) * 1024 * 1024
                        && kill_pid(pid)
                    {
                        over.push((inst.id, format!("{} ({} MB limit)", inst.game.name, limit)));
                    }
                }
            }
        }
    }
    m.instances.retain(|i| !ended.contains(&i.id) && !over.iter().any(|(id, _)| *id == i.id));
    m.over_memory.extend(over.into_iter().map(|(_, name)| name));
}

/// Games Vodka closed for going over their memory limit since the last call.
pub fn take_over_memory(manager: &SharedManager) -> Vec<String> {
    std::mem::take(&mut manager.lock().unwrap().over_memory)
}

/// Writes Vodka's FastFlags into every installed Roblox client's `ClientSettings\ClientAppSettings.json`, keeping
/// flags someone else put there. Roblox reads the file when a client starts.
fn write_flags(config_dir: &std::path::Path, flags: &std::collections::BTreeMap<String, String>) {
    let owned_file = config_dir.join("windows-fflags-owned.json");
    let previous: Vec<String> = std::fs::read_to_string(&owned_file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let Some(versions) = std::env::var_os("LOCALAPPDATA").map(|d| std::path::PathBuf::from(d).join("Roblox").join("Versions")) else { return };
    let Ok(entries) = std::fs::read_dir(&versions) else { return };
    let mut owned = previous.clone();
    for entry in entries.flatten() {
        if !entry.path().join("RobloxPlayerBeta.exe").is_file() {
            continue;
        }
        let dir = entry.path().join("ClientSettings");
        let file = dir.join("ClientAppSettings.json");
        let existing = std::fs::read_to_string(&file).unwrap_or_default();
        let (text, now_owned) = winclient::merge_flag_file(&existing, &previous, flags);
        if std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&file, text)).is_ok() {
            owned = now_owned;
        }
    }
    let _ = std::fs::create_dir_all(config_dir);
    let _ = std::fs::write(&owned_file, serde_json::to_string(&owned).unwrap_or_else(|_| "[]".into()));
}

/// Games that never started since the last call. Roblox's own error window (if any) says why.
pub fn take_given_up(manager: &SharedManager) -> Vec<String> {
    std::mem::take(&mut manager.lock().unwrap().given_up)
}

pub fn running_games(manager: &SharedManager) -> Vec<(u32, u64, u64)> {
    manager.lock().unwrap().instances.iter().map(|i| (i.id, i.account.id, i.game.universe_id)).collect()
}

pub fn set_game(manager: &SharedManager, id: u32, game: Game) {
    if let Some(inst) = manager.lock().unwrap().instances.iter_mut().find(|i| i.id == id) {
        inst.game = game;
    }
}

pub fn running_account_ids(manager: &SharedManager) -> Vec<u64> {
    manager.lock().unwrap().instances.iter().map(|i| i.account.id).collect()
}

pub fn list(manager: &SharedManager) -> Vec<InstanceView> {
    if !manager.lock().unwrap().instances.is_empty() {
        sync(manager);
    }
    manager
        .lock()
        .unwrap()
        .instances
        .iter()
        .map(|i| InstanceView {
            id: i.id,
            slot: 0,
            account_id: i.account.id,
            account_name: i.account.display_name.clone(),
            game: i.game.clone(),
            phase: if i.playing { "playing" } else { "starting" },
            started_at_ms: i.started_at_ms,
            cpu_percent: 0.0,
            memory_bytes: i.memory_bytes,
            fps: None,
            log: Vec::new(),
        })
        .collect()
}
