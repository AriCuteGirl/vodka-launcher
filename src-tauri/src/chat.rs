//! Vodka's own chat: a small server (see `server/` in the repository) that relays messages
//! and game invites between people who use Vodka. It is opt-in per account.
//!
//! What goes to that server: the Roblox user id, the public names, messages and invites.
//! What never goes there: the Roblox session cookie. Ownership of an account is proven by
//! a short phrase the user puts in their public Roblox profile.

use crate::error::{AppError, Result};
use serde_json::{Value, json};
use std::time::Duration;

/// The one address this module talks to.
const SERVER: &str = "https://tcmfsivvjinjqpppkmhk.supabase.co/functions/v1/vodka";
const KEYRING_SERVICE: &str = "vodka-launcher";

pub fn entry_name(account_id: u64) -> String {
    format!("vodka-chat:{account_id}")
}

fn entry(account_id: u64) -> Result<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, &entry_name(account_id)).map_err(|e| AppError::Keyring(e.to_string()))
}

pub fn load_token(account_id: u64) -> Result<Option<String>> {
    match entry(account_id)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Keyring(e.to_string())),
    }
}

pub fn store_token(account_id: u64, token: &str) -> Result<()> {
    if !valid_token(token) {
        return Err(AppError::Chat("The chat server sent a sign-in Vodka can't use.".into()));
    }
    entry(account_id)?.set_password(token).map_err(|e| AppError::Keyring(e.to_string()))
}

pub fn clear_token(account_id: u64) -> Result<()> {
    match entry(account_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Keyring(e.to_string())),
    }
}

pub fn valid_token(token: &str) -> bool {
    token.len() == 64 && token.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Turns the server's answer into data or the error it described.
pub fn interpret(status: u16, body: &str) -> Result<Value> {
    let parsed: Option<Value> = serde_json::from_str(body).ok();
    if (200..300).contains(&status) {
        return parsed.ok_or_else(|| AppError::Chat("The chat server sent something Vodka didn't understand.".into()));
    }
    let error = parsed.as_ref().and_then(|v| v.get("error"));
    let code = error.and_then(|e| e.get("code")).and_then(Value::as_str).unwrap_or("");
    let message = error.and_then(|e| e.get("message")).and_then(Value::as_str);
    Err(match (status, code) {
        (401, _) | (_, "signed_out") => AppError::ChatSignedOut,
        (_, _) => AppError::Chat(message.map(str::to_owned).unwrap_or_else(|| format!("The chat server answered with an error ({status})."))),
    })
}

pub struct Client {
    http: reqwest::Client,
}

impl Client {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("VodkaLauncher/", env!("CARGO_PKG_VERSION")))
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(20))
            .build()
            .expect("HTTP client");
        Self { http }
    }

    /// Sends one action. `token` is the chat sign-in, for actions that need one.
    pub async fn call(&self, token: Option<&str>, mut body: Value, action: &str) -> Result<Value> {
        body["action"] = json!(action);
        let mut req = self.http.post(SERVER).json(&body);
        if let Some(t) = token {
            req = req.header("x-vodka-token", t);
        }
        let res = req.send().await.map_err(|e| {
            if e.is_connect() || e.is_timeout() { AppError::Offline } else { AppError::Chat(e.without_url().to_string()) }
        })?;
        let status = res.status().as_u16();
        let text = res.text().await.map_err(|e| AppError::Chat(e.without_url().to_string()))?;
        interpret(status, &text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_address_is_fixed_and_https() {
        assert!(SERVER.starts_with("https://") && SERVER.ends_with("/functions/v1/vodka"));
    }

    #[test]
    fn success_returns_the_data() {
        let v = interpret(200, r#"{"challenge":"vodka amber river"}"#).unwrap();
        assert_eq!(v["challenge"], "vodka amber river");
        assert_eq!(interpret(200, "<html>").unwrap_err().kind(), "chat");
    }

    #[test]
    fn server_errors_keep_their_message() {
        let e = interpret(409, r#"{"error":{"code":"not_found","message":"The code isn't in your Roblox About text yet."}}"#).unwrap_err();
        assert_eq!(e.kind(), "chat");
        assert!(e.to_string().contains("About text"));
        assert!(interpret(502, "bad gateway").unwrap_err().to_string().contains("502"));
    }

    #[test]
    fn a_dead_sign_in_is_its_own_error() {
        let e = interpret(401, r#"{"error":{"code":"signed_out","message":"x"}}"#).unwrap_err();
        assert_eq!(e.kind(), "chat_signed_out");
    }

    #[test]
    fn tokens_are_64_lowercase_hex() {
        assert!(valid_token(&"a1".repeat(32)));
        assert!(!valid_token(&"A1".repeat(32)));
        assert!(!valid_token("abc"));
        assert!(!valid_token(&"g".repeat(64)));
    }

    #[test]
    fn chat_sign_ins_have_their_own_keyring_entries() {
        assert_eq!(entry_name(7), "vodka-chat:7");
        assert_ne!(entry_name(7), crate::accounts::entry_name(7));
    }
}
