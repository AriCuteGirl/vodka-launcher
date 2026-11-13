//! Typed Roblox web API client: allowlisted hosts, short cache, 429 backoff.

pub mod parse;

use crate::error::{AppError, Result};
pub use parse::{Chart, Game, GameStanding, Item, Offer, Outfit, Person, Presence, PrivateServer, Server, User};
use reqwest::{StatusCode, Url, header};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const RETRIES: u32 = 3;
const LIST_TTL: Duration = Duration::from_secs(60);
const THUMB_TTL: Duration = Duration::from_secs(600);
const CHARTS_TTL: Duration = Duration::from_secs(300);
const GAMES_PER_CHART: usize = 24;
/// The thumbnails API takes at most this many ids per request.
const ICON_BATCH: usize = 100;
const USER_BATCH: usize = 100;
const PRESENCE_BATCH: usize = 50;
const NAMES_TTL: Duration = Duration::from_secs(600);

/// Only https://roblox.com and its subdomains may ever be contacted.
pub fn check_url(url: &Url) -> Result<()> {
    let host = url.host_str().unwrap_or_default();
    let ok = url.scheme() == "https"
        && (host == "roblox.com" || host.ends_with(".roblox.com"))
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none();
    if ok { Ok(()) } else { Err(AppError::BlockedUrl) }
}

/// How long to wait before retry number `attempt` (0-based).
pub fn backoff(attempt: u32, retry_after: Option<u64>) -> Duration {
    let base = Duration::from_millis(500 * 2u64.pow(attempt));
    retry_after.map(Duration::from_secs).unwrap_or(base).min(Duration::from_secs(5))
}

fn join_ids(ids: &[u64]) -> String {
    ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",")
}

pub struct Client {
    http: reqwest::Client,
    cache: Mutex<HashMap<String, (Instant, String)>>,
}

