//! Luduvo, the second game next to Roblox.
//!
//! Vodka drives the official Luduvo launcher (installed by luduvo.com's own script) the way the website does:
//! it asks api.luduvo.com for a join ticket and hands the launcher the resulting `luduvo://` link. Sign-in uses
//! Luduvo's own Studio flow: the site sends a one-time ticket to a listener on 127.0.0.1, and Vodka redeems it.
//! The refresh token lives in the system keyring; the short-lived access token only in memory.

pub mod parse;

use crate::error::{AppError, Result};
pub use parse::{Friend, Group, OwnedItem, Outfit, Profile, Place, PlaceDetails, Rating, Server, Shelf, StoreCategory, StoreItem, User};
use reqwest::{Method, StatusCode, Url, header};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const API: &str = "https://api.luduvo.com";
const SITE: &str = "https://luduvo.com";
const SERVICE: &str = "vodka-launcher";
const SESSION_ENTRY: &str = "luduvo-session";
const RETRIES: u32 = 3;
/// How long Vodka waits for the browser to finish signing in.
const SIGN_IN_WAIT: Duration = Duration::from_secs(300);
/// The command luduvo.com/download shows for Linux.
pub const INSTALL_COMMAND: &str = "curl -fsSL https://assets.luduvo.com/install.sh | sh";

/// Only https://api.luduvo.com may be called, and never with a port or credentials in the URL.
pub fn check_url(url: &Url) -> Result<()> {
    let ok = url.scheme() == "https"
        && url.host_str() == Some("api.luduvo.com")
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none();
    if ok { Ok(()) } else { Err(AppError::BlockedUrl) }
}

// ---------------------------------------------------------------------------------------------
// The installed launcher

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub installed: bool,
    /// Luduvo's own build number, from the launcher's state file.
    pub version: Option<String>,
    /// False where Vodka can't start Luduvo yet (Windows).
    pub supported: bool,
    pub install_command: &'static str,
}

fn install_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(dir).join("Luduvo"));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share/Luduvo"))
}

/// The launcher binary, if the official install is there.
pub fn launcher() -> Option<PathBuf> {
    if cfg!(windows) {
        return None;
    }
    let exe = install_dir()?.join("Luduvo");
    exe.is_file().then_some(exe)
}

pub fn detect() -> Status {
    let exe = launcher();
    let version = install_dir()
        .and_then(|d| std::fs::read_to_string(d.join("state.json")).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("version").and_then(|v| v.as_str()).map(str::to_owned));
    Status { installed: exe.is_some(), version, supported: !cfg!(windows), install_command: INSTALL_COMMAND }
}

/// The Luduvo game Vodka started and is watching.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Running {
    pub place_id: u64,
    pub title: String,
    pub pid: u32,
    /// Unix seconds.
    pub started_at: u64,
    /// `checking`, `updating`, `content` (downloading game content), `starting` or `playing`.
    pub stage: String,
    /// Content downloaded so far, out of `total`, while `stage` is `content`.
    pub done: Option<u64>,
    pub total: Option<u64>,
}

/// Whether the launcher Vodka started has turned into the game itself (it replaces itself with `LuduvoGame`).
pub fn is_game_process(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/comm")).is_ok_and(|c| c.trim() == "LuduvoGame")
}

/// What Luduvo's launcher log says about a start that began at `since_ms` (Unix milliseconds).
pub fn launch_step(since_ms: u64) -> Option<parse::LaunchStep> {
    let log = std::fs::read_to_string(install_dir()?.join("launcher.log")).ok()?;
    parse::launch_step(&log, since_ms)
}

/// Starts the launcher with `arg` (a join link) or, with `None`, opens Luduvo Studio.
/// The launcher updates itself, then replaces itself (same process) with the game or the editor,
/// so the returned child is the game for as long as it runs.
pub fn spawn(arg: Option<&str>) -> Result<tokio::process::Child> {
    let exe = launcher().ok_or_else(|| AppError::Other("Luduvo isn't installed. Install it with the command on the Luduvo page.".into()))?;
    let mut cmd = tokio::process::Command::new(exe);
    if let Some(arg) = arg {
        cmd.arg(arg);
    }
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| AppError::Other(format!("Luduvo could not be started: {e}")))
}

/// Opens Luduvo Studio and forgets about it; a background task reaps it when it closes.
pub fn open_studio() -> Result<()> {
    let mut child = spawn(None)?;
    tauri::async_runtime::spawn(async move {
        let _ = child.wait().await;
    });
    Ok(())
}

/// Asks a game Vodka started to close, the way closing its window would.
pub fn stop(pid: u32) -> Result<()> {
    let ok = std::process::Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok { Ok(()) } else { Err(AppError::Other("Vodka couldn't close the Luduvo game. Close its window instead.".into())) }
}

// ---------------------------------------------------------------------------------------------
// Join links

