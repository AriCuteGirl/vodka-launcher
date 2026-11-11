mod accounts;
mod error;
mod instances;
mod mocktail;
mod roblox_api;
mod settings;

use error::{AppError, Result};
use instances::{InstanceView, SharedManager};
use mocktail::detect::MocktailStatus;
use roblox_api::{Game, Presence, User};
use serde::{Deserialize, Serialize};
use settings::{SavedAccount, Settings};
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    pub api: roblox_api::Client,
    pub instances: SharedManager,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountView {
    id: u64,
    name: String,
    display_name: String,
    avatar_url: Option<String>,
    active: bool,
    presence: Option<Presence>,
    in_vodka_game: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Flags {
    onboarded: Option<bool>,
    multi_instance_accepted: Option<bool>,
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| AppError::Other(e.to_string()))?
}

fn config_dir(app: &AppHandle) -> Result<PathBuf> {
    app.path().app_config_dir().map_err(|e| AppError::Other(e.to_string()))
}

fn active_account(app: &AppHandle) -> Result<Option<SavedAccount>> {
    Ok(settings::load(&config_dir(app)?).active().cloned())
}

async fn active_cookie(app: &AppHandle) -> Result<(SavedAccount, String)> {
    let account = active_account(app)?.ok_or(AppError::NotSignedIn)?;
    let id = account.id;
    let cookie = blocking(move || accounts::load_session(id)).await?.ok_or(AppError::NotSignedIn)?;
    Ok((account, cookie))
}

/// Phase 1 stored one session without an account id. Move it under its account.
async fn migrate_legacy_session(app: &AppHandle, api: &roblox_api::Client) -> Result<()> {
    let Some(cookie) = blocking(accounts::load_legacy_session).await? else { return Ok(()) };
    match api.current_user(&cookie).await {
        Ok(user) => {
            let id = user.id;
            blocking(move || accounts::store_session(id, &cookie)).await?;
            settings::update(&config_dir(app)?, |s| {
                s.upsert_active(SavedAccount { id: user.id, name: user.name, display_name: user.display_name })
            })?;
            blocking(accounts::clear_legacy_session).await
        }
        // A dead session is not worth keeping.
        Err(AppError::SessionExpired) => blocking(accounts::clear_legacy_session).await,
        // Offline or rate-limited: try again next start.
        Err(_) => Ok(()),
    }
}

#[tauri::command]
async fn mocktail_status() -> Result<MocktailStatus> {
    blocking(|| Ok(mocktail::detect::detect())).await
}

#[tauri::command]
fn begin_login(app: AppHandle) -> Result<()> {
    accounts::open_login(&app)
}

/// The active account, checked against Roblox. `None` when nobody is signed in.
#[tauri::command]
async fn current_account(app: AppHandle, state: State<'_, AppState>) -> Result<Option<User>> {
    migrate_legacy_session(&app, &state.api).await?;
    match active_cookie(&app).await {
        Ok((_, cookie)) => state.api.current_user(&cookie).await.map(Some),
        Err(AppError::NotSignedIn) => Ok(None),
        Err(e) => Err(e),
    }
}

