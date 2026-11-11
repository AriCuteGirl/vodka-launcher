mod accounts;
mod error;
mod mocktail;
mod roblox_api;
mod settings;

use error::{AppError, Result};
use mocktail::detect::MocktailStatus;
use mocktail::launch::SharedSession;
use roblox_api::{Game, User};
use settings::Settings;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    pub api: roblox_api::Client,
    pub session: SharedSession,
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| AppError::Other(e.to_string()))?
}

async fn session_cookie() -> Result<String> {
    blocking(accounts::load_session).await?.ok_or(AppError::NotSignedIn)
}

fn config_dir(app: &AppHandle) -> Result<PathBuf> {
    app.path().app_config_dir().map_err(|e| AppError::Other(e.to_string()))
}

#[tauri::command]
async fn mocktail_status() -> Result<MocktailStatus> {
    blocking(|| Ok(mocktail::detect::detect())).await
}

#[tauri::command]
async fn launch_game(app: AppHandle, state: State<'_, AppState>, place_id: u64) -> Result<()> {
    let status = blocking(|| Ok(mocktail::detect::detect())).await?;
    if !status.flatpak_available {
        return Err(AppError::FlatpakMissing);
    }
    if !status.installed {
        return Err(AppError::MocktailMissing);
    }
    let share = settings::load(&config_dir(&app)?).share_session_with_mocktail;
    // A running Mocktail keeps its own session; the join link is simply forwarded to it.
    let cookie = if share && !status.running { blocking(accounts::load_session).await? } else { None };
    mocktail::launch::launch(app, state.session.clone(), place_id, cookie)
}

#[tauri::command]
fn session_log(state: State<'_, AppState>) -> Vec<String> {
    state.session.lock().unwrap().log.iter().cloned().collect()
}

#[tauri::command]
fn begin_login(app: AppHandle) -> Result<()> {
    accounts::open_login(&app)
}

#[tauri::command]
async fn current_account(state: State<'_, AppState>) -> Result<Option<User>> {
    let Some(cookie) = blocking(accounts::load_session).await? else { return Ok(None) };
    state.api.current_user(&cookie).await.map(Some)
}

#[tauri::command]
async fn sign_out(state: State<'_, AppState>) -> Result<()> {
    if let Some(cookie) = blocking(accounts::load_session).await? {
        // Best effort: the local copy is removed even if Roblox can't be reached.
        let _ = state.api.logout(&cookie).await;
    }
    state.api.clear_cache();
    blocking(accounts::clear_session).await
}

#[tauri::command]
async fn continue_playing(state: State<'_, AppState>) -> Result<Vec<Game>> {
    state.api.continue_playing(&session_cookie().await?).await
}

#[tauri::command]
async fn favorites(state: State<'_, AppState>, user_id: u64) -> Result<Vec<Game>> {
    state.api.favorites(&session_cookie().await?, user_id).await
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Result<Settings> {
    Ok(settings::load(&config_dir(&app)?))
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings) -> Result<()> {
    settings::save(&config_dir(&app)?, &settings)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState { api: roblox_api::Client::new(), session: SharedSession::default() })
        .invoke_handler(tauri::generate_handler![
            mocktail_status,
            launch_game,
            session_log,
            begin_login,
            current_account,
            sign_out,
            continue_playing,
            favorites,
            get_settings,
            save_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vodka Launcher");
}