impl Client {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("VodkaLauncher/", env!("CARGO_PKG_VERSION")))
            // Redirects are never followed, so a cookie can't be carried elsewhere.
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .build()
            .expect("HTTP client");
        Self { http, cache: Mutex::new(HashMap::new()) }
    }

    pub fn clear_cache(&self) {
        self.cache.lock().unwrap().clear();
    }

    async fn send(&self, url: &str, body: Option<&serde_json::Value>, cookie: Option<&str>) -> Result<String> {
        self.request(false, url, body, cookie).await
    }

    /// GET without a body, POST with one, or PATCH with one when `patch` is set.
    async fn request(&self, patch: bool, url: &str, body: Option<&serde_json::Value>, cookie: Option<&str>) -> Result<String> {
        let parsed = Url::parse(url).map_err(|_| AppError::BlockedUrl)?;
        check_url(&parsed)?;
        let mut csrf: Option<String> = None;
        let mut attempt = 0;
        loop {
            let mut req = match body {
                Some(json) if patch => self.http.patch(parsed.clone()).json(json),
                Some(json) => self.http.post(parsed.clone()).json(json),
                None => self.http.get(parsed.clone()),
            }
            .header(header::ACCEPT, "application/json");
            if let Some(c) = cookie {
                req = req.header(header::COOKIE, format!(".ROBLOSECURITY={c}"));
            }
            if let Some(token) = &csrf {
                req = req.header("x-csrf-token", token);
            }
            let res = req.send().await.map_err(|e| {
                if e.is_connect() || e.is_timeout() { AppError::Offline } else { AppError::Api(e.without_url().to_string()) }
            })?;
            let status = res.status();
            if status == StatusCode::FORBIDDEN && csrf.is_none() && body.is_some() {
                if let Some(token) = res.headers().get("x-csrf-token").and_then(|v| v.to_str().ok()) {
                    csrf = Some(token.to_owned());
                    continue;
                }
            }
            if status == StatusCode::TOO_MANY_REQUESTS {
                if attempt == RETRIES {
                    return Err(AppError::RateLimited);
                }
                let retry_after = res
                    .headers()
                    .get(header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse().ok());
                tokio::time::sleep(backoff(attempt, retry_after)).await;
                attempt += 1;
                continue;
            }
            if status == StatusCode::UNAUTHORIZED {
                return Err(if cookie.is_some() { AppError::SessionExpired } else { AppError::NotSignedIn });
            }
            if !status.is_success() {
                return Err(AppError::Api(format!("HTTP {}", status.as_u16())));
            }
            return res.text().await.map_err(|e| AppError::Api(e.without_url().to_string()));
        }
    }

    /// GET with a cache. Signed-in responses are cached per call site, never across accounts:
    /// the cache is cleared on sign-in and sign-out.
    async fn get(&self, url: &str, cookie: Option<&str>, ttl: Duration) -> Result<String> {
        if let Some((at, body)) = self.cache.lock().unwrap().get(url) {
            if at.elapsed() < ttl {
                return Ok(body.clone());
            }
        }
        let body = self.send(url, None, cookie).await?;
        self.cache.lock().unwrap().insert(url.to_owned(), (Instant::now(), body.clone()));
        Ok(body)
    }

    pub async fn current_user(&self, cookie: &str) -> Result<User> {
        let body = self.send("https://users.roblox.com/v1/users/authenticated", None, Some(cookie)).await?;
        let mut user = parse::authenticated_user(&body)?;
        let url = format!(
            "https://thumbnails.roblox.com/v1/users/avatar-headshot?userIds={}&size=150x150&format=Png&isCircular=false",
            user.id
        );
        // A missing avatar is not worth failing sign-in for.
        if let Ok(body) = self.get(&url, None, THUMB_TTL).await {
            user.avatar_url = parse::thumbnails(&body).ok().and_then(|mut m| m.remove(&user.id));
        }
        Ok(user)
    }

    /// Avatar pictures for any users. Public, no session needed.
    pub async fn headshots(&self, ids: &[u64]) -> Result<HashMap<u64, String>> {
        let mut all = HashMap::new();
        for batch in ids.chunks(USER_BATCH) {
            let url = format!(
                "https://thumbnails.roblox.com/v1/users/avatar-headshot?userIds={}&size=150x150&format=Png&isCircular=false",
                join_ids(batch)
            );
            all.extend(parse::thumbnails(&self.get(&url, None, THUMB_TTL).await?)?);
        }
        Ok(all)
    }

    pub async fn presences(&self, cookie: &str, ids: &[u64]) -> Result<HashMap<u64, Presence>> {
        let mut all = HashMap::new();
        for batch in ids.chunks(PRESENCE_BATCH) {
            let request = serde_json::json!({ "userIds": batch });
            let body = self.send("https://presence.roblox.com/v1/presence/users", Some(&request), Some(cookie)).await?;
            all.extend(parse::presences(&body)?);
        }
        Ok(all)
    }

    /// Usernames and display names. Public; cached, since names rarely change.
    async fn names(&self, ids: &[u64]) -> Result<HashMap<u64, (String, String)>> {
        let mut all = HashMap::new();
        for batch in ids.chunks(USER_BATCH) {
            let key = format!("names:{}", join_ids(batch));
            let cached = self.cache.lock().unwrap().get(&key).filter(|(at, _)| at.elapsed() < NAMES_TTL).map(|(_, b)| b.clone());
            let body = match cached {
                Some(body) => body,
                None => {
                    let request = serde_json::json!({ "userIds": batch, "excludeBannedUsers": false });
                    let body = self.send("https://users.roblox.com/v1/users", Some(&request), None).await?;
                    self.cache.lock().unwrap().insert(key, (Instant::now(), body.clone()));
                    body
                }
            };
            all.extend(parse::user_names(&body)?);
        }
        Ok(all)
    }

    /// Turns ids into people with names, pictures and (optionally) presence.
    async fn people(&self, cookie: &str, ids: &[u64], with_presence: bool) -> Result<Vec<Person>> {
        let names = self.names(ids).await?;
        // Pictures and presence are extras; a list of names is still useful without them.
        let avatars = self.headshots(ids).await.unwrap_or_default();
        let mut presence = if with_presence { self.presences(cookie, ids).await.unwrap_or_default() } else { HashMap::new() };
        Ok(ids
            .iter()
            .filter_map(|id| {
                let (name, display_name) = names.get(id)?.clone();
                Some(Person {
                    id: *id,
                    name,
                    display_name,
                    avatar_url: avatars.get(id).cloned(),
                    presence: presence.remove(id).unwrap_or_else(Presence::offline),
                })
            })
            .collect())
    }

    pub async fn friends(&self, cookie: &str, user_id: u64) -> Result<Vec<Person>> {
        let url = format!("https://friends.roblox.com/v1/users/{user_id}/friends");
        let ids = parse::id_list(&self.get(&url, Some(cookie), LIST_TTL).await?)?;
        let mut people = self.people(cookie, &ids, true).await?;
        parse::sort_people(&mut people);
        Ok(people)
    }

    /// People who asked to be friends. Never cached: the list changes when the user answers.
    pub async fn friend_requests(&self, cookie: &str) -> Result<Vec<Person>> {
        let url = "https://friends.roblox.com/v1/my/friends/requests?limit=50&sortOrder=Desc";
        let ids = parse::id_list(&self.send(url, None, Some(cookie)).await?)?;
        self.people(cookie, &ids, false).await
    }

    /// Accepts or declines a friend request. Only ever called from a button the user pressed.
    pub async fn answer_friend_request(&self, cookie: &str, user_id: u64, accept: bool) -> Result<()> {
        let action = if accept { "accept-friend-request" } else { "decline-friend-request" };
        let url = format!("https://friends.roblox.com/v1/users/{user_id}/{action}");
        self.send(&url, Some(&serde_json::json!({})), Some(cookie)).await?;
        // The friends list just changed.
        self.cache.lock().unwrap().retain(|key, _| !key.contains("/friends"));
        Ok(())
    }

    /// Pictures for items by id. `bundles` switches to the bundle picture endpoint.
    async fn item_pictures(&self, ids: &[u64], bundles: bool) -> HashMap<u64, String> {
        let mut all = HashMap::new();
        for batch in ids.chunks(ICON_BATCH) {
            let url = if bundles {
                format!("https://thumbnails.roblox.com/v1/bundles/thumbnails?bundleIds={}&size=150x150&format=Png", join_ids(batch))
            } else {
                format!("https://thumbnails.roblox.com/v1/assets?assetIds={}&size=150x150&format=Png", join_ids(batch))
            };
            if let Ok(body) = self.get(&url, None, THUMB_TTL).await {
                all.extend(parse::thumbnails(&body).unwrap_or_default());
            }
        }
        all
    }

    async fn with_pictures(&self, mut items: Vec<Item>) -> Vec<Item> {
        let assets: Vec<u64> = items.iter().filter(|i| i.kind != "Bundle").map(|i| i.id).collect();
        let bundles: Vec<u64> = items.iter().filter(|i| i.kind == "Bundle").map(|i| i.id).collect();
        let asset_pictures = self.item_pictures(&assets, false).await;
        let bundle_pictures = if bundles.is_empty() { HashMap::new() } else { self.item_pictures(&bundles, true).await };
        for item in &mut items {
            let from = if item.kind == "Bundle" { &bundle_pictures } else { &asset_pictures };
            item.image_url = from.get(&item.id).cloned();
        }
        items
    }

    /// What the account is wearing right now (with pictures) and the raw avatar.
    pub async fn wearing(&self, cookie: &str) -> Result<(Vec<Item>, serde_json::Value)> {
        let body = self.send("https://avatar.roblox.com/v2/avatar/avatar", None, Some(cookie)).await?;
        let (items, raw) = parse::current_avatar(&body)?;
        Ok((self.with_pictures(items).await, raw))
    }

    /// One page of the account's own items of the given Roblox types.
    pub async fn inventory(&self, cookie: &str, user_id: u64, types: &str, cursor: Option<&str>) -> Result<(Vec<Item>, Option<String>)> {
        let mut url = Url::parse(&format!("https://inventory.roblox.com/v2/users/{user_id}/inventory")).map_err(|_| AppError::BlockedUrl)?;
        url.query_pairs_mut().append_pair("assetTypes", types).append_pair("limit", "50").append_pair("sortOrder", "Desc");
        if let Some(c) = cursor {
            url.query_pairs_mut().append_pair("cursor", c);
        }
        let (items, next) = parse::inventory(&self.get(url.as_str(), Some(cookie), LIST_TTL).await?)?;
        Ok((self.with_pictures(items).await, next))
    }

    /// Wears exactly `ids`. Returns the ids Roblox refused. Only ever called from a button the user pressed.
    pub async fn set_wearing(&self, cookie: &str, ids: &[u64]) -> Result<Vec<u64>> {
        let now = self.send("https://avatar.roblox.com/v2/avatar/avatar", None, Some(cookie)).await?;
        let (worn, _) = parse::current_avatar(&now)?;
        let body = parse::wearing_body(ids, &worn);
        let answer = self.send("https://avatar.roblox.com/v2/avatar/set-wearing-assets", Some(&body), Some(cookie)).await?;
        Ok(parse::refused_ids(&answer))
    }

    /// Replaces the worn accessories with exactly `accessory_ids` (Roblox's "Advanced" box), keeping
    /// clothes and body parts. Returns the ids Roblox refused.
    pub async fn set_accessories(&self, cookie: &str, accessory_ids: &[u64]) -> Result<Vec<u64>> {
        let now = self.send("https://avatar.roblox.com/v2/avatar/avatar", None, Some(cookie)).await?;
        let (worn, _) = parse::current_avatar(&now)?;
        let ids = parse::with_accessories(&worn, accessory_ids);
        let body = parse::wearing_body(&ids, &worn);
        let answer = self.send("https://avatar.roblox.com/v2/avatar/set-wearing-assets", Some(&body), Some(cookie)).await?;
        Ok(parse::refused_ids(&answer))
    }

    /// Saves the current look as a new outfit. Only ever called from a button the user pressed.
    pub async fn save_outfit(&self, cookie: &str, name: &str) -> Result<()> {
        let now = self.send("https://avatar.roblox.com/v2/avatar/avatar", None, Some(cookie)).await?;
        let (_, raw) = parse::current_avatar(&now)?;
        self.send("https://avatar.roblox.com/v3/outfits/create", Some(&parse::outfit_body(name, &raw)), Some(cookie)).await.map(|_| ())
    }

    /// Searches the avatar catalog. Public, no session needed.
    pub async fn catalog(&self, keyword: &str, category: u32, sort: u32, cursor: Option<&str>) -> Result<(Vec<Item>, Option<String>)> {
        let mut url = Url::parse("https://catalog.roblox.com/v1/search/items/details").map_err(|_| AppError::BlockedUrl)?;
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("Category", &category.to_string()).append_pair("SortType", &sort.to_string()).append_pair("Limit", "30");
            if !keyword.trim().is_empty() {
                q.append_pair("Keyword", keyword.trim());
            }
            if let Some(c) = cursor {
                q.append_pair("Cursor", c);
            }
        }
        let (items, next) = parse::catalog(&self.get(url.as_str(), None, CHARTS_TTL).await?)?;
        Ok((self.with_pictures(items).await, next))
    }

    /// The account's Robux balance.
    pub async fn robux(&self, cookie: &str, user_id: u64) -> Result<u64> {
        let body = self.send(&format!("https://economy.roblox.com/v1/users/{user_id}/currency"), None, Some(cookie)).await?;
        serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("robux")?.as_u64())
            .ok_or_else(|| AppError::Api("balance".into()))
    }

    async fn sale_details(&self, asset_id: u64) -> Result<String> {
        // Never cached: the price shown and the price paid must be the current one.
        self.send(&format!("https://economy.roblox.com/v2/assets/{asset_id}/details"), None, None).await
    }

    /// What buying this item would involve for this account. Changes nothing.
    pub async fn offer(&self, cookie: &str, user_id: u64, asset_id: u64) -> Result<Offer> {
        let details = self.sale_details(asset_id).await?;
        let owned_url = format!("https://inventory.roblox.com/v1/users/{user_id}/items/Asset/{asset_id}/is-owned");
        let owned = self.send(&owned_url, None, Some(cookie)).await.is_ok_and(|b| b.trim() == "true");
        let balance = self.robux(cookie, user_id).await.ok();
        parse::offer(&details, owned, balance)
    }

    /// Buys an item for exactly `agreed_price` Robux. Spends the account's Robux.
    /// Only ever called after the user confirmed that item and that price in a dialog.
    pub async fn buy(&self, cookie: &str, user_id: u64, asset_id: u64, agreed_price: u64, idempotency_key: &str) -> Result<()> {
        let details = self.sale_details(asset_id).await?;
        let (url, body) = parse::purchase_request(&details, agreed_price, user_id, idempotency_key)?;
        let answer = self.send(&url, Some(&body), Some(cookie)).await?;
        parse::purchase_outcome(&answer)
    }

    /// Whether this account favorited and how it rated a game, plus the game's public vote counts.
    pub async fn game_standing(&self, cookie: &str, universe_id: u64) -> Result<GameStanding> {
        let base = format!("https://games.roblox.com/v1/games/{universe_id}");
        let favorite = self.send(&format!("{base}/favorites"), None, Some(cookie)).await?;
        let mine = self.send(&format!("{base}/votes/user"), None, Some(cookie)).await?;
        // Totals are public and only decoration; show the buttons even without them.
        let totals = self.get(&format!("{base}/votes"), None, LIST_TTL).await.ok();
        parse::game_standing(&favorite, &mine, totals.as_deref())
    }

    /// Adds or removes a game from the account's favorites. Only ever called from a button the user pressed.
    pub async fn set_favorite(&self, cookie: &str, universe_id: u64, favorited: bool) -> Result<()> {
        let url = format!("https://games.roblox.com/v1/games/{universe_id}/favorites");
        self.send(&url, Some(&serde_json::json!({ "isFavorited": favorited })), Some(cookie)).await?;
        // Home's favorites row just changed.
        self.cache.lock().unwrap().retain(|key, _| !key.contains("/favorite/games"));
        Ok(())
    }

    /// Likes (`true`) or dislikes (`false`) a game. Only ever called from a button the user pressed.
    pub async fn set_vote(&self, cookie: &str, universe_id: u64, like: bool) -> Result<()> {
        let url = format!("https://games.roblox.com/v1/games/{universe_id}/user-votes");
        self.request(true, &url, Some(&serde_json::json!({ "vote": like })), Some(cookie)).await?;
        self.cache.lock().unwrap().retain(|key, _| !key.ends_with(&format!("/games/{universe_id}/votes")));
        Ok(())
    }

    /// The account's saved outfits, with pictures.
    pub async fn outfits(&self, cookie: &str, user_id: u64) -> Result<Vec<Outfit>> {
        let url = format!("https://avatar.roblox.com/v2/avatar/users/{user_id}/outfits?page=1&itemsPerPage=50&isEditable=true");
        let mut list = parse::outfits(&self.send(&url, None, Some(cookie)).await?)?;
        let ids: Vec<u64> = list.iter().map(|o| o.id).collect();
        let mut pictures = HashMap::new();
        for batch in ids.chunks(ICON_BATCH) {
            let url = format!(
                "https://thumbnails.roblox.com/v1/users/outfits?userOutfitIds={}&size=150x150&format=Png&isCircular=false",
                join_ids(batch)
            );
            // Pictures are extras; the list is still usable by name. Not cached: a new outfit's
            // picture takes Roblox a few seconds to draw, and a cached "not ready" would hide it for minutes.
            if let Ok(body) = self.send(&url, None, None).await {
                pictures.extend(parse::thumbnails(&body).unwrap_or_default());
            }
        }
        for o in &mut list {
            o.image_url = pictures.get(&o.id).cloned();
        }
        Ok(list)
    }

    /// The account's current full-body picture. Never cached, so a change shows as soon as Roblox has redrawn it.
    pub async fn avatar_picture(&self, user_id: u64) -> Result<Option<String>> {
        let url = format!("https://thumbnails.roblox.com/v1/users/avatar?userIds={user_id}&size=420x420&format=Png&isCircular=false");
        Ok(parse::thumbnails(&self.send(&url, None, None).await?)?.remove(&user_id))
    }

    /// Puts a saved outfit on. Returns the parts Roblox refused, if any (empty means everything applied).
    /// Only ever called from a button the user pressed.
    pub async fn wear_outfit(&self, cookie: &str, outfit_id: u64) -> Result<Vec<String>> {
        let details = self.send(&format!("https://avatar.roblox.com/v3/outfits/{outfit_id}/details"), None, Some(cookie)).await?;
        let mut failed = Vec::new();
        for (what, path, body) in parse::wear_requests(&details)? {
            match self.send(&format!("https://avatar.roblox.com{path}"), Some(&body), Some(cookie)).await {
                Ok(_) => {}
                // Without the items nothing useful happened: report that as the error.
                Err(e) if what == "items" => return Err(e),
                Err(AppError::SessionExpired) => return Err(AppError::SessionExpired),
                Err(_) => failed.push(what.to_owned()),
            }
        }
        Ok(failed)
    }

    /// Public servers of a place that still have room, emptiest first. Not cached: counts change by the second.
    pub async fn public_servers(&self, place_id: u64) -> Result<Vec<Server>> {
        let url = format!("https://games.roblox.com/v1/games/{place_id}/servers/Public?sortOrder=Asc&limit=100&excludeFullGames=true");
        parse::public_servers(&self.send(&url, None, None).await?)
    }

    /// Private servers of a place that this account can join.
    pub async fn private_servers(&self, cookie: &str, place_id: u64) -> Result<Vec<PrivateServer>> {
        let url = format!("https://games.roblox.com/v1/games/{place_id}/private-servers?limit=50&sortOrder=Desc");
        parse::private_servers(&self.send(&url, None, Some(cookie)).await?)
    }

    /// The game a presence points at, with its name and icon, for showing a join as a running game.
    pub async fn game_for(&self, universe_id: u64, place_id: u64, fallback_name: &str) -> Game {
        let url = format!("https://games.roblox.com/v1/games?universeIds={universe_id}");
        let found = match self.get(&url, None, LIST_TTL).await {
            Ok(body) => parse::games(&body).ok().and_then(|mut g| g.pop()),
            Err(_) => None,
        };
        let game = found.unwrap_or(Game { universe_id, place_id, name: fallback_name.to_owned(), playing: None, icon_url: None });
        self.with_icons(vec![game]).await.remove(0)
    }

    async fn icons(&self, ids: &[u64]) -> Result<HashMap<u64, String>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut all = HashMap::new();
        for batch in ids.chunks(ICON_BATCH) {
            let url = format!(
                "https://thumbnails.roblox.com/v1/games/icons?universeIds={}&size=256x256&format=Png&isCircular=false",
                join_ids(batch)
            );
            all.extend(parse::thumbnails(&self.get(&url, None, THUMB_TTL).await?)?);
        }
        Ok(all)
    }

    async fn with_icons(&self, mut games: Vec<Game>) -> Vec<Game> {
        let ids: Vec<u64> = games.iter().map(|g| g.universe_id).collect();
        // A game without a picture is still worth listing.
        let icons = self.icons(&ids).await.unwrap_or_default();
        for g in &mut games {
            g.icon_url = icons.get(&g.universe_id).cloned();
        }
        games
    }

    /// Searches every experience on Roblox. Public, no session needed.
    pub async fn search(&self, query: &str) -> Result<Vec<Game>> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(vec![]);
        }
        let url = Url::parse_with_params(
            "https://apis.roblox.com/search-api/omni-search",
            [("searchQuery", query), ("sessionId", "vodka-launcher"), ("pageType", "all")],
        )
        .map_err(|_| AppError::BlockedUrl)?;
        let games = parse::search_results(&self.get(url.as_str(), None, LIST_TTL).await?)?;
        Ok(self.with_icons(games).await)
    }

    /// The public charts: trending, most played and so on.
    pub async fn charts(&self) -> Result<Vec<Chart>> {
        let url = "https://apis.roblox.com/explore-api/v1/get-sorts?sessionId=vodka-launcher&device=computer&country=all";
        let mut charts = parse::charts(&self.get(url, None, CHARTS_TTL).await?, GAMES_PER_CHART)?;
        let all: Vec<Game> = charts.iter().flat_map(|c| c.games.clone()).collect();
        let icons: HashMap<u64, Option<String>> = self.with_icons(all).await.into_iter().map(|g| (g.universe_id, g.icon_url)).collect();
        for g in charts.iter_mut().flat_map(|c| c.games.iter_mut()) {
            g.icon_url = icons.get(&g.universe_id).cloned().flatten();
        }
        Ok(charts)
    }

    pub async fn favorites(&self, cookie: &str, user_id: u64) -> Result<Vec<Game>> {
        let url = format!("https://games.roblox.com/v2/users/{user_id}/favorite/games?limit=50&sortOrder=Desc");
        let games = parse::favorites(&self.get(&url, Some(cookie), LIST_TTL).await?)?;
        let ids: Vec<u64> = games.iter().map(|g| g.universe_id).collect();
        let icons = self.icons(&ids).await?;
        Ok(parse::assemble(&ids, games, &icons))
    }

    pub async fn continue_playing(&self, cookie: &str) -> Result<Vec<Game>> {
        let request = serde_json::json!({ "pageType": "Home", "sessionId": "vodka-launcher" });
        let body = self
            .send("https://apis.roblox.com/discovery-api/omni-recommendation", Some(&request), Some(cookie))
            .await?;
        let mut ids = parse::continue_universe_ids(&body)?;
        ids.truncate(18);
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let url = format!("https://games.roblox.com/v1/games?universeIds={}", join_ids(&ids));
        let games = parse::games(&self.get(&url, None, LIST_TTL).await?)?;
        let icons = self.icons(&ids).await?;
        Ok(parse::assemble(&ids, games, &icons))
    }

    /// Ends the session on Roblox's side. Best effort.
    pub async fn logout(&self, cookie: &str) -> Result<()> {
        let empty = serde_json::json!({});
        self.send("https://auth.roblox.com/v2/logout", Some(&empty), Some(cookie)).await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed(u: &str) -> bool {
        check_url(&Url::parse(u).unwrap()).is_ok()
    }

    #[test]
    fn allows_only_https_roblox_hosts() {
        assert!(allowed("https://users.roblox.com/v1/users/authenticated"));
        assert!(allowed("https://roblox.com/"));
        assert!(!allowed("http://users.roblox.com/"));
        assert!(!allowed("https://roblox.com.evil.example/"));
        assert!(!allowed("https://evilroblox.com/"));
        assert!(!allowed("https://user:pw@www.roblox.com/"));
        assert!(!allowed("https://www.roblox.com:8443/"));
        assert!(!allowed("https://tr.rbxcdn.com/x"));
    }

    // These two talk to the real Roblox API. Run them by hand: cargo test live_ -- --ignored
    #[tokio::test]
    #[ignore]
    async fn live_search_finds_games_with_icons() {
        let games = Client::new().search("tower").await.unwrap();
        assert!(games.len() > 5);
        assert!(games.iter().filter(|g| g.icon_url.is_some()).count() > 3);
    }

    #[tokio::test]
    #[ignore]
    async fn live_public_servers_come_emptiest_first() {
        let servers = Client::new().public_servers(17625359962).await.unwrap();
        assert!(!servers.is_empty());
        assert!(servers.windows(2).all(|w| w[0].playing <= w[1].playing));
        assert!(servers.iter().all(|s| s.max_players > 0 && s.playing < s.max_players));
    }

    #[tokio::test]
    #[ignore]
    async fn live_catalog_search_finds_items_with_pictures() {
        let (items, next) = Client::new().catalog("cat", 11, 0, None).await.unwrap();
        assert!(items.len() > 5);
        assert!(next.is_some());
        assert!(items.iter().filter(|i| i.image_url.is_some()).count() > 3);
    }

    #[tokio::test]
    #[ignore]
    async fn live_charts_have_rows_of_games() {
        let charts = Client::new().charts().await.unwrap();
        assert!(charts.len() >= 3);
        assert!(charts.iter().all(|c| !c.games.is_empty() && c.games.len() <= GAMES_PER_CHART));
        assert!(charts[0].games.iter().any(|g| g.icon_url.is_some()));
    }

    /// Needs a real session: VODKA_TEST_COOKIE and VODKA_TEST_USER_ID. Prints counts only.
    #[tokio::test]
    #[ignore]
    async fn live_friends_have_names_and_presence() {
        let cookie = std::env::var("VODKA_TEST_COOKIE").expect("VODKA_TEST_COOKIE");
        let id: u64 = std::env::var("VODKA_TEST_USER_ID").expect("VODKA_TEST_USER_ID").parse().unwrap();
        let client = Client::new();
        let friends = client.friends(&cookie, id).await.unwrap();
        assert!(friends.iter().all(|f| !f.name.is_empty() && !f.display_name.is_empty()));
        let with_avatar = friends.iter().filter(|f| f.avatar_url.is_some()).count();
        let online = friends.iter().filter(|f| f.presence.kind != "offline").count();
        let joinable = friends.iter().filter(|f| f.presence.join_target().is_some()).count();
        println!("friends={} with_avatar={with_avatar} not_offline={online} joinable={joinable}", friends.len());
        let requests = client.friend_requests(&cookie).await.unwrap();
        println!("requests={}", requests.len());
    }

    #[test]
    fn backoff_grows_and_is_capped() {
        assert_eq!(backoff(0, None), Duration::from_millis(500));
        assert_eq!(backoff(2, None), Duration::from_secs(2));
        assert_eq!(backoff(0, Some(3)), Duration::from_secs(3));
        assert_eq!(backoff(0, Some(600)), Duration::from_secs(5));
    }
}