/// Encodes like JavaScript's `encodeURIComponent`, which is what luduvo.com uses for these links.
fn encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `luduvo://<token>/<address>/<port>/<public_key>[/<purchase_token>]`, built exactly as the website builds it.
pub fn join_link(answer: &serde_json::Value) -> Result<String> {
    let text = |k: &str| answer.get(k).and_then(|v| v.as_str()).filter(|s| !s.is_empty());
    let port = answer.get("port").and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())));
    let (Some(token), Some(address), Some(port), Some(key)) = (text("token"), text("address"), port, text("public_key")) else {
        return Err(AppError::Other("Luduvo didn't send everything needed to join that game. Try again.".into()));
    };
    if !(1..=65535).contains(&port) {
        return Err(AppError::Other("Luduvo sent a server port Vodka can't use.".into()));
    }
    let mut link = format!("luduvo://{}/{}/{}/{}", encode_component(token), encode_component(address), port, encode_component(key));
    if let Some(purchase) = text("purchase_token") {
        link.push('/');
        link.push_str(&encode_component(purchase));
    }
    Ok(link)
}

// ---------------------------------------------------------------------------------------------
// Session in the keyring

#[derive(serde::Serialize, serde::Deserialize)]
struct Saved {
    refresh_token: String,
}

fn entry() -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, SESSION_ENTRY).map_err(|e| AppError::Keyring(e.to_string()))
}

/// A token we are willing to store and put in a header: printable ASCII, no spaces.
fn valid_token(t: &str) -> bool {
    (16..=8192).contains(&t.len()) && t.bytes().all(|b| (0x21..0x7f).contains(&b))
}

fn load_refresh() -> Result<Option<String>> {
    match entry()?.get_password() {
        Ok(json) => Ok(serde_json::from_str::<Saved>(&json).ok().map(|s| s.refresh_token).filter(|t| valid_token(t))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Keyring(e.to_string())),
    }
}

fn store_refresh(token: &str) -> Result<()> {
    if !valid_token(token) {
        return Err(AppError::Other("Luduvo returned a session Vodka can't store safely.".into()));
    }
    let json = serde_json::to_string(&Saved { refresh_token: token.to_owned() }).expect("plain struct");
    entry()?.set_password(&json).map_err(|e| AppError::Keyring(e.to_string()))
}

fn clear_refresh() -> Result<()> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Keyring(e.to_string())),
    }
}

// ---------------------------------------------------------------------------------------------
// API client

fn not_signed_in() -> AppError {
    AppError::Other("You're not signed in to Luduvo.".into())
}

/// Turns Luduvo's error codes into sentences, like the website does.
fn explain(status: StatusCode, body: &str) -> AppError {
    let json: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    let code = json.get("code").or_else(|| json.get("error")).and_then(|v| v.as_str()).unwrap_or_default();
    let known = match code {
        "join_flood" => Some("You're joining games too quickly. Wait a few minutes and try again."),
        "paid_access_required" => Some("This game needs paid access. Buy it on luduvo.com first."),
        _ => None,
    };
    if let Some(text) = known {
        return AppError::Other(text.into());
    }
    if status == StatusCode::PAYMENT_REQUIRED {
        return AppError::Other("This game needs paid access. Buy it on luduvo.com first.".into());
    }
    // Luduvo's own messages are short sentences meant for players; show them, but never a whole page of HTML.
    let message = json.get("message").or_else(|| json.get("error")).and_then(|v| v.as_str()).filter(|m| m.contains(' ') && m.len() < 200);
    match message {
        Some(m) => AppError::Other(format!("Luduvo: {m}")),
        None => AppError::Other(format!("Luduvo sent a response Vodka didn't understand (HTTP {}).", status.as_u16())),
    }
}

pub struct Client {
    http: reqwest::Client,
    access: Mutex<Option<String>>,
    /// Only one refresh at a time: Luduvo rotates the refresh token on every use.
    refreshing: tokio::sync::Mutex<()>,
    /// Lets a waiting sign-in be cancelled from the UI.
    cancel: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    /// The game Vodka started, while it runs.
    pub running: Mutex<Option<Running>>,
}

