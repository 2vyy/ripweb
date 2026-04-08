#[allow(dead_code)]
mod common;
use common::metrics::calculate_metrics;
use ripweb::extract::{Extractor, web::WebExtractor};

// ── HTML Apostles (WebExtractor) ──────────────────────────────────────────────

#[test]
fn apostle_snapshot_stackoverflow_accepted() {
    let raw = include_bytes!("fixtures/apostles/stackoverflow_accepted.html");
    let md = WebExtractor::extract(raw, Some("text/html")).unwrap_or_default();
    insta::assert_snapshot!(md);
}

#[test]
fn apostle_snapshot_github_issue() {
    let raw = include_bytes!("fixtures/apostles/github_issue.html");
    let md = WebExtractor::extract(raw, Some("text/html")).unwrap_or_default();
    insta::assert_snapshot!(md);
}

#[test]
fn apostle_snapshot_rust_docs_axum() {
    let raw = include_bytes!("fixtures/apostles/rust_docs_axum.html");
    let md = WebExtractor::extract(raw, Some("text/html")).unwrap_or_default();
    insta::assert_snapshot!(md);
}

#[test]
fn apostle_snapshot_ars_technica() {
    let raw = include_bytes!("fixtures/apostles/ars_technica.html");
    let md = WebExtractor::extract(raw, Some("text/html")).unwrap_or_default();
    insta::assert_snapshot!(md);
}

#[test]
fn apostle_snapshot_blog_ghost() {
    let raw = include_bytes!("fixtures/apostles/blog_ghost.html");
    let md = WebExtractor::extract(raw, Some("text/html")).unwrap_or_default();
    insta::assert_snapshot!(md);
}

#[test]
fn apostle_snapshot_amazon_product() {
    let raw = include_bytes!("fixtures/apostles/amazon_product.html");
    let md = WebExtractor::extract(raw, Some("text/html")).unwrap_or_default();
    insta::assert_snapshot!(md);
}

#[test]
fn apostle_snapshot_generic_listing() {
    let raw = include_bytes!("fixtures/apostles/generic_listing.html");
    let md = WebExtractor::extract(raw, Some("text/html")).unwrap_or_default();
    insta::assert_snapshot!(md);
}

#[test]
fn apostle_snapshot_404_page() {
    let raw = include_bytes!("fixtures/apostles/404_page.html");
    let md = WebExtractor::extract(raw, Some("text/html")).unwrap_or_default();
    insta::assert_snapshot!(md);
}

// ── JSON Apostles (Platform Parsers) ─────────────────────────────────────────

#[test]
fn apostle_snapshot_reddit_thread_v2() {
    let json = include_str!("fixtures/apostles/reddit_thread.json");
    let content =
        ripweb::search::reddit::parse_reddit_json(json).expect("reddit JSON parse failed");
    let md = ripweb::run::format_reddit(&content, 2);
    insta::assert_snapshot!(md);
}

#[test]
fn apostle_snapshot_hn_item_v2() {
    let json = include_str!("fixtures/apostles/hn_item.json");
    let content = ripweb::search::hackernews::parse_hn_json(json).expect("HN JSON parse failed");
    let md = ripweb::run::format_hn(&content, 2);
    insta::assert_snapshot!(md);
}

#[test]
fn apostle_snapshot_youtube_oembed() {
    let json = include_str!("fixtures/apostles/youtube_oembed.json");
    let oembed =
        ripweb::search::youtube::parse_youtube_oembed(json).expect("YouTube oEmbed parse failed");
    let md = ripweb::search::youtube::format_youtube_content(&oembed, None, 2);
    insta::assert_snapshot!(md);
}

#[test]
fn apostle_snapshot_wikipedia_rust_v2() {
    let json = include_str!("fixtures/apostles/wikipedia_rust.json");
    let md = ripweb::search::wikipedia::parse_wiki_summary(json, 2)
        .expect("Wikipedia JSON parse failed");
    insta::assert_snapshot!(md);
}

// ── Scoreboard (HTML apostles only) ──────────────────────────────────────────

#[test]
fn apostle_scoreboard() {
    let fixtures: &[(&str, &str)] = &[
        (
            "stackoverflow_accepted",
            "tests/fixtures/apostles/stackoverflow_accepted.html",
        ),
        ("github_issue", "tests/fixtures/apostles/github_issue.html"),
        (
            "rust_docs_axum",
            "tests/fixtures/apostles/rust_docs_axum.html",
        ),
        ("ars_technica", "tests/fixtures/apostles/ars_technica.html"),
        ("blog_ghost", "tests/fixtures/apostles/blog_ghost.html"),
        (
            "amazon_product",
            "tests/fixtures/apostles/amazon_product.html",
        ),
        (
            "generic_listing",
            "tests/fixtures/apostles/generic_listing.html",
        ),
        ("404_page", "tests/fixtures/apostles/404_page.html"),
    ];

    println!("\n{:=<76}", "");
    println!(
        "{:<30} {:>10} {:>10} {:>10} {:>10}",
        "FIXTURE", "COMPRESS", "NOISE/SIG", "LINK-SAT", "CODE"
    );
    println!("{:=<76}", "");

    let mut failures = Vec::new();

    for (name, path) in fixtures {
        let raw = match std::fs::read(path) {
            Ok(b) => b,
            Err(_) => {
                println!("{:<30} {:>10}", name, "MISSING");
                failures.push(format!("{name}: fixture file not found at {path}"));
                continue;
            }
        };
        let md = WebExtractor::extract(&raw, Some("text/html")).unwrap_or_default();
        let m = calculate_metrics(&raw, &md);
        println!(
            "{:<30} {:>10.3} {:>10.3} {:>10.3} {:>10.3}",
            name, m.compression_ratio, m.noise_to_signal, m.link_saturation, m.code_to_text
        );

        if m.noise_to_signal > 2.5 {
            failures.push(format!(
                "{name}: noise_to_signal={:.3} > 2.5",
                m.noise_to_signal
            ));
        }
    }

    println!("{:=<76}", "");
    println!("Tip: run `cargo test apostle_scoreboard -- --nocapture` to see this table.");

    assert!(
        failures.is_empty(),
        "Quality threshold exceeded:\n{}",
        failures.join("\n")
    );
}
