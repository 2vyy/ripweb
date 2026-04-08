use regex::Regex;
use ripweb::extract::{Extractor, web::WebExtractor};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
struct QualityMetrics {
    compression_ratio: f64,
    noise_to_signal: f64,
    link_saturation: f64,
    code_to_text: f64,
}

fn calculate_metrics(raw_html: &[u8], extracted_md: &str) -> QualityMetrics {
    let raw_len = raw_html.len() as f64;
    let md_len = extracted_md.len() as f64;

    // 1. Noise-to-Signal: len(extracted_markdown) / len(total_text_in_html)
    // Extract raw text from HTML safely
    let html_str = String::from_utf8_lossy(raw_html);
    let dom = tl::parse(&html_str, tl::ParserOptions::default()).unwrap();
    let parser = dom.parser();

    // We compute raw text length from the body to avoid CSS/JS noise where possible
    let html_text_len = dom
        .query_selector("body")
        .and_then(|mut nodes| nodes.next().and_then(|node| node.get(parser)))
        .map(|body| body.inner_text(parser).len() as f64)
        .unwrap_or(raw_len); // Fail safe

    // 2. Link Saturation: char_in_links / total_chars
    let link_re = Regex::new(r"\[([^\]]+)\]\([^\)]+\)").unwrap();
    let chars_in_links: usize = link_re
        .captures_iter(extracted_md)
        .map(|c| c.get(0).unwrap().as_str().len())
        .sum();

    // 3. Code-to-Text: char_in_fences / total_chars
    let code_block_re = Regex::new(r"(?s)```.*?```").unwrap();
    let chars_in_code_blocks: usize = code_block_re
        .captures_iter(extracted_md)
        .map(|c| c.get(0).unwrap().as_str().len())
        .sum();

    // Limit decimal places to 3 for stability in snapshot tests
    let round3 = |val: f64| (val * 1000.0).round() / 1000.0;

    QualityMetrics {
        compression_ratio: if raw_len > 0.0 {
            round3(md_len / raw_len)
        } else {
            0.0
        },
        noise_to_signal: if html_text_len > 0.0 {
            round3(md_len / html_text_len)
        } else {
            0.0
        },
        link_saturation: if md_len > 0.0 {
            round3(chars_in_links as f64 / md_len)
        } else {
            0.0
        },
        code_to_text: if md_len > 0.0 {
            round3(chars_in_code_blocks as f64 / md_len)
        } else {
            0.0
        },
    }
}

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
