//! One error type for everything the UI can be told about.

use serde::Serialize;
use serde::ser::SerializeStruct;

#[derive(Debug, Clone, thiserror::Error)]
pub enum AppError {
    #[error("Flatpak is not installed, so Vodka can't find Mocktail.")]
    FlatpakMissing,
    #[error("Mocktail is not installed.")]
    MocktailMissing,
    #[error("Mocktail could not be started: {0}")]
    Launch(String),
    #[error("You're not signed in to Roblox.")]
    NotSignedIn,
    #[error("Your Roblox session has expired. Sign in again.")]
    SessionExpired,
    #[error("Roblox is limiting requests right now. Give it a minute and retry.")]
    RateLimited,
    #[error("Can't reach Roblox. Check your internet connection.")]
    Offline,
    #[error("Roblox sent a response Vodka didn't understand ({0}).")]
    Api(String),
    #[error("The system keyring isn't available: {0}")]
    Keyring(String),
    #[error("Vodka refused to contact an address that isn't roblox.com.")]
    BlockedUrl,
    #[error("{0}")]
    Chat(String),
    #[error("You're not signed in to Vodka chat on this account.")]
    ChatSignedOut,
    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::FlatpakMissing => "flatpak_missing",
            Self::MocktailMissing => "mocktail_missing",
            Self::Launch(_) => "launch",
            Self::NotSignedIn => "not_signed_in",
            Self::SessionExpired => "session_expired",
            Self::RateLimited => "rate_limited",
            Self::Offline => "offline",
            Self::Api(_) => "api",
            Self::Keyring(_) => "keyring",
            Self::BlockedUrl => "blocked_url",
            Self::Chat(_) => "chat",
            Self::ChatSignedOut => "chat_signed_out",
            Self::Other(_) => "other",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("AppError", 2)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
