//! Central Error Definitions
//!
//! Provides the `RipwebError` enum which maps internal failures (Network,
//! Config, RateLimit) to CLI exit codes and user-facing messages.

use std::fmt;

use crate::fetch::FetchError;

#[derive(Debug)]
pub enum RipwebError {
    Config(String),
    Network(String),
    Blocked,
    NoContent,
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

impl fmt::Display for RipwebError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(s) => write!(f, "configuration error: {s}"),
            Self::Network(s) => write!(f, "network error: {s}"),
            Self::Blocked => write!(f, "blocked (rate-limited or WAF): exhausted retries"),
            Self::NoContent => write!(f, "no content: fetched successfully but extracted nothing"),
            Self::InputTooLarge(n) => write!(f, "input too large: {} bytes exceeds 5MB limit", n),
        }
    }
}

impl std::error::Error for RipwebError {}

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
