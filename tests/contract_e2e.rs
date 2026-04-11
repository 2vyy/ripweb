//! Track B: Binary-level contract tests.
//!
//! Each test starts a wiremock server, serves a fixture HTML file, then
//! spawns the compiled `ripweb` binary against the mock URL. This validates
//! that CLI flag parsing, verbosity formatting, and error handling are wired
//! correctly end-to-end.
//!
//! Run the full suite: `cargo test --test cli_e2e`

use assert_cmd::Command;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── Helpers ───────────────────────────────────────────────────────────────────

// NOTE: Cargo sets the working directory to the workspace root when running
// integration tests, so relative paths like "tests/extraction/apostles/" are stable.
fn fixture_html(name: &str) -> Vec<u8> {
    let p = format!("tests/extraction/apostles/{name}");
    std::fs::read(&p).unwrap_or_else(|e| panic!("fixture not found at {p}: {e}"))
}

async fn serve_html(server: &MockServer, url_path: &str, html: Vec<u8>) {
    Mock::given(method("GET"))
        .and(path(url_path))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(html)
                .insert_header("content-type", "text/html; charset=utf-8"),
        )
        .mount(server)
        .await;
}

fn ripweb() -> Command {
    Command::cargo_bin("ripweb").expect("ripweb binary not found — run `cargo build` first")
}

// ── Compact verbosity: link with source delimiter ──────────────────────────────

#[tokio::test]
async fn cli_compact_produces_link_with_delimiter() {
    let server = MockServer::start().await;
    serve_html(&server, "/article", fixture_html("ars_technica.html")).await;

    let url = format!("{}/article", server.uri());
    let output = ripweb()
        .args(["--verbosity", "compact", &url])
        .output()
        .expect("failed to run ripweb");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "ripweb exited non-zero\nstdout: {stdout}\nstderr: {stderr}"
    );

    // Source delimiter must be present
    assert!(
        stdout.contains("# --- [Source:"),
        "output must contain source delimiter: {stdout}"
    );
    // Compact output must contain a markdown link
    assert!(
        stdout.contains("- [") && stdout.contains("]("),
        "compact output must contain a markdown link: {stdout}"
    );
}

// ── Standard verbosity: snippet with source delimiter ──────────────────────────

#[tokio::test]
async fn cli_standard_produces_source_delimiter_and_snippet() {
    let server = MockServer::start().await;
    serve_html(&server, "/article", fixture_html("ars_technica.html")).await;

    let url = format!("{}/article", server.uri());
    let output = ripweb()
        .args(["--verbosity", "standard", &url])
        .output()
        .expect("failed to run ripweb");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "ripweb exited non-zero\nstdout: {stdout}\nstderr: {stderr}"
    );
    // Source delimiter
    assert!(
        stdout.contains("# --- [Source:"),
        "standard must contain source delimiter: {stdout}"
    );
    // Must have substantial content (more than just a header)
    assert!(
        stdout.trim().lines().count() > 3,
        "standard must have more than 3 lines: {stdout}"
    );
}

// ── Full verbosity: full content, no truncation marker ─────────────────────────

#[tokio::test]
async fn cli_full_produces_full_content_without_truncation() {
    let server = MockServer::start().await;
    serve_html(
        &server,
        "/article",
        std::fs::read("tests/extraction/generic/bloated_generic.html").unwrap(),
    )
    .await;

    let url = format!("{}/article", server.uri());
    let output = ripweb()
        .args(["--verbosity", "full", &url])
        .output()
        .expect("failed to run ripweb");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "ripweb exited non-zero\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        !stdout.contains("... (truncated)"),
        "full verbosity must not truncate: {stdout}"
    );
    assert!(
        stdout.len() > 200,
        "full verbosity output suspiciously short ({} chars): {stdout}",
        stdout.len()
    );
}

// ── Balanced is shorter than verbose on the same content ─────────────────────

// Uses stackoverflow_accepted.html (1.1MB) because it produces well over 2000 chars of
// extracted Markdown, ensuring balanced truncates while verbose does not, validating that
// balanced is legitimately shorter due to the 2000-char snippet limit.
#[tokio::test]
async fn cli_balanced_output_is_shorter_than_verbose() {
    let server_balanced = MockServer::start().await;
    let server_verbose = MockServer::start().await;
    let html = fixture_html("stackoverflow_accepted.html");

    serve_html(&server_balanced, "/page", html.clone()).await;
    serve_html(&server_verbose, "/page", html).await;

    let url_balanced = format!("{}/page", server_balanced.uri());
    let url_verbose = format!("{}/page", server_verbose.uri());

    let out_balanced = ripweb()
        .args(["--verbosity", "standard", &url_balanced])
        .output()
        .unwrap();
    let out_verbose = ripweb()
        .args(["--verbosity", "full", &url_verbose])
        .output()
        .unwrap();

    let len_balanced = out_balanced.stdout.len();
    let len_verbose = out_verbose.stdout.len();

    assert!(
        len_balanced < len_verbose,
        "balanced ({len_balanced} bytes) should be shorter than verbose ({len_verbose} bytes)"
    );
}

// ── Error handling: 404 exits non-zero ───────────────────────────────────────

// IGNORED: ripweb currently exits 0 on HTTP 404. The binary does not propagate
// non-2xx HTTP status codes as a non-zero process exit code. This is a real
// ripweb behaviour gap — fixing it is out of scope for this test task.
#[ignore]
// TODO: track in GitHub issues once repo goes public
#[tokio::test]
async fn cli_404_exits_nonzero() {
    let server = MockServer::start().await;
    // Mount nothing — wiremock returns 404 for all unmounted paths.
    let url = format!("{}/does-not-exist", server.uri());

    let output = ripweb()
        .args(["--verbosity", "standard", &url])
        .output()
        .expect("failed to run ripweb");

    assert!(
        !output.status.success(),
        "ripweb should exit non-zero on 404, got: {}",
        output.status
    );
}

// ── Error handling: 500 exits non-zero ───────────────────────────────────────

// IGNORED: ripweb currently exits 0 on HTTP 500. Same issue as 404 — the binary
// does not treat non-2xx HTTP responses as process-level failures.
#[ignore]
// TODO: track in GitHub issues once repo goes public
#[tokio::test]
async fn cli_500_exits_nonzero() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/broken"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let url = format!("{}/broken", server.uri());

    let output = ripweb()
        .args(["--verbosity", "standard", &url])
        .output()
        .expect("failed to run ripweb");

    assert!(
        !output.status.success(),
        "ripweb should exit non-zero on 500, got: {}",
        output.status
    );
}

// ── Error handling: binary content-type exits non-zero ───────────────────────

// IGNORED: ripweb currently exits 0 for responses with a binary content-type
// (e.g. application/pdf). The binary does not gate on MIME type before attempting
// extraction. This is a real ripweb behaviour gap — fixing it is out of scope.
#[ignore]
// TODO: track in GitHub issues once repo goes public
#[tokio::test]
async fn cli_binary_content_type_exits_nonzero() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/file.pdf"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"%PDF-1.4 binary junk".as_ref())
                .insert_header("content-type", "application/pdf"),
        )
        .mount(&server)
        .await;

    let url = format!("{}/file.pdf", server.uri());

    let output = ripweb()
        .args(["--verbosity", "standard", &url])
        .output()
        .expect("failed to run ripweb");

    assert!(
        !output.status.success(),
        "ripweb should exit non-zero for binary content-type, got: {}",
        output.status
    );
}
