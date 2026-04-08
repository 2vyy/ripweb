use std::time::Duration;

use rquest::Client;

pub use super::error::FetchError;

/// Retry policy handed to [`fetch_with_retry`].
pub struct RetryConfig {
    /// Maximum number of retry attempts (not counting the first try).
    pub max_retries: u32,
    /// Base delay before the first retry.  Each subsequent retry doubles it
    /// (exponential backoff) with a random jitter ≤ base_delay.
    pub base_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base_delay: Duration::from_millis(500),
        }
    }
}

/// Build the shared `rquest::Client` with a strict 10-second timeout.
pub fn build_client() -> rquest::Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
}

/// Returns `true` for status codes that should trigger a retry.
fn is_retryable(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

/// Fetch `url`, retrying on HTTP 429 / 5xx with exponential back-off + jitter.
///
/// Returns the successful [`rquest::Response`] so the caller can inspect
/// headers (pre-flight) and stream the body.
pub async fn fetch_with_retry(
    client: &Client,
    url: &str,
    cfg: &RetryConfig,
) -> Result<rquest::Response, FetchError> {
    let mut delay = cfg.base_delay;
    let mut last_status: u16 = 0;

    for attempt in 0..=cfg.max_retries {
        let resp = client
            .get(url)
            .send()
            .await
            .map_err(FetchError::Network)?;

        let status = resp.status().as_u16();

        if resp.status().is_success() {
            return Ok(resp);
        }

        if !is_retryable(status) {
            return Err(FetchError::ServerError(status));
        }

        last_status = status;

        // Don't sleep after the final attempt.
        if attempt < cfg.max_retries {
            let jitter_ms = rand::random_range(0..=delay.as_millis() as u64);
            tokio::time::sleep(delay + Duration::from_millis(jitter_ms)).await;
            delay *= 2;
        }
    }

    if last_status == 429 {
        Err(FetchError::RateLimited)
    } else {
        Err(FetchError::ServerError(last_status))
    }
}
