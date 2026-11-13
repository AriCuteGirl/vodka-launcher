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
use roblox_api::{Chart, Game, GameStanding, Item, Outfit, Person, Presence, User};
use serde::{Deserialize, Serialize};
use settings::{SavedAccount, Settings};
use std::path::PathBuf;
use tweaks::LaunchOptions;
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    pub api: roblox_api::Client,
    pub chat: chat::Client,
    pub instances: SharedManager,
    /// The account this window is using. Kept per running Vodka, so two Vodka windows can use
    /// two different accounts; the settings file only remembers which one to start with.
    pub active: std::sync::Mutex<Option<u64>>,
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

/// Which saved account this window uses: its own choice if it made one and the account still exists,
/// otherwise the one remembered in settings.
fn pick_active(saved: &Settings, chosen: Option<u64>) -> Option<SavedAccount> {
    chosen.and_then(|id| saved.accounts.iter().find(|a| a.id == id)).or_else(|| saved.active()).cloned()
}

fn active_account(app: &AppHandle) -> Result<Option<SavedAccount>> {
    let saved = settings::load(&config_dir(app)?);
    let state = app.state::<AppState>();
    let mut chosen = state.active.lock().unwrap();
    let account = pick_active(&saved, *chosen);
    // Pin the first answer. Otherwise a window that never switched would follow whatever
    // another Vodka window last saved, and start acting as that window's account.
    if chosen.is_none() {
        *chosen = account.as_ref().map(|a| a.id);
    }
    Ok(account)
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
    let active_id = active_account(&app)?.map(|a| a.id);
    Ok(saved
        .accounts
        .iter()
        .map(|a| AccountView {
            id: a.id,
            name: a.name.clone(),
            display_name: a.display_name.clone(),
            avatar_url: avatars.get(&a.id).cloned(),
            active: active_id == Some(a.id),
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
    *state.active.lock().unwrap() = Some(account_id);
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
async fn prepare_launch(app: &AppHandle, account_id: Option<u64>) -> Result<(SavedAccount, Vec<(&'static str, String)>, bool)> {
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
        None => active_account(app)?,
    }
    .ok_or(AppError::NotSignedIn)?;
    let options = tweaks::modernize(tweaks::effective(&saved.defaults, saved.account_options.get(&account.id)));
    // Saved options were validated when they were saved; check again in case the file was edited by hand.
    let options = tweaks::validate(&options).map_err(AppError::Launch)?;
    Ok((account, tweaks::to_env(&options), options.auto_rejoin == Some(true)))
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
    let (account, env, auto_rejoin) = prepare_launch(&app, account_id).await?;
    instances::launch(app, state.instances.clone(), account, game, uri, env, auto_rejoin).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Regrouped {
    /// How many games were sent to the server.
    moved: usize,
    playing: u32,
    max_players: u32,
}

/// Moves every account this window is running in one game into the same quiet public server.
/// Each of those games is restarted to get there, since a running game can't be redirected from outside.
#[tauri::command]
async fn regroup(state: State<'_, AppState>, universe_id: u64) -> Result<Regrouped> {
    let manager = state.instances.clone();
    let running = blocking(move || Ok(instances::list(&manager))).await?;
    let group: Vec<_> = running.into_iter().filter(|i| i.game.universe_id == universe_id).collect();
    let Some(first) = group.first() else {
        return Err(AppError::Other("None of your accounts is in that game any more.".into()));
    };
    let place_id = first.game.place_id;
    let servers = state.api.public_servers(place_id).await?;
    let server = roblox_api::parse::server_for_group(&servers, group.len() as u32)
        .ok_or_else(|| AppError::Other("Roblox listed no public server with room for that game right now.".into()))?;
    let uri = mocktail::launch::join_uri(place_id, &server.id);
    let mut moved = 0;
    for inst in &group {
        let manager = state.instances.clone();
        let (id, uri) = (inst.id, uri.clone());
        if blocking(move || instances::restart_into(&manager, id, uri)).await.is_ok() {
            moved += 1;
        }
        // One at a time: signing several accounts in at the same instant makes Roblox's
        // sign-in service refuse some of them.
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
    }
    Ok(Regrouped { moved, playing: server.playing, max_players: server.max_players })
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
    let (account, env, auto_rejoin) = prepare_launch(&app, None).await?;
    let at = locate_friend(&app, &state.api, user_id).await?;
    let uri = mocktail::launch::join_uri(at.place_id, &at.game_id);
    instances::launch(app, state.instances.clone(), account, at.game, uri, env, auto_rejoin).await
}

/// Players can switch games inside Roblox without Vodka knowing. This asks Roblox where each
/// playing account really is and corrects the game shown for it. Returns whether anything changed.
#[tauri::command]
async fn refresh_instance_games(app: AppHandle, state: State<'_, AppState>) -> Result<bool> {
    let mut changed = false;
    for (instance_id, account_id, universe_id) in instances::running_games(&state.instances) {
        let Some(here) = own_presence(&app, &state.api, account_id).await else { continue };
        // Out of the game for two checks in a row, with auto-rejoin on: bring the account back.
        if instances::note_presence(&state.instances, instance_id, here.kind == "in_game") {
            let manager = state.instances.clone();
            let _ = blocking(move || instances::stop_for_rejoin(&manager, instance_id)).await;
            changed = true;
            continue;
        }
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

/// Whether the active account favorited and rated a game.
#[tauri::command]
async fn game_standing(app: AppHandle, state: State<'_, AppState>, universe_id: u64) -> Result<GameStanding> {
    state.api.game_standing(&active_cookie(&app).await?.1, universe_id).await
}

#[tauri::command]
async fn set_favorite(app: AppHandle, state: State<'_, AppState>, universe_id: u64, favorited: bool) -> Result<GameStanding> {
    let (_, cookie) = active_cookie(&app).await?;
    state.api.set_favorite(&cookie, universe_id, favorited).await?;
    state.api.game_standing(&cookie, universe_id).await
}

#[tauri::command]
async fn set_vote(app: AppHandle, state: State<'_, AppState>, universe_id: u64, like: bool) -> Result<GameStanding> {
    let (_, cookie) = active_cookie(&app).await?;
    state.api.set_vote(&cookie, universe_id, like).await?;
    state.api.game_standing(&cookie, universe_id).await
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ItemPage {
    items: Vec<Item>,
    next: Option<String>,
}

/// Which of the account's own items a tab of the avatar editor lists: its key and Roblox's type names.
const INVENTORY_TABS: [(&str, &str); 6] = [
    ("accessories", "Hat,HairAccessory,FaceAccessory,NeckAccessory,ShoulderAccessory,FrontAccessory,BackAccessory,WaistAccessory"),
    ("shirts", "Shirt"),
    ("pants", "Pants"),
    ("tshirts", "TShirt"),
    ("faces", "Face"),
    ("gear", "Gear"),
];

/// What the active account is wearing right now.
#[tauri::command]
async fn avatar_wearing(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<Item>> {
    let (_, cookie) = active_cookie(&app).await?;
    Ok(state.api.wearing(&cookie).await?.0)
}

/// One page of the active account's own items in an editor tab.
#[tauri::command]
async fn avatar_inventory(app: AppHandle, state: State<'_, AppState>, tab: String, cursor: Option<String>) -> Result<ItemPage> {
    let types = INVENTORY_TABS.iter().find(|(key, _)| *key == tab).map(|(_, types)| *types);
    let types = types.ok_or_else(|| AppError::Other("Vodka doesn't know that kind of item.".into()))?;
    let (account, cookie) = active_cookie(&app).await?;
    let (items, next) = state.api.inventory(&cookie, account.id, types, cursor.as_deref()).await?;
    Ok(ItemPage { items, next })
}

/// Makes the active account wear exactly these items. Returns the ids Roblox refused.
#[tauri::command]
async fn avatar_set_items(app: AppHandle, state: State<'_, AppState>, asset_ids: Vec<u64>) -> Result<Vec<u64>> {
    if asset_ids.len() > 80 {
        return Err(AppError::Other("That's more items than an avatar can wear.".into()));
    }
    let mut ids = asset_ids;
    ids.retain(|id| *id > 0);
    let mut seen = std::collections::HashSet::new();
    ids.retain(|id| seen.insert(*id));
    let (_, cookie) = active_cookie(&app).await?;
    state.api.set_wearing(&cookie, &ids).await
}

/// Roblox's "Advanced" box: wear exactly these accessories (up to ten), by item id.
#[tauri::command]
async fn avatar_set_accessories(app: AppHandle, state: State<'_, AppState>, asset_ids: Vec<u64>) -> Result<Vec<u64>> {
    if asset_ids.len() > roblox_api::parse::MAX_ADVANCED_ACCESSORIES {
        return Err(AppError::Other(format!("Roblox allows up to {} accessories this way.", roblox_api::parse::MAX_ADVANCED_ACCESSORIES)));
    }
    let (_, cookie) = active_cookie(&app).await?;
    state.api.set_accessories(&cookie, &asset_ids).await
}

/// Saves the active account's current look as a new outfit.
#[tauri::command]
async fn avatar_save_outfit(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<()> {
    let name = name.trim().to_owned();
    if name.is_empty() || name.chars().count() > 25 {
        return Err(AppError::Other("Give the outfit a name of 1 to 25 characters.".into()));
    }
    let (_, cookie) = active_cookie(&app).await?;
    state.api.save_outfit(&cookie, &name).await
}

/// Searches the avatar catalog. Works signed out.
#[tauri::command]
async fn catalog_search(state: State<'_, AppState>, keyword: String, category: u32, sort: u32, cursor: Option<String>) -> Result<ItemPage> {
    if keyword.chars().count() > 100 {
        return Err(AppError::Other("That search is too long.".into()));
    }
    if ![1, 3, 4, 11, 12].contains(&category) || sort > 5 {
        return Err(AppError::Other("Vodka doesn't know that catalog filter.".into()));
    }
    let (items, next) = state.api.catalog(&keyword, category, sort, cursor.as_deref()).await?;
    Ok(ItemPage { items, next })
}

/// The active account's Robux balance.
#[tauri::command]
async fn robux_balance(app: AppHandle, state: State<'_, AppState>) -> Result<u64> {
    let (account, cookie) = active_cookie(&app).await?;
    state.api.robux(&cookie, account.id).await
}

/// What buying an item would involve for the active account: price, balance, whether it's possible. Changes nothing.
#[tauri::command]
async fn item_offer(app: AppHandle, state: State<'_, AppState>, asset_id: u64) -> Result<roblox_api::Offer> {
    let (account, cookie) = active_cookie(&app).await?;
    state.api.offer(&cookie, account.id, asset_id).await
}

/// A random identifier Roblox uses to make sure a retried purchase isn't charged twice.
fn purchase_key() -> Result<String> {
    use std::io::Read;
    let mut bytes = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|e| AppError::Other(format!("Could not prepare the purchase: {e}")))?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32]))
}

/// Buys an item for the active account with its Robux, at exactly the price the user confirmed.
/// Returns the offer afterwards (now owned, new balance).
#[tauri::command]
async fn buy_item(app: AppHandle, state: State<'_, AppState>, asset_id: u64, agreed_price: u64) -> Result<roblox_api::Offer> {
    let (account, cookie) = active_cookie(&app).await?;
    state.api.buy(&cookie, account.id, asset_id, agreed_price, &purchase_key()?).await?;
    state.api.clear_cache();
    state.api.offer(&cookie, account.id, asset_id).await
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

/// The game the active account is playing through Vodka, as an invite: with its exact server when Roblox shares it.
async fn my_game_invite(app: &AppHandle, state: &AppState) -> Result<serde_json::Value> {
    let account = active_account(app)?.ok_or(AppError::NotSignedIn)?;
    let manager = state.instances.clone();
    let running = blocking(move || Ok(instances::list(&manager))).await?;
    let mine = running
        .into_iter()
        .find(|i| i.account_id == account.id)
        .ok_or_else(|| AppError::Chat("Start a game first, then invite people to it.".into()))?;
    let here = own_presence(app, &state.api, account.id).await;
    let server = here.as_ref().and_then(|p| p.join_target());
    Ok(serde_json::json!({
        "game": mine.game,
        "placeId": server.map(|(place, _)| place).unwrap_or(mine.game.place_id),
        "gameId": server.map(|(_, id)| id),
    }))
}

/// Invites one contact to the game the active account is playing.
#[tauri::command]
async fn chat_invite(app: AppHandle, state: State<'_, AppState>, to: u64) -> Result<serde_json::Value> {
    let payload = my_game_invite(&app, &state).await?;
    chat_call(&app, &state, "send", serde_json::json!({ "to": to, "kind": "invite", "body": "", "payload": payload })).await
}

/// Runs a party action for the active account. "launch" calls the whole party into the game being played.
#[tauri::command]
async fn party(app: AppHandle, state: State<'_, AppState>, action: String, args: Option<serde_json::Value>) -> Result<serde_json::Value> {
    let mut body = args.filter(|a| a.is_object()).unwrap_or_else(|| serde_json::json!({}));
    if action == "launch" {
        body = serde_json::json!({ "payload": my_game_invite(&app, &state).await? });
    }
    let (account, token) = chat_session(&app).await?;
    match state.chat.party(&token, body, &action).await {
        Err(AppError::ChatSignedOut) => {
            let id = account.id;
            let _ = blocking(move || chat::clear_token(id)).await;
            Err(AppError::ChatSignedOut)
        }
        other => other,
    }
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
        .manage(AppState {
            api: roblox_api::Client::new(),
            chat: chat::Client::new(),
            instances: SharedManager::default(),
            active: std::sync::Mutex::new(None),
        })
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
            regroup,
            list_instances,
            refresh_instance_games,
            game_standing,
            set_favorite,
            set_vote,
            avatar,
            avatar_picture,
            avatar_wearing,
            avatar_inventory,
            avatar_set_items,
            avatar_set_accessories,
            avatar_save_outfit,
            catalog_search,
            robux_balance,
            item_offer,
            buy_item,
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
            party,
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
    fn each_window_keeps_its_own_active_account() {
        let acct = |id| SavedAccount { id, name: format!("u{id}"), display_name: format!("U{id}") };
        let saved = Settings { accounts: vec![acct(1), acct(2)], active_account: Some(1), ..Default::default() };
        // No choice yet: start with the remembered account.
        assert_eq!(pick_active(&saved, None).unwrap().id, 1);
        // This window chose account 2, whatever another window saved since.
        assert_eq!(pick_active(&saved, Some(2)).unwrap().id, 2);
        // Its choice was removed elsewhere: fall back instead of acting as nobody.
        assert_eq!(pick_active(&saved, Some(9)).unwrap().id, 1);
        assert!(pick_active(&Settings::default(), Some(2)).is_none());
    }

    #[test]
    fn purchase_keys_are_random_uuids() {
        let (a, b) = (purchase_key().unwrap(), purchase_key().unwrap());
        assert_ne!(a, b);
        assert_eq!(a.len(), 36);
        assert_eq!(a.as_bytes()[14], b'4');
        assert!(a.bytes().all(|c| c == b'-' || c.is_ascii_hexdigit()));
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
