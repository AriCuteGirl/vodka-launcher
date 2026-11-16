//! Signed-in Roblox accounts. Each session cookie lives in the system keyring
//! (Secret Service) under its account id, and nowhere else Vodka controls.

use crate::error::{AppError, Result};
use crate::roblox_api::check_url;
use crate::settings::{self, SavedAccount};
use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindowBuilder};

const SERVICE: &str = "vodka-launcher";
/// Phase 1 kept a single session under this name.
const LEGACY_ENTRY: &str = "roblox-session";
const LOGIN_WINDOW: &str = "roblox-login";
const LOGIN_URL: &str = "https://www.roblox.com/login";
const COOKIE_NAME: &str = ".ROBLOSECURITY";

pub fn entry_name(account_id: u64) -> String {
    format!("roblox-session:{account_id}")
}

fn entry(name: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, name).map_err(|e| AppError::Keyring(e.to_string()))
}

/// A cookie value we are willing to store and send: printable ASCII, no separators.
pub fn valid_cookie(value: &str) -> bool {
    (20..=4096).contains(&value.len()) && value.bytes().all(|b| (0x21..0x7f).contains(&b) && b != b';')
}

fn read(name: &str) -> Result<Option<String>> {
    match entry(name)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Keyring(e.to_string())),
    }
}

fn delete(name: &str) -> Result<()> {
    match entry(name)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Keyring(e.to_string())),
    }
}

pub fn load_session(account_id: u64) -> Result<Option<String>> {
    read(&entry_name(account_id))
}

pub fn store_session(account_id: u64, cookie: &str) -> Result<()> {
    if !valid_cookie(cookie) {
        return Err(AppError::Other("Roblox returned a session Vodka can't store safely.".into()));
    }
    entry(&entry_name(account_id))?.set_password(cookie).map_err(|e| AppError::Keyring(e.to_string()))
}

pub fn clear_session(account_id: u64) -> Result<()> {
    delete(&entry_name(account_id))
}

pub fn load_legacy_session() -> Result<Option<String>> {
    read(LEGACY_ENTRY)
}

pub fn clear_legacy_session() -> Result<()> {
    delete(LEGACY_ENTRY)
}

/// Opens Roblox's own login page in a locked-down, private window.
/// It can only navigate within roblox.com, keeps nothing on disk, and has no access to Vodka's commands.
pub fn open_login(app: &AppHandle) -> Result<()> {
    if let Some(existing) = app.get_webview_window(LOGIN_WINDOW) {
        let _ = existing.set_focus();
        return Ok(());
    }
    let url = Url::parse(LOGIN_URL).expect("static url");
    let handle = app.clone();
    let window = WebviewWindowBuilder::new(app, LOGIN_WINDOW, WebviewUrl::External(url))
        .title("Sign in to Roblox")
        .inner_size(480.0, 720.0)
        .incognito(true)
        .on_navigation(|url| check_url(url).is_ok())
        .on_page_load(move |window, payload| {
            if payload.event() != tauri::webview::PageLoadEvent::Finished {
                return;
            }
            let handle = handle.clone();
            // Off the UI thread: reading cookies there can deadlock.
            tauri::async_runtime::spawn_blocking(move || {
                let Ok(site) = Url::parse("https://www.roblox.com/") else { return };
                let Ok(cookies) = window.cookies_for_url(site) else { return };
                let Some(cookie) = cookies.iter().find(|c| c.name() == COOKIE_NAME) else { return };
                let value = cookie.value().to_owned();
                tauri::async_runtime::spawn(async move {
                    let state = handle.state::<crate::AppState>();
                    // Only a cookie Roblox accepts counts as signed in.
                    let result = match state.api.current_user(&value).await {
                        Err(AppError::SessionExpired) => return,
                        Err(e) => Err(e),
                        Ok(user) => (|| {
                            store_session(user.id, &value)?;
                            let dir = handle.path().app_config_dir().map_err(|e| AppError::Other(e.to_string()))?;
                            settings::update(&dir, |s| {
                                s.upsert_active(SavedAccount {
                                    id: user.id,
                                    name: user.name.clone(),
                                    display_name: user.display_name.clone(),
                                })
                            })?;
                            // The window that signed in uses the new account.
                            *state.active.lock().unwrap() = Some(user.id);
                            Ok(user)
                        })(),
                    };
                    state.api.clear_cache();
                    match result {
                        Ok(user) => {
                            let _ = handle.emit("auth://signed-in", user);
                        }
                        Err(e) => {
                            let _ = handle.emit("auth://error", e);
                        }
                    }
                    if let Some(w) = handle.get_webview_window(LOGIN_WINDOW) {
                        let _ = w.clear_all_browsing_data();
                        let _ = w.close();
                    }
                });
            });
        })
        .build()
        .map_err(|e| AppError::Other(format!("Could not open the sign-in window: {e}")))?;
    let handle = app.clone();
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) {
            let _ = handle.emit("auth://closed", ());
        }
    });
    Ok(())
}

pub const ROBLOX_PAGE_WINDOW: &str = "roblox-page";

/// Runs on a group's page in the hidden Roblox window: presses its "Join Community" button once, so Roblox's own
/// page does the join, including the human check it runs in the background.
pub const JOIN_GROUP_SCRIPT: &str = r#"(() => {
  if (!/^(www\.)?roblox\.com$/.test(location.hostname) || !/^\/(communities|groups)\//.test(location.pathname)) return;
  let pressed = false;
  const started = Date.now();
  const tick = setInterval(() => {
    if (pressed || Date.now() - started > 30000) return clearInterval(tick);
    const button = [...document.querySelectorAll("button")].find((b) =>
      /^\s*join (community|group)\s*$/i.test(b.textContent || "") && !b.disabled
    );
    if (button) {
      pressed = true;
      button.click();
    }
  }, 500);
})();"#;

