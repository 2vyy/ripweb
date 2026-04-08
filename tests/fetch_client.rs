use std::time::Duration;

use ripweb::fetch::client::{build_client, fetch_with_retry, FetchError, RetryConfig};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fast_retry() -> RetryConfig {
    RetryConfig {
        max_retries: 2,
        base_delay: Duration::from_millis(1),
    }
}

// ── Happy path ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn fetch_succeeds_on_200() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("hello"))
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    let resp = fetch_with_retry(&client, &server.uri(), &fast_retry())
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

// ── Retry on 429 ─────────────────────────────────────────────────────────────

/// Server returns 429 twice, then 200.  Must succeed within 2 retries.
#[tokio::test]
async fn retries_twice_on_429_then_succeeds() {
    let server = MockServer::start().await;

    // First two requests → 429
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429))
        .up_to_n_times(2)
        .mount(&server)
        .await;

    // Third request → 200
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    let resp = fetch_with_retry(&client, &server.uri(), &fast_retry())
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

/// Server returns 429 three times (more than max_retries=2).  Must error.
#[tokio::test]
async fn fails_after_max_retries_on_429() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    let err = fetch_with_retry(&client, &server.uri(), &fast_retry())
        .await
        .unwrap_err();
    assert!(
        matches!(err, FetchError::RateLimited),
        "expected RateLimited, got: {err}"
    );
}

// ── Retry on 503 / 504 ───────────────────────────────────────────────────────

#[tokio::test]
async fn retries_on_503_then_succeeds() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("recovered"))
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    let resp = fetch_with_retry(&client, &server.uri(), &fast_retry())
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn fails_after_max_retries_on_503() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    let err = fetch_with_retry(&client, &server.uri(), &fast_retry())
        .await
        .unwrap_err();
    assert!(
        matches!(err, FetchError::ServerError(503)),
        "expected ServerError(503), got: {err}"
    );
}

// ── Non-retryable errors ──────────────────────────────────────────────────────

/// A 404 is a client error — must NOT be retried.
#[tokio::test]
async fn does_not_retry_on_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = build_client().unwrap();
    // Should fail immediately with ServerError, not RateLimited
    let err = fetch_with_retry(&client, &server.uri(), &fast_retry())
        .await
        .unwrap_err();
    assert!(
        matches!(err, FetchError::ServerError(404)),
        "expected ServerError(404), got: {err}"
    );

    // Confirm wiremock only received exactly 1 request (no retries).
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}