#[tauri::command]
async fn list_accounts(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<AccountView>> {
    let saved = settings::load(&config_dir(&app)?);
    let ids: Vec<u64> = saved.accounts.iter().map(|a| a.id).collect();
    // Pictures and presence are nice to have; the list still shows without them.
    let avatars = state.api.headshots(&ids).await.unwrap_or_default();
    let presence = match active_cookie(&app).await {
        Ok((_, cookie)) => state.api.presences(&cookie, &ids).await.unwrap_or_default(),
        Err(_) => Default::default(),
    };
    let running = instances::running_account_ids(&state.instances);
    Ok(saved
        .accounts
        .iter()
        .map(|a| AccountView {
            id: a.id,
            name: a.name.clone(),
            display_name: a.display_name.clone(),
            avatar_url: avatars.get(&a.id).cloned(),
            active: saved.active_account == Some(a.id),
            presence: presence.get(&a.id).cloned(),
            in_vodka_game: running.contains(&a.id),
        })
        .collect())
}

#[tauri::command]
fn switch_account(app: AppHandle, state: State<'_, AppState>, account_id: u64) -> Result<()> {
    let dir = config_dir(&app)?;
    if !settings::load(&dir).accounts.iter().any(|a| a.id == account_id) {
        return Err(AppError::Other("That account isn't in Vodka any more.".into()));
    }
    settings::update(&dir, |s| s.active_account = Some(account_id))?;
    state.api.clear_cache();
    Ok(())
}

/// Signs an account out everywhere Vodka put it: Roblox, the keyring and its Mocktail profile.
#[tauri::command]
async fn remove_account(app: AppHandle, state: State<'_, AppState>, account_id: u64) -> Result<()> {
    if instances::running_account_ids(&state.instances).contains(&account_id) {
        return Err(AppError::Other("That account is in a game. Close the game first.".into()));
    }
    if let Some(cookie) = blocking(move || accounts::load_session(account_id)).await? {
        // Best effort: the local copy is removed even if Roblox can't be reached.
        let _ = state.api.logout(&cookie).await;
    }
    blocking(move || {
        accounts::clear_session(account_id)?;
        let home = std::env::var_os("HOME").ok_or_else(|| AppError::Other("HOME is not set.".into()))?;
        let data = instances::profiles::flatpak_data(&PathBuf::from(home));
        instances::profiles::remove_tree(&instances::profiles::profile_paths(&data, account_id).root)
            .map_err(|e| AppError::Other(format!("Signed out, but the account's game profile couldn't be deleted: {e}")))
    })
    .await?;
    settings::update(&config_dir(&app)?, |s| s.remove(account_id))?;
    state.api.clear_cache();
    Ok(())
}

#[tauri::command]
async fn continue_playing(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<Game>> {
    state.api.continue_playing(&active_cookie(&app).await?.1).await
}

#[tauri::command]
async fn favorites(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<Game>> {
    let (account, cookie) = active_cookie(&app).await?;
    state.api.favorites(&cookie, account.id).await
}

#[tauri::command]
async fn launch_game(app: AppHandle, state: State<'_, AppState>, game: Game) -> Result<u32> {
    let status = blocking(|| Ok(mocktail::detect::detect())).await?;
    if !status.flatpak_available {
        return Err(AppError::FlatpakMissing);
    }
    if !status.installed {
        return Err(AppError::MocktailMissing);
    }
    let account = active_account(&app)?.ok_or(AppError::NotSignedIn)?;
    instances::launch(app, state.instances.clone(), account, game).await
}

#[tauri::command]
async fn list_instances(state: State<'_, AppState>) -> Result<Vec<InstanceView>> {
    let manager = state.instances.clone();
    blocking(move || Ok(instances::list(&manager))).await
}

#[tauri::command]
async fn stop_instance(state: State<'_, AppState>, id: u32, restart: bool) -> Result<()> {
    let manager = state.instances.clone();
    blocking(move || instances::stop(&manager, id, restart)).await
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Result<Settings> {
    Ok(settings::load(&config_dir(&app)?))
}

#[tauri::command]
fn set_flags(app: AppHandle, flags: Flags) -> Result<Settings> {
    settings::update(&config_dir(&app)?, |s| {
        if let Some(v) = flags.onboarded {
            s.onboarded = v;
        }
        if let Some(v) = flags.multi_instance_accepted {
            s.multi_instance_accepted = v;
        }
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState { api: roblox_api::Client::new(), instances: SharedManager::default() })
        .invoke_handler(tauri::generate_handler![
            mocktail_status,
            begin_login,
            current_account,
            list_accounts,
            switch_account,
            remove_account,
            continue_playing,
            favorites,
            launch_game,
            list_instances,
            stop_instance,
            get_settings,
            set_flags
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vodka Launcher");
}