impl Client {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("VodkaLauncher/", env!("CARGO_PKG_VERSION")))
            // Redirects are never followed, so a token can't be carried elsewhere.
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .build()
            .expect("HTTP client");
        Self { http, access: Mutex::new(None), refreshing: tokio::sync::Mutex::new(()), cancel: Mutex::new(None), running: Mutex::new(None) }
    }

    fn current(&self) -> Option<String> {
        self.access.lock().unwrap().clone()
    }

    /// One call with retries on 429 and 5xx. `token` adds the Bearer header.
    async fn send(&self, method: Method, path: &str, body: Option<&serde_json::Value>, token: Option<&str>) -> Result<(StatusCode, String)> {
        let url = Url::parse(&format!("{API}{path}")).map_err(|_| AppError::BlockedUrl)?;
        check_url(&url)?;
        let mut attempt = 0;
        loop {
            let mut req = self.http.request(method.clone(), url.clone()).header(header::ACCEPT, "application/json");
            if let Some(json) = body {
                req = req.json(json);
            } else if method == Method::POST {
                req = req.header(header::CONTENT_LENGTH, "0");
            }
            if let Some(t) = token {
                req = req.bearer_auth(t);
            }
            let res = req.send().await.map_err(|e| {
                if e.is_timeout() || e.is_connect() {
                    AppError::Other("Can't reach Luduvo. Check your internet connection.".into())
                } else {
                    AppError::Other(format!("Couldn't talk to Luduvo: {e}"))
                }
            })?;
            let status = res.status();
            let retry_after = res.headers().get(header::RETRY_AFTER).and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok());
            let text = res.text().await.unwrap_or_default();
            let again = status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
            // A join is never repeated by itself: a second ticket could put the player in twice.
            if again && attempt + 1 < RETRIES && !path.ends_with("/join") {
                tokio::time::sleep(crate::roblox_api::backoff(attempt, retry_after)).await;
                attempt += 1;
                continue;
            }
            if status.is_server_error() {
                return Err(AppError::Other(format!("Luduvo's servers are having trouble right now (HTTP {}). Try again in a bit.", status.as_u16())));
            }
            return Ok((status, text));
        }
    }

    async fn public(&self, path: &str) -> Result<serde_json::Value> {
        let (status, text) = self.send(Method::GET, path, None, None).await?;
        if !status.is_success() {
            return Err(explain(status, &text));
        }
        serde_json::from_str(&text).map_err(|_| AppError::Other("Luduvo sent a response Vodka didn't understand.".into()))
    }

    /// Swaps the stored refresh token for a fresh pair. Clears the session when Luduvo refuses it.
    async fn refresh(&self, stale: Option<&str>) -> Result<String> {
        let _guard = self.refreshing.lock().await;
        // Someone else refreshed while we waited.
        let current = self.current();
        if let Some(current) = current
            && Some(current.as_str()) != stale
        {
            return Ok(current);
        }
        let refresh = tokio::task::spawn_blocking(load_refresh).await.map_err(|e| AppError::Other(e.to_string()))??.ok_or_else(not_signed_in)?;
        let (status, text) = self.send(Method::POST, "/auth/refresh", Some(&serde_json::json!({ "refresh_token": refresh })), None).await?;
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            *self.access.lock().unwrap() = None;
            let _ = tokio::task::spawn_blocking(clear_refresh).await;
            return Err(AppError::Other("Your Luduvo session has expired. Sign in again.".into()));
        }
        if !status.is_success() {
            return Err(explain(status, &text));
        }
        self.keep_tokens(&text).await
    }

    /// Stores a `{token, refresh_token}` answer and returns the access token.
    async fn keep_tokens(&self, text: &str) -> Result<String> {
        let pair = parse::tokens(text).ok_or_else(|| AppError::Other("Luduvo sent a session Vodka didn't understand.".into()))?;
        if !valid_token(&pair.0) {
            return Err(AppError::Other("Luduvo returned a session Vodka can't use safely.".into()));
        }
        let refresh = pair.1.clone();
        tokio::task::spawn_blocking(move || store_refresh(&refresh)).await.map_err(|e| AppError::Other(e.to_string()))??;
        *self.access.lock().unwrap() = Some(pair.0.clone());
        Ok(pair.0)
    }

    /// A signed-in call. Refreshes the access token once when Luduvo says it has expired.
    async fn authed(&self, method: Method, path: &str) -> Result<serde_json::Value> {
        self.authed_with(method, path, None).await
    }

    async fn authed_with(&self, method: Method, path: &str, body: Option<&serde_json::Value>) -> Result<serde_json::Value> {
        let cached = self.current();
        let mut token = match cached {
            Some(t) => t,
            None => self.refresh(None).await?,
        };
        for attempt in 0..2 {
            let (status, text) = self.send(method.clone(), path, body, Some(&token)).await?;
            if status == StatusCode::UNAUTHORIZED && attempt == 0 {
                token = self.refresh(Some(&token)).await?;
                continue;
            }
            if status == StatusCode::UNAUTHORIZED {
                return Err(AppError::Other("Your Luduvo session has expired. Sign in again.".into()));
            }
            if !status.is_success() {
                return Err(explain(status, &text));
            }
            // Some actions (accept, decline) answer with no body at all.
            if text.trim().is_empty() {
                return Ok(serde_json::Value::Null);
            }
            return serde_json::from_str(&text).map_err(|_| AppError::Other("Luduvo sent a response Vodka didn't understand.".into()));
        }
        unreachable!("the loop returns on its second pass")
    }

    /// Whether a session is saved, without contacting Luduvo.
    pub async fn has_session(&self) -> Result<bool> {
        if self.current().is_some() {
            return Ok(true);
        }
        Ok(tokio::task::spawn_blocking(load_refresh).await.map_err(|e| AppError::Other(e.to_string()))??.is_some())
    }

    pub async fn me(&self) -> Result<User> {
        let json = self.authed(Method::GET, "/me").await?;
        parse::user(&json).ok_or_else(|| AppError::Other("Luduvo sent an account Vodka didn't understand.".into()))
    }

    pub async fn discover(&self) -> Result<Vec<Shelf>> {
        Ok(parse::shelves(&self.public("/places/discovery?surface=discover&limit=36").await?))
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Place>> {
        let q: String = query.trim().chars().take(200).collect();
        let mut url = Url::parse(&format!("{API}/places")).expect("static url");
        {
            let mut pairs = url.query_pairs_mut();
            if !q.is_empty() {
                pairs.append_pair("q", &q);
            }
            pairs.append_pair("limit", "40").append_pair("offset", "0");
        }
        let path = format!("/places?{}", url.query().unwrap_or_default());
        Ok(parse::place_list(&self.public(&path).await?))
    }

    pub async fn recently_played(&self) -> Result<Vec<Place>> {
        Ok(parse::place_list(&self.authed(Method::GET, "/me/recently-played?limit=30&offset=0").await?))
    }

    /// Asks Luduvo for a seat in `place_id` and returns the link the launcher joins with.
    pub async fn join(&self, place_id: u64) -> Result<String> {
        let answer = self.authed(Method::POST, &format!("/places/{place_id}/join")).await?;
        join_link(&answer)
    }

    /// Everything the game page shows about one game. Public.
    pub async fn details(&self, place_id: u64) -> Result<PlaceDetails> {
        parse::details(&self.public(&format!("/places/{place_id}")).await?).ok_or_else(|| AppError::Other("Luduvo doesn't know that game.".into()))
    }

    /// Likes, dislikes and, when signed in, your own vote.
    pub async fn rating(&self, place_id: u64) -> Result<Rating> {
        let path = format!("/places/{place_id}/rating");
        let json = if self.has_session().await? { self.authed(Method::GET, &path).await? } else { self.public(&path).await? };
        Ok(parse::rating(&json))
    }

    /// Votes 1 (like), -1 (dislike) or 0 (take it back), then reads the new totals.
    pub async fn rate(&self, place_id: u64, value: i64) -> Result<Rating> {
        self.authed_with(Method::POST, &format!("/places/{place_id}/rate"), Some(&serde_json::json!({ "value": value.signum() }))).await?;
        self.rating(place_id).await
    }

    /// A seat in a private server, from its code (or a pasted link ending in one).
    pub async fn join_private(&self, pasted: &str) -> Result<String> {
        let code = parse::private_code(pasted).ok_or_else(|| AppError::Other("That doesn't look like a Luduvo private server code.".into()))?;
        let answer = self.authed(Method::POST, &format!("/private-servers/{code}/join")).await?;
        join_link(&answer)
    }

    /// Joins the group that owns a game.
    pub async fn join_group(&self, group_id: u64) -> Result<()> {
        self.authed(Method::POST, &format!("/groups/{group_id}/join")).await?;
        Ok(())
    }

    /// The game's running servers and who is in each. Public.
    pub async fn servers(&self, place_id: u64) -> Result<Vec<Server>> {
        Ok(parse::servers(&self.public(&format!("/places/{place_id}/servers")).await?))
    }

    /// A seat in one particular server.
    pub async fn join_server(&self, server_id: &str) -> Result<String> {
        if !parse::valid_server_id(server_id) {
            return Err(AppError::Other("That isn't a Luduvo server Vodka can join.".into()));
        }
        let answer = self.authed(Method::POST, &format!("/sessions/{server_id}/join")).await?;
        join_link(&answer)
    }

    pub async fn store_categories(&self) -> Result<Vec<StoreCategory>> {
        Ok(parse::store_categories(&self.public("/store/categories").await?))
    }

    /// One page of the store. `sort` is relevance, popular, newest or algorithm.
    pub async fn store(&self, query: &str, asset_type: &str, slot: &str, sort: &str, offset: u64) -> Result<(Vec<StoreItem>, u64)> {
        let mut url = Url::parse(&format!("{API}/store")).expect("static url");
        {
            let mut q = url.query_pairs_mut();
            let text: String = query.trim().chars().take(200).collect();
            if !text.is_empty() {
                q.append_pair("q", &text);
            }
            if !asset_type.is_empty() {
                q.append_pair("asset_type", asset_type);
            }
            if !slot.is_empty() {
                q.append_pair("slot", slot);
            }
            let sort = if ["relevance", "popular", "newest", "algorithm"].contains(&sort) { sort } else { "relevance" };
            q.append_pair("sort", sort).append_pair("limit", "50").append_pair("offset", &offset.to_string());
        }
        let path = format!("/store?{}", url.query().unwrap_or_default());
        Ok(parse::store_page(&self.public(&path).await?))
    }

    /// An item's own page; signed in, it also has your like or dislike.
    pub async fn store_item(&self, id: u64) -> Result<StoreItem> {
        let path = format!("/store/{id}");
        let json = if self.has_session().await? { self.authed(Method::GET, &path).await? } else { self.public(&path).await? };
        parse::store_item(&json).ok_or_else(|| AppError::Other("Luduvo doesn't know that item.".into()))
    }

    pub async fn rate_item(&self, id: u64, value: i64) -> Result<StoreItem> {
        self.authed_with(Method::POST, &format!("/store/{id}/rate"), Some(&serde_json::json!({ "value": value.signum() }))).await?;
        self.store_item(id).await
    }

    pub async fn favorite_item(&self, id: u64, on: bool) -> Result<()> {
        self.authed(if on { Method::POST } else { Method::DELETE }, &format!("/items/{id}/favorite")).await?;
        Ok(())
    }

    /// Buys an item at exactly `agreed_price` C$. Luduvo refuses if the price changed; if it wants a captcha, the
    /// person is sent to luduvo.com instead.
    pub async fn buy_item(&self, id: u64, agreed_price: u64) -> Result<()> {
        let current = self.store_item(id).await?;
        if current.price != Some(agreed_price) {
            return Err(AppError::Other("The price changed since you looked. Check it and confirm again.".into()));
        }
        match self.authed_with(Method::POST, &format!("/store/{id}/buy"), Some(&serde_json::json!({ "expected_price": agreed_price }))).await {
            Ok(_) => Ok(()),
            Err(AppError::Other(m)) if m.to_lowercase().contains("captcha") => {
                Err(AppError::Other("Luduvo wants a captcha for this purchase. Buy it on luduvo.com instead.".into()))
            }
            Err(e) => Err(e),
        }
    }

    /// A full-body picture of someone's avatar. Luduvo answers with a redirect to the picture on its asset host.
    pub async fn avatar_picture(&self, user_id: u64) -> Result<Option<String>> {
        let url = Url::parse(&format!("{API}/users/{user_id}/avatar/full-body")).expect("static url");
        check_url(&url)?;
        let res = self.http.get(url).send().await.map_err(|e| AppError::Other(format!("Couldn't talk to Luduvo: {e}")))?;
        let to = res.headers().get(header::LOCATION).and_then(|v| v.to_str().ok()).map(str::to_owned);
        Ok(parse::asset_url(to.map(serde_json::Value::String).as_ref()))
    }

    /// Someone's profile, with their avatar picture and what they're doing. Public.
    pub async fn profile(&self, user_id: u64) -> Result<Profile> {
        let mut p = parse::profile(&self.public(&format!("/users/{user_id}/profile")).await?).ok_or_else(|| AppError::Other("Luduvo doesn't know that person.".into()))?;
        p.picture_url = self.avatar_picture(user_id).await.ok().flatten();
        if let Ok(json) = self.public(&format!("/users/{user_id}/presence")).await {
            #[cfg(debug_assertions)]
            eprintln!("[luduvo] single presence shape: {}", parse::shape(&json));
            let pr = parse::single_presence(&json);
            p.state = pr.state;
            p.place_id = pr.place_id;
            p.place_title = pr.place_title;
            p.joinable = pr.joinable;
        }
        Ok(p)
    }

    pub async fn user_friends(&self, user_id: u64) -> Result<Vec<Friend>> {
        Ok(parse::friend_list(&self.public(&format!("/users/{user_id}/friends?limit=200&offset=0")).await?))
    }

    pub async fn user_places(&self, user_id: u64) -> Result<Vec<Place>> {
        Ok(parse::place_list(&self.public(&format!("/users/{user_id}/places?limit=50&offset=0")).await?))
    }

    pub async fn user_groups(&self, user_id: u64) -> Result<Vec<Group>> {
        let json = self.public(&format!("/users/{user_id}/groups?limit=50&offset=0")).await?;
        #[cfg(debug_assertions)]
        eprintln!("[luduvo] user groups shape: {}", parse::shape(&json));
        Ok(parse::groups(&json))
    }

    /// What you're wearing now.
    pub async fn equipped(&self) -> Result<Vec<OwnedItem>> {
        // `/me/equipped` only takes writes; the website reads your look from your profile.
        let me = self.me().await?;
        let json = self.authed(Method::GET, &format!("/users/{}/profile", me.id)).await?;
        Ok(parse::owned_list(&json))
    }

    /// Wears exactly `items` (item id and variant), replacing what you had on.
    pub async fn set_equipped(&self, items: &[(u64, u64)]) -> Result<()> {
        let list: Vec<serde_json::Value> = items.iter().map(|(id, variant)| serde_json::json!({ "item_id": id, "variant_index": variant })).collect();
        self.authed_with(Method::PUT, "/me/equipped", Some(&serde_json::json!({ "items": list }))).await?;
        Ok(())
    }

    /// One page of what you own, optionally one kind of item.
    pub async fn inventory(&self, asset_type: &str, slot: &str, offset: u64) -> Result<Vec<OwnedItem>> {
        let mut url = Url::parse(&format!("{API}/me/inventory")).expect("static url");
        {
            let mut q = url.query_pairs_mut();
            if !asset_type.is_empty() {
                q.append_pair("asset_type", asset_type);
            }
            if !slot.is_empty() {
                q.append_pair("slot", slot);
            }
            q.append_pair("limit", "100").append_pair("offset", &offset.to_string());
        }
        let json = self.authed(Method::GET, &format!("/me/inventory?{}", url.query().unwrap_or_default())).await?;
        #[cfg(debug_assertions)]
        eprintln!("[luduvo] inventory shape: {}", parse::shape(&json));
        Ok(parse::owned_list(&json))
    }

    pub async fn outfits(&self) -> Result<Vec<Outfit>> {
        Ok(parse::outfits(&self.authed(Method::GET, "/me/outfits").await?))
    }

    pub async fn wear_outfit(&self, id: u64) -> Result<()> {
        self.authed(Method::POST, &format!("/me/outfits/{id}/wear")).await?;
        Ok(())
    }

    /// Saves what you're wearing as a new outfit.
    pub async fn save_outfit(&self, name: &str) -> Result<()> {
        let name: String = name.trim().chars().take(50).collect();
        if name.is_empty() {
            return Err(AppError::Other("Give the outfit a name first.".into()));
        }
        self.authed_with(Method::POST, "/me/outfits", Some(&serde_json::json!({ "name": name }))).await?;
        Ok(())
    }

    pub async fn delete_outfit(&self, id: u64) -> Result<()> {
        self.authed(Method::DELETE, &format!("/me/outfits/{id}")).await?;
        Ok(())
    }

    /// Sends a friend request by username.
    pub async fn add_friend(&self, username: &str) -> Result<()> {
        let name = username.trim().trim_start_matches('@');
        if name.is_empty() || name.len() > 40 || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.') {
            return Err(AppError::Other("That doesn't look like a Luduvo username.".into()));
        }
        self.authed(Method::POST, &format!("/friends/by-username/{name}/request")).await?;
        Ok(())
    }

    pub async fn remove_friend(&self, user_id: u64) -> Result<()> {
        self.authed(Method::DELETE, &format!("/friends/{user_id}")).await?;
        Ok(())
    }

    /// Your friends with what each is doing, people in a game first, then online, then the rest.
    pub async fn friends(&self) -> Result<Vec<Friend>> {
        let json = self.authed(Method::GET, "/me/friends?limit=200&offset=0").await?;
        #[cfg(debug_assertions)]
        eprintln!(
            "[luduvo] friends shape: {}; hidden={:?} total={:?}",
            parse::shape(&json),
            json.get("hidden"),
            json.get("total")
        );
        let mut friends = parse::friend_list(&json);
        // The website also lists friends through the public profile route; use it when "my friends" comes back empty.
        if friends.is_empty() {
            let me = self.me().await?;
            let other = self.authed(Method::GET, &format!("/users/{}/friends?limit=200&offset=0", me.id)).await?;
            #[cfg(debug_assertions)]
            eprintln!("[luduvo] /users/me/friends shape: {}; total={:?}", parse::shape(&other), other.get("total"));
            friends = parse::friend_list(&other);
        }
        let ids: Vec<u64> = friends.iter().map(|f| f.id).collect();
        let mut presence = std::collections::HashMap::new();
        for chunk in ids.chunks(100) {
            let list = chunk.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
            let json = self.authed(Method::GET, &format!("/users/presence?ids={list}")).await?;
            #[cfg(debug_assertions)]
            eprintln!("[luduvo] presence shape: {}", parse::shape(&json));
            presence.extend(parse::presence(&json));
        }
        for f in &mut friends {
            if let Some(p) = presence.remove(&f.id) {
                f.state = p.state;
                f.place_id = p.place_id;
                f.place_title = p.place_title;
                f.joinable = p.joinable;
            }
        }
        let rank = |f: &Friend| match f.state.as_str() {
            "in_game" => 0,
            "online" => 1,
            _ => 2,
        };
        friends.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase())));
        Ok(friends)
    }

    /// People asking to be your friend.
    pub async fn friend_requests(&self) -> Result<Vec<Friend>> {
        let json = self.authed(Method::GET, "/me/friends/pending?limit=200&offset=0").await?;
        #[cfg(debug_assertions)]
        eprintln!("[luduvo] pending shape: {}", parse::shape(&json));
        Ok(parse::friend_list(&json))
    }

    /// Accepts a request, or declines it the way the website does (by deleting it).
    pub async fn answer_request(&self, user_id: u64, accept: bool) -> Result<()> {
        if accept {
            self.authed(Method::POST, &format!("/friends/{user_id}/accept")).await?;
        } else {
            self.authed(Method::DELETE, &format!("/friends/{user_id}")).await?;
        }
        Ok(())
    }

    /// A seat in the server a friend is in.
    pub async fn join_friend(&self, user_id: u64) -> Result<String> {
        let answer = self.authed(Method::POST, &format!("/users/{user_id}/join")).await?;
        join_link(&answer)
    }

    pub async fn balance(&self) -> Result<Option<i64>> {
        let json = self.authed(Method::GET, "/me/balance").await?;
        #[cfg(debug_assertions)]
        eprintln!("[luduvo] balance shape: {}", parse::shape(&json));
        Ok(parse::balance(&json))
    }

    /// Claims today's reward. Returns Luduvo's answer for the UI to describe.
    pub async fn claim_daily(&self) -> Result<serde_json::Value> {
        let json = self.authed(Method::POST, "/me/rewards/daily").await?;
        #[cfg(debug_assertions)]
        eprintln!("[luduvo] daily reward shape: {}", parse::shape(&json));
        Ok(json)
    }

    pub async fn sign_out(&self) -> Result<()> {
        let refresh = tokio::task::spawn_blocking(load_refresh).await.map_err(|e| AppError::Other(e.to_string()))??;
        if let Some(r) = refresh {
            // Best effort: the session is forgotten here either way.
            let _ = self.send(Method::POST, "/auth/logout", Some(&serde_json::json!({ "refresh_token": r })), None).await;
        }
        *self.access.lock().unwrap() = None;
        tokio::task::spawn_blocking(clear_refresh).await.map_err(|e| AppError::Other(e.to_string()))?
    }

    pub fn cancel_sign_in(&self) {
        if let Some(tx) = self.cancel.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }

    /// Luduvo Studio's sign-in, done by Vodka: luduvo.com (in the person's own browser, where they may already be
    /// signed in) sends a one-time ticket to a listener on 127.0.0.1, which Vodka redeems for a session.
    pub async fn sign_in(&self) -> Result<User> {
        self.cancel_sign_in();
        let (tx, rx) = tokio::sync::oneshot::channel();
        *self.cancel.lock().unwrap() = Some(tx);

        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|e| AppError::Other(format!("Couldn't get ready for the Luduvo sign-in: {e}")))?;
        let port = listener.local_addr().map_err(|e| AppError::Other(e.to_string()))?.port();
        let state = nonce();
        open_in_browser(&format!("{SITE}/auth/studio?port={port}&state={state}"))?;

        let ticket = tokio::select! {
            t = wait_for_ticket(&listener, &state) => t?,
            _ = tokio::time::sleep(SIGN_IN_WAIT) => return Err(AppError::Other("The Luduvo sign-in took too long. Start it again.".into())),
            _ = rx => return Err(AppError::Other("Sign-in cancelled.".into())),
        };
        self.cancel.lock().unwrap().take();
        let (status, text) = self.send(Method::POST, "/auth/ticket/redeem", Some(&serde_json::json!({ "ticket": ticket })), None).await?;
        if !status.is_success() {
            return Err(explain(status, &text));
        }
        self.keep_tokens(&text).await?;
        self.me().await
    }
}

