mod accounts;
mod chat;
mod error;
mod instances;
mod mocktail;
mod roblox_api;
mod settings;
mod tweaks;

use error::{AppError, Result};
use instances::{InstanceView, SharedManager};
use mocktail::detect::MocktailStatus;
use roblox_api::{Chart, Game, Outfit, Person, Presence, User};
use serde::{Deserialize, Serialize};
use settings::{SavedAccount, Settings};
use std::path::PathBuf;
use tweaks::LaunchOptions;
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    pub api: roblox_api::Client,
    pub chat: chat::Client,
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
        // Chat is only turned off here; the chat account can be deleted from the Chat page first.
        chat::clear_token(account_id)?;
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

/// Which server of a game to start in. Absent means "let Roblox choose".
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Target {
    /// One public server, by its id.
    #[serde(rename_all = "camelCase")]
    Server { place_id: u64, game_id: String },
    /// A private server, by its access code.
    #[serde(rename_all = "camelCase")]
    Private { access_code: String },
}

/// The launch link for a game and an optional server choice. Rejects ids that could alter the link.
fn link_for(game: &Game, target: Option<&Target>) -> Result<String> {
    match target {
        None => Ok(mocktail::launch::launch_uri(game.place_id)),
        Some(Target::Server { place_id, game_id }) if roblox_api::parse::valid_game_id(game_id) => {
            Ok(mocktail::launch::join_uri(*place_id, game_id))
        }
        Some(Target::Private { access_code }) if roblox_api::parse::valid_access_code(access_code) => {
            Ok(mocktail::launch::private_uri(game.place_id, access_code))
        }
        Some(_) => Err(AppError::Launch("that server id doesn't look right.".into())),
    }
}

