//! Network Errors
//!
//! Defines errors specific to the fetch layer, including rate-limiting,
//! server failures, and network timeouts.

use rquest::Error;

#[derive(Debug)]
pub enum FetchError {
    Network(Error),
    RateLimited,
    ServerError(u16),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "network error: {e}"),
            Self::RateLimited => write!(f, "rate-limited after max retries"),
            Self::ServerError(c) => write!(f, "server error {c} after max retries"),
        }
    }
}
