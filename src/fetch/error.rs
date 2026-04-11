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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::client::build_client;

    #[test]
    fn display_messages_for_rate_limited_and_server_error() {
        assert_eq!(
            FetchError::RateLimited.to_string(),
            "rate-limited after max retries"
        );
        assert_eq!(
            FetchError::ServerError(503).to_string(),
            "server error 503 after max retries"
        );
    }

    #[tokio::test]
    async fn display_message_for_network_error_variant() {
        let client = build_client().unwrap();
        let err = client.get("::not-a-url::").send().await.unwrap_err();
        let display = FetchError::Network(err).to_string();
        assert!(display.starts_with("network error: "));
    }
}
