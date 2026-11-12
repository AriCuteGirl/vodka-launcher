mod accounts;
mod error;
mod instances;
mod mocktail;
mod roblox_api;
mod settings;
mod tweaks;

use error::{AppError, Result};
use instances::{InstanceView, SharedManager};
use mocktail::detect::MocktailStatus;
use roblox_api::{Chart, Game, Person, Presence, User};
use serde::{Deserialize, Serialize};
use settings::{SavedAccount, Settings};
use std::path::PathBuf;
use tweaks::LaunchOptions;
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
    /// Roblox client version of this account's profile; `None` before its first game.
    client_version: Option<String>,
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

fn flatpak_data() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or_else(|| AppError::Other("HOME is not set.".into()))?;
    Ok(instances::profiles::flatpak_data(&PathBuf::from(home)))
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
    let data = flatpak_data()?;
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
            client_version: instances::profiles::client_version(&instances::profiles::profile_paths(&data, a.id)),
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
        let data = flatpak_data()?;
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
async fn search_games(state: State<'_, AppState>, query: String) -> Result<Vec<Game>> {
    if query.chars().count() > 100 {
        return Err(AppError::Other("That search is too long.".into()));
    }
    state.api.search(&query).await
}

#[tauri::command]
async fn game_charts(state: State<'_, AppState>) -> Result<Vec<Chart>> {
    state.api.charts().await
}

/// Checks Mocktail is there and works out who is playing with which options.
async fn prepare_launch(app: &AppHandle) -> Result<(SavedAccount, Vec<(&'static str, String)>)> {
    let status = blocking(|| Ok(mocktail::detect::detect())).await?;
    if !status.flatpak_available {
        return Err(AppError::FlatpakMissing);
    }
    if !status.installed {
        return Err(AppError::MocktailMissing);
    }
    let saved = settings::load(&config_dir(app)?);
    let account = saved.active().cloned().ok_or(AppError::NotSignedIn)?;
    let options = tweaks::effective(&saved.defaults, saved.account_options.get(&account.id));
    // Saved options were validated when they were saved; check again in case the file was edited by hand.
    let options = tweaks::validate(&options).map_err(AppError::Launch)?;
    Ok((account, tweaks::to_env(&options)))
}

#[tauri::command]
async fn launch_game(app: AppHandle, state: State<'_, AppState>, game: Game) -> Result<u32> {
    let (account, env) = prepare_launch(&app).await?;
    let uri = mocktail::launch::launch_uri(game.place_id);
    instances::launch(app, state.instances.clone(), account, game, uri, env).await
}

#[tauri::command]
async fn list_friends(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<Person>> {
    let (account, cookie) = active_cookie(&app).await?;
    state.api.friends(&cookie, account.id).await
}

#[tauri::command]
async fn list_friend_requests(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<Person>> {
    state.api.friend_requests(&active_cookie(&app).await?.1).await
}

#[tauri::command]
async fn answer_friend_request(app: AppHandle, state: State<'_, AppState>, user_id: u64, accept: bool) -> Result<()> {
    state.api.answer_friend_request(&active_cookie(&app).await?.1, user_id, accept).await
}

/// Joins the server a friend is on, if Roblox lets us see it. Their presence is read fresh,
/// so a friend who just left is reported instead of joined.
#[tauri::command]
async fn join_friend(app: AppHandle, state: State<'_, AppState>, user_id: u64) -> Result<u32> {
    let (account, env) = prepare_launch(&app).await?;
    let cookie = blocking({
        let id = account.id;
        move || accounts::load_session(id)
    })
    .await?
    .ok_or(AppError::NotSignedIn)?;
    let presence = state.api.presences(&cookie, &[user_id]).await?.remove(&user_id);
    let presence = presence.ok_or_else(|| AppError::Other("Roblox didn't say where that friend is.".into()))?;
    let Some((place_id, game_id)) = presence.join_target() else {
        return Err(AppError::Other(match presence.kind {
            "in_game" => "That friend's game can't be joined: their privacy settings hide the server.".into(),
            _ => "That friend isn't in a game any more.".into(),
        }));
    };
    let uri = mocktail::launch::join_uri(place_id, game_id);
    let name = presence.location.clone().unwrap_or_else(|| "a friend's game".into());
    let game = state.api.game_for(presence.universe_id.unwrap_or(0), presence.root_place_id.unwrap_or(place_id), &name).await;
    instances::launch(app, state.instances.clone(), account, game, uri, env).await
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

/// Checks one FFlag and returns it as it will be saved.
#[tauri::command]
fn check_fflag(name: String, value: String) -> Result<(String, String)> {
    tweaks::normalize_flag(&name, &value).map_err(AppError::Other)
}

/// Saves launch options for every account (`account_id` empty) or for one account.
/// For an account, `options` empty removes its overrides.
#[tauri::command]
fn save_launch_options(app: AppHandle, account_id: Option<u64>, options: Option<LaunchOptions>) -> Result<Settings> {
    let options = options.map(|o| tweaks::validate(&o)).transpose().map_err(AppError::Other)?;
    settings::update(&config_dir(&app)?, |s| match (account_id, options) {
        (None, options) => s.defaults = options.unwrap_or_default(),
        (Some(id), Some(options)) => {
            s.account_options.insert(id, options);
        }
        (Some(id), None) => {
            s.account_options.remove(&id);
        }
    })
}

/// Puts an account's profile back on its previous Roblox client, using Mocktail's own updater,
/// and holds updates for that account so the next launch doesn't undo it.
#[tauri::command]
async fn rollback_client(app: AppHandle, state: State<'_, AppState>, account_id: u64) -> Result<Settings> {
    if instances::running_account_ids(&state.instances).contains(&account_id) {
        return Err(AppError::Other("That account is in a game. Close the game first.".into()));
    }
    let profile = instances::profiles::profile_paths(&flatpak_data()?, account_id);
    if instances::profiles::client_version(&profile).is_none() {
        return Err(AppError::Other("This account hasn't played yet, so there is nothing to roll back.".into()));
    }
    let output = tokio::process::Command::new("flatpak")
        .args(["run", "--command=/app/lib/mocktail/mocktail_updater", mocktail::detect::APP_ID, "rollback"])
        .env("MOCKTAIL_DATA_ROOT", &profile.data)
        .env("MOCKTAIL_CACHE_ROOT", &profile.cache)
        .env("MOCKTAIL_STATE_ROOT", &profile.state)
        .output()
        .await
        .map_err(|e| AppError::Other(format!("Could not run Mocktail's updater: {e}")))?;
    if !output.status.success() {
        let reason = String::from_utf8_lossy(&output.stderr);
        let reason = reason.lines().rev().find(|l| !l.trim().is_empty() && !l.starts_with("F:")).unwrap_or("no reason given");
        return Err(AppError::Other(format!("Mocktail couldn't roll back: {}", reason.trim())));
    }
    settings::update(&config_dir(&app)?, |s| {
        s.account_options.entry(account_id).or_default().hold_updates = Some(true);
    })
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
            search_games,
            game_charts,
            launch_game,
            list_friends,
            list_friend_requests,
            answer_friend_request,
            join_friend,
            list_instances,
            stop_instance,
            get_settings,
            set_flags,
            check_fflag,
            save_launch_options,
            rollback_client
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vodka Launcher");
}
