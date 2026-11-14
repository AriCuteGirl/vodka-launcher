//! Running games on Windows. Here Vodka starts the official Roblox client (there is no Mocktail),
//! signed in as the chosen account with a one-time ticket.
//!
//! What's different from Linux: one game at a time, no per-game CPU, memory or FPS, and no
//! restart or gather. Whether the game is running is learned from Roblox's presence for the
//! account, because the program Vodka starts is only Roblox's own launcher, which hands off and exits.

use crate::accounts;
use crate::error::{AppError, Result};
use crate::roblox_api::Game;
use crate::settings::SavedAccount;
use crate::windows_player;
use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager as _};

/// Don't flash a console window for helper commands.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// A launch that Roblox never reports as "in a game" is forgotten after this long.
const GIVE_UP_MS: u64 = 4 * 60_000;

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
    _env: Vec<(&'static str, String)>,
    _auto_rejoin: bool,
) -> Result<u32> {
    if !manager.lock().unwrap().instances.is_empty() {
        return Err(AppError::Launch("the Windows version runs one game at a time for now. Close the running game first.".into()));
    }
    let to = windows_player::destination(&uri).ok_or_else(|| AppError::Launch("Vodka couldn't work out where that link goes.".into()))?;
    let account_id = account.id;
    let cookie = tauri::async_runtime::spawn_blocking(move || accounts::load_session(account_id))
        .await
        .map_err(|e| AppError::Other(e.to_string()))??
        .ok_or(AppError::NotSignedIn)?;
    // A one-time ticket signs the client in as this account; the session cookie itself never leaves Vodka.
    let ticket = app.state::<crate::AppState>().api.auth_ticket(&cookie).await?;
    let now = now_ms();
    let link = windows_player::player_link(&ticket, &to, now, now % 1_000_000_000 + 100_000_000_000)
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
    m.instances.push(Instance { id, account, game, started_at_ms: now, playing: false, absent_checks: 0 });
    drop(m);
    let _ = app.emit("instances://changed", ());
    Ok(id)
}

/// Closes the Roblox client. Restarting isn't available on Windows yet.
pub fn stop(manager: &SharedManager, id: u32, restart: bool) -> Result<()> {
    if restart {
        return Err(AppError::Other("Restart isn't available in the Windows version yet. Close the game and press Play again.".into()));
    }
    manager.lock().unwrap().instances.retain(|i| i.id != id);
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/IM", "RobloxPlayerBeta.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
    Ok(())
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
        } else if inst.playing {
            inst.absent_checks = inst.absent_checks.saturating_add(1);
            ended = inst.absent_checks >= 2;
        } else {
            ended = now.saturating_sub(inst.started_at_ms) > GIVE_UP_MS;
        }
    }
    if ended {
        m.instances.retain(|i| i.id != id);
    }
    false
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
            memory_bytes: 0,
            fps: None,
            log: Vec::new(),
        })
        .collect()
}
