//! The signed-in Roblox account. The session cookie lives in the system keyring
//! (Secret Service) and nowhere else: not in files, not in logs, not in events.

use crate::error::{AppError, Result};
use crate::roblox_api::check_url;
use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindowBuilder};

const SERVICE: &str = "vodka-launcher";
const ENTRY: &str = "roblox-session";
const LOGIN_WINDOW: &str = "roblox-login";
const LOGIN_URL: &str = "https://www.roblox.com/login";
const COOKIE_NAME: &str = ".ROBLOSECURITY";

fn entry() -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, ENTRY).map_err(|e| AppError::Keyring(e.to_string()))
}

/// A cookie value we are willing to store and send: printable ASCII, no separators.
pub fn valid_cookie(value: &str) -> bool {
    (20..=4096).contains(&value.len()) && value.bytes().all(|b| (0x21..0x7f).contains(&b) && b != b';')
}

pub fn load_session() -> Result<Option<String>> {
    match entry()?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Keyring(e.to_string())),
    }
}

pub fn store_session(cookie: &str) -> Result<()> {
    if !valid_cookie(cookie) {
        return Err(AppError::Other("Roblox returned a session Vodka can't store safely.".into()));
    }
    entry()?.set_password(cookie).map_err(|e| AppError::Keyring(e.to_string()))
}

pub fn clear_session() -> Result<()> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Keyring(e.to_string())),
    }
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
                        Ok(user) => store_session(&value).map(|_| user),
                        Err(AppError::SessionExpired) => return,
                        Err(e) => Err(e),
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
}
