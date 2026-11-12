//! Typed Roblox web API client: allowlisted hosts, short cache, 429 backoff.

pub mod parse;

use crate::error::{AppError, Result};
pub use parse::{Chart, Game, Person, Presence, User};
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
        let parsed = Url::parse(url).map_err(|_| AppError::BlockedUrl)?;
        check_url(&parsed)?;
        let mut csrf: Option<String> = None;
        let mut attempt = 0;
        loop {
            let mut req = match body {
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
