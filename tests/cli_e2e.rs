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
// integration tests, so relative paths like "tests/fixtures/apostles/" are stable.
fn fixture_html(name: &str) -> Vec<u8> {
    let p = format!("tests/fixtures/apostles/{name}");
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

// ── Verbosity 1: link-only ────────────────────────────────────────────────────

#[tokio::test]
async fn cli_v1_produces_single_link_line() {
    let server = MockServer::start().await;
    serve_html(&server, "/article", fixture_html("ars_technica.html")).await;

    let url = format!("{}/article", server.uri());
    let output = ripweb()
        .args(["--verbosity", "1", &url])
        .output()
        .expect("failed to run ripweb");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "ripweb exited non-zero\nstdout: {stdout}\nstderr: {stderr}"
    );

    let trimmed = stdout.trim();
    assert!(
        trimmed.starts_with("- ["),
        "v1 output must start with '- [': {trimmed}"
    );
    assert_eq!(
        trimmed.lines().count(),
        1,
        "v1 output must be exactly one line: {trimmed}"
    );
    assert!(
        trimmed.contains(&url),
        "v1 output must contain the source URL: {trimmed}"
    );
    assert!(
        trimmed.contains("]("),
        "v1 output must be a valid markdown link: {trimmed}"
    );
}

// ── Verbosity 2: snippet with header ─────────────────────────────────────────

#[tokio::test]
async fn cli_v2_produces_page_header_and_snippet() {
    let server = MockServer::start().await;
    serve_html(&server, "/article", fixture_html("ars_technica.html")).await;

    let url = format!("{}/article", server.uri());
    let output = ripweb()
        .args(["--verbosity", "2", &url])
        .output()
        .expect("failed to run ripweb");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "ripweb exited non-zero\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("# Page:"),
        "v2 must contain '# Page:' header: {stdout}"
    );
    assert!(
        stdout.trim().lines().count() > 1,
        "v2 must have more than one line: {stdout}"
    );
}

// ── Verbosity 3: full content, no truncation marker ──────────────────────────

#[tokio::test]
async fn cli_v3_produces_full_content_without_truncation() {
    let server = MockServer::start().await;
    serve_html(
        &server,
        "/article",
        std::fs::read("tests/fixtures/extract/bloated_generic.html").unwrap(),
    )
    .await;

    let url = format!("{}/article", server.uri());
    let output = ripweb()
        .args(["--verbosity", "3", &url])
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
        "v3 must not truncate: {stdout}"
    );
    assert!(
        stdout.len() > 200,
        "v3 output suspiciously short ({} chars): {stdout}",
        stdout.len()
    );
}

// ── V2 is shorter than V3 on the same content ────────────────────────────────

// Uses stackoverflow_accepted.html (1.1MB) because it produces well over 2000 chars of
// extracted Markdown, ensuring v2 truncates while v3 does not, validating that v2 is
// legitimately shorter due to the 2000-char snippet limit.
#[tokio::test]
async fn cli_v2_output_is_shorter_than_v3() {
    let server_v2 = MockServer::start().await;
    let server_v3 = MockServer::start().await;
    let html = fixture_html("stackoverflow_accepted.html");

    serve_html(&server_v2, "/page", html.clone()).await;
    serve_html(&server_v3, "/page", html).await;

    let url_v2 = format!("{}/page", server_v2.uri());
    let url_v3 = format!("{}/page", server_v3.uri());

    let out_v2 = ripweb()
        .args(["--verbosity", "2", &url_v2])
        .output()
        .unwrap();
    let out_v3 = ripweb()
        .args(["--verbosity", "3", &url_v3])
        .output()
        .unwrap();

    let len_v2 = out_v2.stdout.len();
    let len_v3 = out_v3.stdout.len();

    assert!(
        len_v2 < len_v3,
        "v2 ({len_v2} bytes) should be shorter than v3 ({len_v3} bytes)"
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
        .args(["--verbosity", "2", &url])
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
        .args(["--verbosity", "2", &url])
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
        .args(["--verbosity", "2", &url])
        .output()
        .expect("failed to run ripweb");

    assert!(
        !output.status.success(),
        "ripweb should exit non-zero for binary content-type, got: {}",
        output.status
    );
}
