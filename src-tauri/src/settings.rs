//! Non-secret preferences, kept as JSON in Vodka's own config dir.
//! Account names and ids live here; their sessions live in the keyring.

use crate::error::{AppError, Result};
use crate::tweaks::LaunchOptions;
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SavedAccount {
    pub id: u64,
    pub name: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub onboarded: bool,
    pub accounts: Vec<SavedAccount>,
    pub active_account: Option<u64>,
    /// The user has read and accepted the multi-instance warning.
    pub multi_instance_accepted: bool,
    /// Launch options for every account.
    pub defaults: LaunchOptions,
    /// Per-account launch options, keyed by account id. They override `defaults`.
    pub account_options: BTreeMap<u64, LaunchOptions>,
}

impl Settings {
    pub fn active(&self) -> Option<&SavedAccount> {
        let id = self.active_account?;
        self.accounts.iter().find(|a| a.id == id)
    }

    /// Adds or refreshes an account and makes it the active one.
    pub fn upsert_active(&mut self, account: SavedAccount) {
        self.active_account = Some(account.id);
        match self.accounts.iter_mut().find(|a| a.id == account.id) {
            Some(existing) => *existing = account,
            None => self.accounts.push(account),
        }
    }

    /// Removes an account; another one becomes active if there is one.
    pub fn remove(&mut self, id: u64) {
        self.accounts.retain(|a| a.id != id);
        self.account_options.remove(&id);
        if self.active_account == Some(id) {
            self.active_account = self.accounts.first().map(|a| a.id);
        }
    }
}

static LOCK: Mutex<()> = Mutex::new(());

pub fn load(dir: &Path) -> Settings {
    std::fs::read_to_string(dir.join("settings.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save(dir: &Path, settings: &Settings) -> Result<()> {
    let write = || -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let tmp = dir.join("settings.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
        std::fs::rename(tmp, dir.join("settings.json"))
    };
    write().map_err(|e| AppError::Other(format!("Could not save settings: {e}")))
}

/// Read-modify-write under a lock, so two commands can't lose each other's change.
pub fn update(dir: &Path, change: impl FnOnce(&mut Settings)) -> Result<Settings> {
    let _guard = LOCK.lock().unwrap();
    let mut settings = load(dir);
    change(&mut settings);
    save(dir, &settings)?;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acct(id: u64) -> SavedAccount {
        SavedAccount { id, name: format!("user{id}"), display_name: format!("User {id}") }
    }

    #[test]
    fn round_trips_and_tolerates_missing_or_broken_files() {
        let dir = std::env::temp_dir().join(format!("vodka-settings-test-{}", std::process::id()));
        assert_eq!(load(&dir), Settings::default());
        let saved = update(&dir, |s| {
            s.onboarded = true;
            s.upsert_active(acct(7));
        })
        .unwrap();
        assert_eq!(load(&dir), saved);
        std::fs::write(dir.join("settings.json"), "{ nope").unwrap();
        assert_eq!(load(&dir), Settings::default());
        // A file from phase 1, with a field that no longer exists, still loads.
        std::fs::write(dir.join("settings.json"), r#"{"onboarded":true,"shareSessionWithMocktail":true}"#).unwrap();
        assert!(load(&dir).onboarded);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn adding_switching_and_removing_accounts() {
        let mut s = Settings::default();
        s.upsert_active(acct(1));
        s.upsert_active(acct(2));
        assert_eq!(s.active().unwrap().id, 2);
        // Signing in again as an existing account updates it instead of duplicating.
        s.upsert_active(SavedAccount { display_name: "Renamed".into(), ..acct(1) });
        assert_eq!(s.accounts.len(), 2);
        assert_eq!(s.active().unwrap().display_name, "Renamed");
        s.account_options.insert(1, LaunchOptions { fps: Some("30".into()), ..Default::default() });
        s.remove(1);
        assert!(s.account_options.is_empty());
        assert_eq!(s.active().unwrap().id, 2);
        s.remove(2);
        assert!(s.active().is_none());
    }
}