/// 64 lowercase hex characters, the `state` format luduvo.com accepts.
fn nonce() -> String {
    let mut bytes = [0u8; 32];
    let from_os = std::fs::File::open("/dev/urandom").and_then(|mut f| std::io::Read::read_exact(&mut f, &mut bytes)).is_ok();
    if !from_os {
        // Hash keys are seeded from the OS's random source.
        use std::hash::{BuildHasher, Hasher};
        for (i, chunk) in bytes.chunks_mut(8).enumerate() {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_usize(i);
            chunk.copy_from_slice(&h.finish().to_le_bytes());
        }
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn open_in_browser(url: &str) -> Result<()> {
    let parsed = Url::parse(url).map_err(|_| AppError::BlockedUrl)?;
    if parsed.scheme() != "https" || parsed.host_str() != Some("luduvo.com") {
        return Err(AppError::BlockedUrl);
    }
    open_browser(url)
}

/// Opens a page Vodka already checked in the person's own browser.
pub fn open_browser(url: &str) -> Result<()> {
    #[cfg(windows)]
    let mut cmd = {
        let mut c = std::process::Command::new("rundll32");
        c.args(["url.dll,FileProtocolHandler", url]);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    let mut child = cmd.spawn().map_err(|e| AppError::Other(format!("Couldn't open your browser for the Luduvo sign-in: {e}")))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Reads `ticket` from the one request that carries our `state`; anything else (a favicon, a stray tab) is turned away.
pub fn ticket_from_request(head: &str, state: &str) -> Option<String> {
    let target = head.lines().next()?.strip_prefix("GET ")?.split(' ').next()?;
    let url = Url::parse(&format!("http://127.0.0.1{target}")).ok()?;
    let mut ticket = None;
    let mut matched = false;
    for (k, v) in url.query_pairs() {
        match &*k {
            "ticket" => ticket = Some(v.into_owned()),
            "state" => matched = v == state,
            _ => {}
        }
    }
    ticket.filter(|t| matched && t.len() == 32 && t.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()))
}

async fn wait_for_ticket(listener: &tokio::net::TcpListener, state: &str) -> Result<String> {
    loop {
        let (mut sock, _) = listener.accept().await.map_err(|e| AppError::Other(format!("The Luduvo sign-in stopped: {e}")))?;
        let mut buf = vec![0u8; 8192];
        let mut len = 0;
        // Read the request head only; there is no body.
        let read = tokio::time::timeout(Duration::from_secs(5), async {
            while len < buf.len() {
                let n = sock.read(&mut buf[len..]).await?;
                if n == 0 {
                    break;
                }
                len += n;
                if buf[..len].windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            std::io::Result::Ok(())
        })
        .await;
        if !matches!(read, Ok(Ok(()))) {
            continue;
        }
        let head = String::from_utf8_lossy(&buf[..len]);
        let ticket = ticket_from_request(&head, state);
        let (code, page) = if ticket.is_some() {
            ("200 OK", "You're signed in to Luduvo in Vodka Launcher. You can close this tab.")
        } else {
            ("400 Bad Request", "This isn't a Luduvo sign-in Vodka started.")
        };
        let html = format!(
            "<!doctype html><meta charset=utf-8><title>Vodka Launcher</title><body style=\"font:16px system-ui;background:#05070a;color:#eef6fc;display:grid;place-items:center;height:90vh\"><p>{page}</p>"
        );
        let reply = format!(
            "HTTP/1.1 {code}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n{html}",
            html.len()
        );
        let _ = sock.write_all(reply.as_bytes()).await;
        let _ = sock.shutdown().await;
        if let Some(t) = ticket {
            return Ok(t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_luduvo_api_is_allowed() {
        assert!(check_url(&Url::parse("https://api.luduvo.com/me").unwrap()).is_ok());
        assert!(check_url(&Url::parse("http://api.luduvo.com/me").unwrap()).is_err());
        assert!(check_url(&Url::parse("https://luduvo.com/me").unwrap()).is_err());
        assert!(check_url(&Url::parse("https://api.luduvo.com.evil.test/me").unwrap()).is_err());
        assert!(check_url(&Url::parse("https://api.luduvo.com:8443/me").unwrap()).is_err());
    }

    #[test]
    fn builds_the_same_join_link_as_the_website() {
        let answer = serde_json::json!({ "token": "a b/c", "address": "1.2.3.4", "port": 7777, "public_key": "ab12" });
        assert_eq!(join_link(&answer).unwrap(), "luduvo://a%20b%2Fc/1.2.3.4/7777/ab12");
        let paid = serde_json::json!({ "token": "t", "address": "h", "port": "1", "public_key": "k", "purchase_token": "p" });
        assert_eq!(join_link(&paid).unwrap(), "luduvo://t/h/1/k/p");
        assert!(join_link(&serde_json::json!({ "token": "t", "address": "h", "port": 0, "public_key": "k" })).is_err());
        assert!(join_link(&serde_json::json!({ "token": "t", "port": 1, "public_key": "k" })).is_err());
    }

    #[test]
    fn takes_the_ticket_only_with_our_state() {
        let state = "f".repeat(64);
        let ticket = "0123456789abcdef0123456789abcdef";
        let ok = format!("GET /?ticket={ticket}&state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        assert_eq!(ticket_from_request(&ok, &state).as_deref(), Some(ticket));
        let wrong = format!("GET /?ticket={ticket}&state={} HTTP/1.1\r\n\r\n", "e".repeat(64));
        assert_eq!(ticket_from_request(&wrong, &state), None);
        assert_eq!(ticket_from_request("GET /favicon.ico HTTP/1.1\r\n\r\n", &state), None);
        let odd = format!("GET /?ticket=../../x&state={state} HTTP/1.1\r\n\r\n");
        assert_eq!(ticket_from_request(&odd, &state), None);
    }

    #[test]
    fn nonce_matches_what_luduvo_accepts() {
        let n = nonce();
        assert_eq!(n.len(), 64);
        assert!(n.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
        assert_ne!(n, nonce());
    }

    /// Talks to the real api.luduvo.com: `cargo test luduvo -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn live_discovery_and_search() {
        let client = Client::new();
        let shelves = client.discover().await.unwrap();
        assert!(shelves.iter().any(|s| !s.places.is_empty()), "{shelves:?}");
        let found = client.search("snow battle").await.unwrap();
        assert!(found.iter().any(|p| p.title == "snow battle"), "{found:?}");
        assert!(client.recently_played().await.is_err() || client.has_session().await.unwrap());
        println!("{} shelves; first: {:?}", shelves.len(), shelves[0].places.first());
        let cats = client.store_categories().await.unwrap();
        let (items, total) = client.store("", "", "", "popular", 0).await.unwrap();
        assert!(!cats.is_empty() && !items.is_empty() && total > 0);
        let hats = client.store("", "accessory", "hat", "newest", 0).await.unwrap().0;
        assert!(hats.iter().all(|i| i.slot == "hat"), "{hats:?}");
        let one = client.store_item(items[0].id).await.unwrap();
        let picture = client.avatar_picture(1).await.unwrap();
        let prof = client.profile(1).await.unwrap();
        assert_eq!(prof.id, 1);
        assert!(prof.picture_url.is_some() && !prof.wearing.is_empty(), "{prof:?}");
        let _ = (client.user_friends(1).await.unwrap(), client.user_places(1).await.unwrap(), client.user_groups(1).await.unwrap());
        println!("profile: {} ({}), state {}, {} worn, badges {:?}", prof.display_name, prof.name, prof.state, prof.wearing.len(), prof.badges);
        assert!(picture.as_deref().is_some_and(|u| u.starts_with("https://assets.luduvo.com/full_body/")), "{picture:?}");
        println!("{} categories, {total} items; popular: {} ({:?} C$, {} sold, {:?} owners)", cats.len(), one.name, one.price, one.sales, one.owners);
    }

    #[test]
    fn explains_join_errors() {
        let e = explain(StatusCode::TOO_MANY_REQUESTS, r#"{"error":"join_flood"}"#);
        assert!(e.to_string().contains("too quickly"));
        let e = explain(StatusCode::BAD_REQUEST, "<html>nope</html>");
        assert!(e.to_string().contains("HTTP 400"));
    }
}
