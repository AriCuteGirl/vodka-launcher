//! Non-secret preferences, kept as JSON in Vodka's own config dir.

use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub onboarded: bool,
    /// Experimental: start Mocktail signed in as Vodka's account.
    pub share_session_with_mocktail: bool,
}

pub fn load(dir: &Path) -> Settings {
    std::fs::read_to_string(dir.join("settings.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(dir: &Path, settings: &Settings) -> Result<()> {
    let write = || -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let tmp = dir.join("settings.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
        std::fs::rename(tmp, dir.join("settings.json"))
    };
    write().map_err(|e| AppError::Other(format!("Could not save settings: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_tolerates_missing_or_broken_files() {
        let dir = std::env::temp_dir().join(format!("vodka-settings-test-{}", std::process::id()));
        assert_eq!(load(&dir), Settings::default());
        let s = Settings { onboarded: true, share_session_with_mocktail: true };
        save(&dir, &s).unwrap();
        assert_eq!(load(&dir), s);
        std::fs::write(dir.join("settings.json"), "{ nope").unwrap();
        assert_eq!(load(&dir), Settings::default());
        std::fs::write(dir.join("settings.json"), r#"{"onboarded":true}"#).unwrap();
        assert!(load(&dir).onboarded);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
