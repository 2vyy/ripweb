use ripweb::search::hackernews::{hn_api_url, parse_hn_json};

const FIXTURE: &str = r#"{
  "id": 12345,
  "title": "Show HN: ripweb – fast local CLI for LLM context",
  "text": "<p>I built a tool that scrapes web content efficiently for LLM context windows.</p>",
  "author": "devuser",
  "children": [
    {
      "id": 12346,
      "text": "<p>This is really useful! Great work on the token compression.</p>",
      "author": "commenter1"
    },
    {
      "id": 12347,
      "text": null,
      "author": "commenter2"
    },
    {
      "id": 12348,
      "text": "<p>How does the SPA fallback compare to Puppeteer?</p>",
      "author": "commenter3"
    }
  ]
}"#;

// ── URL construction ──────────────────────────────────────────────────────────

#[test]
fn hn_api_url_points_to_algolia() {
    let url = hn_api_url("12345");
    assert_eq!(url.host_str(), Some("hn.algolia.com"), "host: {url}");
}

#[test]
fn hn_api_url_includes_item_id_in_path() {
    let url = hn_api_url("99999");
    assert!(url.path().contains("99999"), "path: {}", url.path());
}

#[test]
fn hn_api_url_uses_v1_items_endpoint() {
    let url = hn_api_url("12345");
    assert!(
        url.path().starts_with("/api/v1/items/"),
        "path: {}",
        url.path()
    );
}

// ── JSON parsing ──────────────────────────────────────────────────────────────

#[test]
fn parse_extracts_title() {
    let content = parse_hn_json(FIXTURE).unwrap();
    assert_eq!(content.title, "Show HN: ripweb – fast local CLI for LLM context");
}

#[test]
fn parse_extracts_op_text_and_strips_html() {
    let content = parse_hn_json(FIXTURE).unwrap();
    let text = content.text.unwrap();
    assert!(
        text.contains("scrapes web content"),
        "OP text missing: {text}"
    );
    assert!(!text.contains("<p>"), "HTML tags must be stripped from text: {text}");
}

#[test]
fn parse_collects_non_null_comment_texts() {
    let content = parse_hn_json(FIXTURE).unwrap();
    let joined = content.comments.join(" ");
    assert!(joined.contains("token compression"), "comment 1 missing");
    assert!(joined.contains("SPA fallback"), "comment 3 missing");
}

#[test]
fn parse_skips_null_text_children() {
    let content = parse_hn_json(FIXTURE).unwrap();
    // commenter2 has null text — must not appear as an empty entry
    assert_eq!(content.comments.len(), 2, "expected 2 non-null comments");
}

#[test]
fn parse_strips_html_from_comment_text() {
    let content = parse_hn_json(FIXTURE).unwrap();
    for comment in &content.comments {
        assert!(!comment.contains("<p>"), "HTML tag found in comment: {comment}");
    }
}