/// Checks Mocktail is there and works out who is playing (the active account unless one is named) with which options.
async fn prepare_launch(app: &AppHandle, account_id: Option<u64>) -> Result<(SavedAccount, Vec<(&'static str, String)>)> {
    let status = blocking(|| Ok(mocktail::detect::detect())).await?;
    if !status.flatpak_available {
        return Err(AppError::FlatpakMissing);
    }
    if !status.installed {
        return Err(AppError::MocktailMissing);
    }
    let saved = settings::load(&config_dir(app)?);
    let account = match account_id {
        Some(id) => saved.accounts.iter().find(|a| a.id == id).cloned(),
        None => saved.active().cloned(),
    }
    .ok_or(AppError::NotSignedIn)?;
    let options = tweaks::modernize(tweaks::effective(&saved.defaults, saved.account_options.get(&account.id)));
    // Saved options were validated when they were saved; check again in case the file was edited by hand.
    let options = tweaks::validate(&options).map_err(AppError::Launch)?;
    Ok((account, tweaks::to_env(&options)))
}

/// Starts a game as the active account, or as `account_id`, optionally in a chosen server.
#[tauri::command]
async fn launch_game(
    app: AppHandle,
    state: State<'_, AppState>,
    game: Game,
    account_id: Option<u64>,
    target: Option<Target>,
) -> Result<u32> {
    let uri = link_for(&game, target.as_ref())?;
    let (account, env) = prepare_launch(&app, account_id).await?;
    instances::launch(app, state.instances.clone(), account, game, uri, env).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerLists {
    public: Vec<roblox_api::Server>,
    private: Vec<roblox_api::PrivateServer>,
    /// Why the private list is empty when that is due to an error rather than there being none.
    private_error: Option<String>,
}

/// Servers of a game: public ones with room (emptiest first) and private ones the active account can join.
#[tauri::command]
async fn list_servers(app: AppHandle, state: State<'_, AppState>, place_id: u64) -> Result<ServerLists> {
    let public = state.api.public_servers(place_id).await?;
    let (private, private_error) = match active_cookie(&app).await {
        Ok((_, cookie)) => match state.api.private_servers(&cookie, place_id).await {
            Ok(list) => (list, None),
            Err(e) => (vec![], Some(e.to_string())),
        },
        Err(e) => (vec![], Some(e.to_string())),
    };
    Ok(ServerLists { public, private, private_error })
}

/// Where a friend is right now, as something several accounts can then be sent to.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FriendLocation {
    game: Game,
    place_id: u64,
    game_id: String,
}

async fn locate_friend(app: &AppHandle, api: &roblox_api::Client, user_id: u64) -> Result<FriendLocation> {
    let (_, cookie) = active_cookie(app).await?;
    let presence = api.presences(&cookie, &[user_id]).await?.remove(&user_id);
    let mut presence = presence.ok_or_else(|| AppError::Other("Roblox didn't say where that friend is.".into()))?;
    if presence.join_target().is_none() {
        if let Some(own) = own_presence(app, api, user_id).await {
            presence = own;
        }
    }
    let Some((place_id, game_id)) = presence.join_target() else {
        return Err(AppError::Other(match presence.kind {
            "in_game" => "That friend's game can't be joined: their privacy settings hide the server.".into(),
            _ => "That friend isn't in a game any more.".into(),
        }));
    };
    let name = presence.location.clone().unwrap_or_else(|| "a friend's game".into());
    let game = api.game_for(presence.universe_id.unwrap_or(0), presence.root_place_id.unwrap_or(place_id), &name).await;
    Ok(FriendLocation { game, place_id, game_id: game_id.to_owned() })
}

/// Looks up a friend's server so the UI can send one or several accounts there with `launch_game`.
#[tauri::command]
async fn friend_location(app: AppHandle, state: State<'_, AppState>, user_id: u64) -> Result<FriendLocation> {
    locate_friend(&app, &state.api, user_id).await
}

/// Where one of Vodka's own accounts is, asked with that account's own session.
/// An account can always see its own server, even with "who can join me" turned off,
/// so this lets your accounts join each other.
async fn own_presence(app: &AppHandle, api: &roblox_api::Client, account_id: u64) -> Option<Presence> {
    let saved = settings::load(&config_dir(app).ok()?);
    saved.accounts.iter().find(|a| a.id == account_id)?;
    let cookie = blocking(move || accounts::load_session(account_id)).await.ok()??;
    api.presences(&cookie, &[account_id]).await.ok()?.remove(&account_id)
}

#[tauri::command]
async fn list_friends(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<Person>> {
    let (account, cookie) = active_cookie(&app).await?;
    let mut friends = state.api.friends(&cookie, account.id).await?;
    let mine: Vec<u64> = settings::load(&config_dir(&app)?).accounts.iter().map(|a| a.id).collect();
    for friend in friends.iter_mut().filter(|f| mine.contains(&f.id) && f.presence.join_target().is_none()) {
        if let Some(own) = own_presence(&app, &state.api, friend.id).await {
            if own.join_target().is_some() {
                friend.presence = own;
            }
        }
    }
    Ok(friends)
}

#[tauri::command]
async fn list_friend_requests(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<Person>> {
    state.api.friend_requests(&active_cookie(&app).await?.1).await
}

#[tauri::command]
async fn answer_friend_request(app: AppHandle, state: State<'_, AppState>, user_id: u64, accept: bool) -> Result<()> {
    state.api.answer_friend_request(&active_cookie(&app).await?.1, user_id, accept).await
}

/// Joins the server a friend is on as the active account. Their presence is read fresh,
/// so a friend who just left is reported instead of joined.
#[tauri::command]
async fn join_friend(app: AppHandle, state: State<'_, AppState>, user_id: u64) -> Result<u32> {
    let (account, env) = prepare_launch(&app, None).await?;
    let at = locate_friend(&app, &state.api, user_id).await?;
    let uri = mocktail::launch::join_uri(at.place_id, &at.game_id);
    instances::launch(app, state.instances.clone(), account, at.game, uri, env).await
}

/// Players can switch games inside Roblox without Vodka knowing. This asks Roblox where each
/// playing account really is and corrects the game shown for it. Returns whether anything changed.
#[tauri::command]
async fn refresh_instance_games(app: AppHandle, state: State<'_, AppState>) -> Result<bool> {
    let mut changed = false;
    for (instance_id, account_id, universe_id) in instances::running_games(&state.instances) {
        let Some(here) = own_presence(&app, &state.api, account_id).await else { continue };
        let (Some(now_universe), Some(place)) = (here.universe_id, here.root_place_id.or(here.place_id)) else { continue };
        if here.kind != "in_game" || now_universe == universe_id {
            continue;
        }
        let name = here.location.clone().unwrap_or_else(|| "A game".into());
        let game = state.api.game_for(now_universe, place, &name).await;
        instances::set_game(&state.instances, instance_id, game);
        changed = true;
    }
    Ok(changed)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AvatarView {
    picture_url: Option<String>,
    outfits: Vec<Outfit>,
}

/// The active account's current look and saved outfits.
#[tauri::command]
async fn avatar(app: AppHandle, state: State<'_, AppState>) -> Result<AvatarView> {
    let (account, cookie) = active_cookie(&app).await?;
    let outfits = state.api.outfits(&cookie, account.id).await?;
    let picture_url = state.api.avatar_picture(account.id).await.unwrap_or(None);
    Ok(AvatarView { picture_url, outfits })
}

/// Profile pictures for any Roblox users (public data), keyed by user id.
#[tauri::command]
async fn headshots(state: State<'_, AppState>, user_ids: Vec<u64>) -> Result<std::collections::HashMap<u64, String>> {
    if user_ids.len() > 200 {
        return Err(AppError::Other("That's too many pictures to ask for at once.".into()));
    }
    state.api.headshots(&user_ids).await
}

#[tauri::command]
async fn avatar_picture(app: AppHandle, state: State<'_, AppState>) -> Result<Option<String>> {
    let account = active_account(&app)?.ok_or(AppError::NotSignedIn)?;
    state.api.avatar_picture(account.id).await
}

/// Puts one of the active account's saved outfits on. Returns the parts Roblox refused.
#[tauri::command]
async fn wear_outfit(app: AppHandle, state: State<'_, AppState>, outfit_id: u64) -> Result<Vec<String>> {
    let (_, cookie) = active_cookie(&app).await?;
    state.api.wear_outfit(&cookie, outfit_id).await
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

/// The active account and its chat sign-in, or `ChatSignedOut` when chat isn't turned on for it.
async fn chat_session(app: &AppHandle) -> Result<(SavedAccount, String)> {
    let account = active_account(app)?.ok_or(AppError::NotSignedIn)?;
    let id = account.id;
    let token = blocking(move || chat::load_token(id)).await?.ok_or(AppError::ChatSignedOut)?;
    Ok((account, token))
}

/// Runs a signed-in chat action. A sign-in the server no longer accepts is forgotten locally.
async fn chat_call(app: &AppHandle, state: &AppState, action: &str, body: serde_json::Value) -> Result<serde_json::Value> {
    let (account, token) = chat_session(app).await?;
    match state.chat.call(Some(&token), body, action).await {
        Err(AppError::ChatSignedOut) => {
            let id = account.id;
            let _ = blocking(move || chat::clear_token(id)).await;
            Err(AppError::ChatSignedOut)
        }
        other => other,
    }
}

/// Whether chat is turned on for the active account on this computer.
#[tauri::command]
async fn chat_enabled(app: AppHandle) -> Result<bool> {
    match chat_session(&app).await {
        Ok(_) => Ok(true),
        Err(AppError::ChatSignedOut) | Err(AppError::NotSignedIn) => Ok(false),
        Err(e) => Err(e),
    }
}

/// Asks the chat server for the phrase to put in the Roblox profile.
#[tauri::command]
async fn chat_start_verify(app: AppHandle, state: State<'_, AppState>) -> Result<serde_json::Value> {
    let account = active_account(&app)?.ok_or(AppError::NotSignedIn)?;
    state.chat.call(None, serde_json::json!({ "robloxId": account.id }), "start_verify").await
}

/// Asks the chat server to look for the phrase; on success the chat sign-in goes into the keyring.
#[tauri::command]
async fn chat_finish_verify(app: AppHandle, state: State<'_, AppState>) -> Result<serde_json::Value> {
    let account = active_account(&app)?.ok_or(AppError::NotSignedIn)?;
    let mut answer = state.chat.call(None, serde_json::json!({ "robloxId": account.id }), "finish_verify").await?;
    let token = answer.get("token").and_then(|t| t.as_str()).unwrap_or_default().to_owned();
    let id = account.id;
    blocking(move || chat::store_token(id, &token)).await?;
    // The sign-in stays in the backend; the UI only needs to know who it is.
    if let Some(map) = answer.as_object_mut() {
        map.remove("token");
    }
    Ok(answer)
}

#[tauri::command]
async fn chat_sync(app: AppHandle, state: State<'_, AppState>, since: u64) -> Result<serde_json::Value> {
    chat_call(&app, &state, "sync", serde_json::json!({ "since": since })).await
}

#[tauri::command]
async fn chat_add_contact(app: AppHandle, state: State<'_, AppState>, code: String) -> Result<serde_json::Value> {
    chat_call(&app, &state, "add_contact", serde_json::json!({ "code": code })).await
}

#[tauri::command]
async fn chat_remove_contact(app: AppHandle, state: State<'_, AppState>, user_id: u64) -> Result<serde_json::Value> {
    chat_call(&app, &state, "remove_contact", serde_json::json!({ "robloxId": user_id })).await
}

#[tauri::command]
async fn chat_block(app: AppHandle, state: State<'_, AppState>, user_id: u64, blocked: bool) -> Result<serde_json::Value> {
    chat_call(&app, &state, if blocked { "block" } else { "unblock" }, serde_json::json!({ "robloxId": user_id })).await
}

#[tauri::command]
async fn chat_report(app: AppHandle, state: State<'_, AppState>, user_id: u64, reason: String) -> Result<serde_json::Value> {
    chat_call(&app, &state, "report", serde_json::json!({ "robloxId": user_id, "reason": reason })).await
}

#[tauri::command]
async fn chat_send(app: AppHandle, state: State<'_, AppState>, to: u64, body: String) -> Result<serde_json::Value> {
    chat_call(&app, &state, "send", serde_json::json!({ "to": to, "kind": "text", "body": body })).await
}

/// Invites someone to the game the active account is playing through Vodka, with its exact server when Roblox shares it.
#[tauri::command]
async fn chat_invite(app: AppHandle, state: State<'_, AppState>, to: u64) -> Result<serde_json::Value> {
    let account = active_account(&app)?.ok_or(AppError::NotSignedIn)?;
    let manager = state.instances.clone();
    let running = blocking(move || Ok(instances::list(&manager))).await?;
    let mine = running
        .into_iter()
        .find(|i| i.account_id == account.id)
        .ok_or_else(|| AppError::Chat("Start a game first, then invite people to it.".into()))?;
    let here = own_presence(&app, &state.api, account.id).await;
    let server = here.as_ref().and_then(|p| p.join_target());
    let payload = serde_json::json!({
        "game": mine.game,
        "placeId": server.map(|(place, _)| place).unwrap_or(mine.game.place_id),
        "gameId": server.map(|(_, id)| id),
    });
    chat_call(&app, &state, "send", serde_json::json!({ "to": to, "kind": "invite", "body": "", "payload": payload })).await
}

/// Turns chat off on this computer for the active account. The chat account itself stays.
#[tauri::command]
async fn chat_sign_out(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    let account = active_account(&app)?.ok_or(AppError::NotSignedIn)?;
    let _ = chat_call(&app, &state, "sign_out", serde_json::json!({})).await;
    let id = account.id;
    blocking(move || chat::clear_token(id)).await
}

/// Deletes the active account's chat account on the server: contacts, messages, everything.
#[tauri::command]
async fn chat_delete_account(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    let account = active_account(&app)?.ok_or(AppError::NotSignedIn)?;
    chat_call(&app, &state, "delete_account", serde_json::json!({})).await?;
    let id = account.id;
    blocking(move || chat::clear_token(id)).await
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
        .manage(AppState { api: roblox_api::Client::new(), chat: chat::Client::new(), instances: SharedManager::default() })
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
            friend_location,
            list_servers,
            list_instances,
            refresh_instance_games,
            avatar,
            avatar_picture,
            headshots,
            wear_outfit,
            stop_instance,
            get_settings,
            set_flags,
            check_fflag,
            save_launch_options,
            rollback_client,
            chat_enabled,
            chat_start_verify,
            chat_finish_verify,
            chat_sync,
            chat_add_contact,
            chat_remove_contact,
            chat_block,
            chat_report,
            chat_send,
            chat_invite,
            chat_sign_out,
            chat_delete_account
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vodka Launcher");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Game {
        Game { universe_id: 1, place_id: 10, name: "G".into(), playing: None, icon_url: None }
    }

    #[test]
    fn links_follow_the_chosen_server() {
        assert_eq!(link_for(&game(), None).unwrap(), "roblox://experiences/start?placeId=10");
        let server = Target::Server { place_id: 11, game_id: "0f0e-11".into() };
        assert_eq!(link_for(&game(), Some(&server)).unwrap(), "roblox://experiences/start?placeId=11&gameInstanceId=0f0e-11");
        let private = Target::Private { access_code: "3f2a_9c".into() };
        assert_eq!(link_for(&game(), Some(&private)).unwrap(), "roblox://experiences/start?placeId=10&accessCode=3f2a_9c");
    }

    #[test]
    fn ids_that_could_change_the_link_are_refused() {
        let server = Target::Server { place_id: 11, game_id: "x&placeId=666".into() };
        assert!(link_for(&game(), Some(&server)).is_err());
        let private = Target::Private { access_code: "a b".into() };
        assert!(link_for(&game(), Some(&private)).is_err());
    }

    #[test]
    fn targets_arrive_from_the_ui_in_camel_case() {
        let t: Target = serde_json::from_str(r#"{"kind":"server","placeId":5,"gameId":"ab-1"}"#).unwrap();
        assert!(matches!(t, Target::Server { place_id: 5, .. }));
        let t: Target = serde_json::from_str(r#"{"kind":"private","accessCode":"zz"}"#).unwrap();
        assert!(matches!(t, Target::Private { .. }));
    }
}
