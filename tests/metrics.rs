use ripweb::extract::{Extractor, web::WebExtractor};

#[allow(dead_code)]
mod common;
use common::metrics::{QualityMetrics, calculate_metrics};

fn assert_fixture_metrics(fixture_path: &str) {
    let raw_html = std::fs::read(fixture_path).expect("failed to load fixture");
    let extracted_md = WebExtractor::extract(&raw_html, Some("text/html; charset=utf-8"))
        .expect("extraction failed");
    let metrics = calculate_metrics(&raw_html, &extracted_md);

    let snapshot_name = fixture_path
        .split('/')
        .next_back()
        .unwrap()
        .replace(".html", "_metrics");

    // Use insta to snapshot the metrics
    insta::assert_debug_snapshot!(snapshot_name, metrics);
}

#[test]
fn metric_baseline_docs_sidebar() {
    assert_fixture_metrics("tests/fixtures/extract/docs_sidebar.html");
}

#[test]
fn metric_baseline_listing_results() {
    assert_fixture_metrics("tests/fixtures/extract/listing_results.html");
}

#[test]
fn metric_baseline_product_detail() {
    assert_fixture_metrics("tests/fixtures/extract/product_detail.html");
}

#[test]
fn metric_baseline_forum_thread() {
    assert_fixture_metrics("tests/fixtures/extract/forum_thread.html");
}

#[test]
fn metric_baseline_bloated_generic() {
    assert_fixture_metrics("tests/fixtures/extract/bloated_generic.html");
}