/// The Roblox hosts roblox.com's own pages call while signed in.
const ROBLOX_HOSTS: &[&str] = &[
    "roblox.com",
    "www.roblox.com",
    "web.roblox.com",
    "apis.roblox.com",
    "auth.roblox.com",
    "users.roblox.com",
    "games.roblox.com",
    "gamejoin.roblox.com",
    "economy.roblox.com",
    "billing.roblox.com",
    "catalog.roblox.com",
    "friends.roblox.com",
    "presence.roblox.com",
    "thumbnails.roblox.com",
    "avatar.roblox.com",
    "groups.roblox.com",
    "badges.roblox.com",
    "inventory.roblox.com",
    "trades.roblox.com",
    "accountsettings.roblox.com",
    "accountinformation.roblox.com",
    "premiumfeatures.roblox.com",
    "privatemessages.roblox.com",
    "notifications.roblox.com",
    "realtime-signalr.roblox.com",
    "twostepverification.roblox.com",
    "develop.roblox.com",
    "followings.roblox.com",
    "voice.roblox.com",
    "metrics.roblox.com",
    "ecsv2.roblox.com",
    "locale.roblox.com",
    "translations.roblox.com",
    "contacts.roblox.com",
    "chat.roblox.com",
];

/// Hosts the in-app Roblox window may show: Roblox itself and the captcha Roblox uses.
pub fn page_host_allowed(url: &Url) -> bool {
    if url.scheme() == "about" {
        return true;
    }
    let host = url.host_str().unwrap_or_default();
    url.scheme() == "https"
        && (host == "roblox.com"
            || host.ends_with(".roblox.com")
            || host.ends_with(".rbxcdn.com")
            || host == "arkoselabs.com"
            || host.ends_with(".arkoselabs.com")
            || host.ends_with(".arkoselabs.roblox.com"))
}

/// Opens a roblox.com page in a private window inside Vodka, signed in as `cookie`'s account, so Roblox's own page
/// can do what Vodka can't (a purchase with its captcha and price confirmation). It can only go to Roblox and its
/// captcha, keeps nothing on disk, and is wiped when it closes.
/// With `hidden`, the window starts invisible (for doing something on the page in the background) and `script`
/// runs on each page it loads.
pub fn open_roblox_page(app: &AppHandle, cookie: &str, url: Url, title: &str, hidden: bool, script: Option<&str>) -> Result<tauri::WebviewWindow> {
    if !valid_cookie(cookie) || url.scheme() != "https" || !matches!(url.host_str(), Some("www.roblox.com")) {
        return Err(AppError::BlockedUrl);
    }
    if let Some(existing) = app.get_webview_window(ROBLOX_PAGE_WINDOW) {
        let _ = existing.close();
    }
    let blank = Url::parse("about:blank").expect("static url");
    let builder = WebviewWindowBuilder::new(app, ROBLOX_PAGE_WINDOW, WebviewUrl::External(blank))
        .title(title)
        .inner_size(1000.0, 780.0)
        .incognito(true)
        .visible(!hidden)
        .on_navigation(page_host_allowed);
    let builder = match script {
        Some(js) => builder.initialization_script(js),
        None => builder,
    };
    let window = builder
        .build()
        .map_err(|e| AppError::Other(format!("Could not open the Roblox window: {e}")))?;
    // The cookie library drops the leading dot from ".roblox.com", which the Linux webview then treats as
    // "roblox.com only". So the session is handed to each Roblox host the page talks to, one by one.
    for host in ROBLOX_HOSTS {
        let session = tauri::webview::cookie::Cookie::build((COOKIE_NAME, cookie.to_owned()))
            .domain(*host)
            .path("/")
            .secure(true)
            .http_only(true)
            .build();
        window.set_cookie(session).map_err(|e| AppError::Other(format!("Could not sign the Roblox window in: {e}")))?;
    }
    window.navigate(url).map_err(|e| AppError::Other(format!("Could not open the Roblox page: {e}")))?;
    let handle = app.clone();
    let wiped = window.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { .. } => {
            let _ = wiped.clear_all_browsing_data();
        }
        tauri::WindowEvent::Destroyed => {
            let _ = handle.emit("roblox-page://closed", ());
        }
        _ => {}
    });
    Ok(window)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_cookies_that_could_break_a_header() {
        let good = "_|WARNING:-DO-NOT-SHARE-THIS.--Sharing-this-will-allow-someone-to-log-in|_ABCDEF0123";
        assert!(valid_cookie(good));
        assert!(!valid_cookie("short"));
        assert!(!valid_cookie(&format!("{good}; other=1")));
        assert!(!valid_cookie(&format!("{good}\r\nX-Evil: 1")));
        assert!(!valid_cookie(&format!("{good} space")));
    }

    #[test]
    fn each_account_gets_its_own_keyring_entry() {
        assert_eq!(entry_name(42), "roblox-session:42");
        assert_ne!(entry_name(1), entry_name(11));
        assert_ne!(entry_name(1), LEGACY_ENTRY);
    }

    #[test]
    fn the_roblox_window_stays_on_roblox_and_its_captcha() {
        let ok = |u: &str| page_host_allowed(&Url::parse(u).unwrap());
        assert!(ok("https://www.roblox.com/games/1"));
        assert!(ok("https://roblox-api.arkoselabs.com/v2/x"));
        assert!(ok("about:blank"));
        assert!(!ok("http://www.roblox.com/"));
        assert!(!ok("https://roblox.com.evil.test/"));
        assert!(!ok("https://evil.test/"));
    }
}
