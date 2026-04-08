//! Central Error Definitions
//!
//! Provides the `RipwebError` enum which maps internal failures (Network,
//! Config, RateLimit) to CLI exit codes and user-facing messages.

use crate::fetch::FetchError;

#[derive(Debug, thiserror::Error)]
pub enum RipwebError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("blocked (rate-limited or WAF): exhausted retries")]
    Blocked,
    #[error("no content: fetched successfully but extracted nothing")]
    NoContent,
    #[error("input too large: {0} bytes exceeds 5MB limit")]
    InputTooLarge(usize),
}

impl RipwebError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Config(_) => 1,
            Self::Network(_) => 2,
            Self::Blocked => 3,
            Self::NoContent => 4,
            Self::InputTooLarge(_) => 4,
        }
    }
}

impl From<FetchError> for RipwebError {
    fn from(e: FetchError) -> Self {
        match e {
            FetchError::RateLimited => Self::Blocked,
            FetchError::ServerError(403) => Self::Blocked,
            FetchError::Network(e) => Self::Network(e.to_string()),
            FetchError::ServerError(c) => Self::Network(format!("HTTP {c}")),
        }
    }
}
