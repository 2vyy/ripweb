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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::client::build_client;

    #[test]
    fn exit_codes_match_cli_contract() {
        assert_eq!(RipwebError::Config("x".into()).exit_code(), 1);
        assert_eq!(RipwebError::Network("x".into()).exit_code(), 2);
        assert_eq!(RipwebError::Blocked.exit_code(), 3);
        assert_eq!(RipwebError::NoContent.exit_code(), 4);
        assert_eq!(RipwebError::InputTooLarge(10).exit_code(), 4);
    }

    #[test]
    fn fetch_error_conversion_maps_rate_limits_and_statuses() {
        let blocked = RipwebError::from(FetchError::RateLimited);
        assert!(matches!(blocked, RipwebError::Blocked));

        let forbidden = RipwebError::from(FetchError::ServerError(403));
        assert!(matches!(forbidden, RipwebError::Blocked));

        let upstream = RipwebError::from(FetchError::ServerError(502));
        assert!(matches!(upstream, RipwebError::Network(msg) if msg == "HTTP 502"));
    }

    #[tokio::test]
    async fn fetch_error_conversion_maps_network_errors() {
        let client = build_client().unwrap();
        let net_err = client.get("::not-a-url::").send().await.unwrap_err();
        let mapped = RipwebError::from(FetchError::Network(net_err));
        assert!(matches!(mapped, RipwebError::Network(_)));
    }
}
