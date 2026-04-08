/// Phase 3 network tests.
///
/// Politeness tests spin up real wiremock servers so we never touch the live
/// internet.  Pre-flight tests are pure unit-level — they inspect headers only.
use std::sync::Arc;
use std::time::{Duration, Instant};

use ripweb::fetch::{
    politeness::DomainSemaphores,
    preflight::{PreflightError, PreflightCheck},
};
use tokio::sync::Semaphore;

// ── Domain Politeness ─────────────────────────────────────────────────────────

/// Three tasks hitting the same host concurrently must be serialised to at
/// most 3-at-a-time.  We verify that a 4th attempt must wait for one of the
/// first three to finish.
#[tokio::test]
async fn politeness_limits_concurrent_requests_per_host() {
    let sems = DomainSemaphores::new(3);

    let start = Instant::now();

    // Grab 3 permits simultaneously — should succeed immediately.
    let p1 = sems.acquire("example.com").await;
    let p2 = sems.acquire("example.com").await;
    let p3 = sems.acquire("example.com").await;

    // Spawn a task that tries to grab a 4th permit while 3 are held.
    // It must block until we release one.
    let sems_clone = sems.clone();
    let waiter = tokio::spawn(async move {
        let _p4 = sems_clone.acquire("example.com").await;
        Instant::now()
    });

    // Give the waiter a moment to block, then release one permit.
    tokio::time::sleep(Duration::from_millis(50)).await;
    drop(p1);

    let unblocked_at = waiter.await.unwrap();
    let waited = unblocked_at.duration_since(start);

    // The waiter must have blocked for at least the sleep we inserted.
    assert!(
        waited >= Duration::from_millis(40),
        "4th request was not blocked by the semaphore (waited {:?})",
        waited
    );

    drop(p2);
    drop(p3);
}

/// Different hosts must get independent semaphores — acquiring 3 permits on
/// host A must not block a request to host B.
#[tokio::test]
async fn politeness_different_hosts_are_independent() {
    let sems = DomainSemaphores::new(3);

    // Saturate host A.
    let _a1 = sems.acquire("alpha.example.com").await;
    let _a2 = sems.acquire("alpha.example.com").await;
    let _a3 = sems.acquire("alpha.example.com").await;

    // Host B must not be blocked.
    let got_b = tokio::time::timeout(
        Duration::from_millis(50),
        sems.acquire("beta.example.com"),
    )
    .await;

    assert!(got_b.is_ok(), "request to different host was blocked by unrelated semaphore");
}

/// `acquire` must normalise the host so that `example.com`, `EXAMPLE.COM`,
/// and `Example.Com` all share the same semaphore slot.
#[tokio::test]
async fn politeness_host_key_is_case_insensitive() {
    let sems = DomainSemaphores::new(1); // 1 permit → strict

    let p1 = sems.acquire("Example.Com").await;

    let blocked = tokio::time::timeout(
        Duration::from_millis(30),
        sems.acquire("EXAMPLE.COM"),
    )
    .await;

    assert!(blocked.is_err(), "uppercase variant bypassed the semaphore");
    drop(p1);
}

// ── Pre-flight Checks ─────────────────────────────────────────────────────────

#[test]
fn preflight_accepts_html_response() {
    let result = PreflightCheck::validate(
        Some("text/html; charset=utf-8"),
        Some(1024 * 100), // 100 KB
    );
    assert!(result.is_ok());
}

#[test]
fn preflight_accepts_missing_content_length() {
    // Chunked / streaming responses have no Content-Length — must be allowed.
    let result = PreflightCheck::validate(Some("text/html"), None);
    assert!(result.is_ok());
}

#[test]
fn preflight_rejects_pdf() {
    let err = PreflightCheck::validate(Some("application/pdf"), Some(512))
        .unwrap_err();
    assert!(matches!(err, PreflightError::NonTextMime(_)));
}

#[test]
fn preflight_rejects_zip() {
    let err = PreflightCheck::validate(Some("application/zip"), Some(512))
        .unwrap_err();
    assert!(matches!(err, PreflightError::NonTextMime(_)));
}

#[test]
fn preflight_rejects_video() {
    let err = PreflightCheck::validate(Some("video/mp4"), Some(512))
        .unwrap_err();
    assert!(matches!(err, PreflightError::NonTextMime(_)));
}

#[test]
fn preflight_rejects_response_over_5mb() {
    const FIVE_MB: u64 = 5 * 1024 * 1024;
    let err = PreflightCheck::validate(
        Some("text/html"),
        Some(FIVE_MB + 1),
    )
    .unwrap_err();
    assert!(matches!(err, PreflightError::TooLarge(_)));
}

#[test]
fn preflight_accepts_response_exactly_at_5mb() {
    const FIVE_MB: u64 = 5 * 1024 * 1024;
    let result = PreflightCheck::validate(Some("text/html"), Some(FIVE_MB));
    assert!(result.is_ok());
}

#[test]
fn preflight_accepts_plain_text() {
    let result = PreflightCheck::validate(Some("text/plain"), Some(1024));
    assert!(result.is_ok());
}

#[test]
fn preflight_rejects_missing_content_type() {
    // No Content-Type at all — we can't safely parse it.
    let err = PreflightCheck::validate(None, None).unwrap_err();
    assert!(matches!(err, PreflightError::MissingContentType));
}
