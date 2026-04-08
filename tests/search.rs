use ripweb::search::{
    duckduckgo::{ddg_search_url, parse_ddg_html},
    hackernews::{hn_api_url, parse_hn_json},
    reddit::{parse_reddit_json, reddit_json_url},
};

// ── Fixtures ──────────────────────────────────────────────────────────────────

const DDG_FIXTURE: &str = include_str!("fixtures/search/ddg_results.html");

const HN_FIXTURE: &str = r#"{
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

const REDDIT_FIXTURE: &str = r#"[
  {
    "kind": "Listing",
    "data": {
      "children": [{
        "kind": "t3",
        "data": {
          "title": "Why is Rust so fast?",
          "selftext": "I've been reading about zero-cost abstractions and I'm impressed.",
          "score": 250
        }
      }]
    }
  },
  {
    "kind": "Listing",
    "data": {
      "children": [
        {
          "kind": "t1",
          "data": {
            "body": "Because it compiles to native code with no GC pauses.",
            "score": 180
          }
        },
        {
          "kind": "t1",
          "data": {
            "body": "The borrow checker ensures memory safety at compile time.",
            "score": 95
          }
        },
        {
          "kind": "t1",
          "data": {
            "body": "This is a low-quality reply.",
            "score": 0
          }
        },
        {
          "kind": "t1",
          "data": {
            "body": "Rust is actually slow (downvoted spam).",
            "score": -12
          }
        },
        {
          "kind": "more",
          "data": {
            "body": "",
            "score": 0
          }
        }
      ]
    }
  }
]"#;

// ── DuckDuckGo ────────────────────────────────────────────────────────────────

#[test]
fn ddg_url_points_to_html_endpoint() {
    let url = ddg_search_url("rust async");
    assert_eq!(url.host_str(), Some("html.duckduckgo.com"));
    assert_eq!(url.path(), "/html/");
}

#[test]
fn ddg_url_encodes_query_in_q_param() {
    let url = ddg_search_url("rust async traits");
    let q: Vec<_> = url
        .query_pairs()
        .filter(|(k, _)| k == "q")
        .map(|(_, v)| v.into_owned())
        .collect();
    assert_eq!(q, vec!["rust async traits"]);
}

#[test]
fn ddg_parse_extracts_decoded_urls_from_fixture() {
    let urls = parse_ddg_html(DDG_FIXTURE, 10);
    assert!(
        urls.iter().any(|u| u.url.contains("doc.rust-lang.org")),
        "async book URL missing: {:?}",
        urls
    );
    assert!(
        urls.iter().any(|u| u.url.contains("tokio.rs")),
        "tokio URL missing: {:?}",
        urls
    );
}

#[test]
fn ddg_parse_respects_limit() {
    let urls = parse_ddg_html(DDG_FIXTURE, 2);
    assert_eq!(urls.len(), 2, "expected exactly 2 results, got: {:?}", urls);
}

#[test]
fn ddg_parse_returns_decoded_urls_not_ddg_redirects() {
    let urls = parse_ddg_html(DDG_FIXTURE, 10);
    for result in &urls {
        assert!(
            !result.url.contains("duckduckgo.com/l/"),
            "DDG redirect URL leaked into results: {}",
            result.url
        );
        assert!(
            result.url.starts_with("http://") || result.url.starts_with("https://"),
            "result is not an absolute URL: {}",
            result.url
        );
    }
}

#[test]
fn ddg_parse_handles_limit_larger_than_results() {
    let urls = parse_ddg_html(DDG_FIXTURE, 100);
    assert_eq!(urls.len(), 4, "fixture has 4 results");
}

#[test]
fn ddg_parse_returns_empty_on_no_results() {
    let urls = parse_ddg_html("<html><body><p>No results found.</p></body></html>", 10);
    assert!(urls.is_empty());
}

// ── HackerNews ────────────────────────────────────────────────────────────────

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

#[test]
fn hn_parse_extracts_title() {
    let content = parse_hn_json(HN_FIXTURE).unwrap();
    assert_eq!(
        content.title,
        "Show HN: ripweb – fast local CLI for LLM context"
    );
}

#[test]
fn hn_parse_extracts_op_text_and_strips_html() {
    let content = parse_hn_json(HN_FIXTURE).unwrap();
    let text = content.text.unwrap();
    assert!(
        text.contains("scrapes web content"),
        "OP text missing: {text}"
    );
    assert!(
        !text.contains("<p>"),
        "HTML tags must be stripped from text: {text}"
    );
}

#[test]
fn hn_parse_collects_non_null_comment_texts() {
    let content = parse_hn_json(HN_FIXTURE).unwrap();
    let joined = content.comments.join(" ");
    assert!(joined.contains("token compression"), "comment 1 missing");
    assert!(joined.contains("SPA fallback"), "comment 3 missing");
}

#[test]
fn hn_parse_skips_null_text_children() {
    let content = parse_hn_json(HN_FIXTURE).unwrap();
    assert_eq!(content.comments.len(), 2, "expected 2 non-null comments");
}

#[test]
fn hn_parse_strips_html_from_comment_text() {
    let content = parse_hn_json(HN_FIXTURE).unwrap();
    for comment in &content.comments {
        assert!(
            !comment.contains("<p>"),
            "HTML tag found in comment: {comment}"
        );
    }
}

// ── Reddit ────────────────────────────────────────────────────────────────────

#[test]
fn reddit_json_url_appends_dot_json() {
    let url = reddit_json_url("https://www.reddit.com/r/rust/comments/abc/title/").unwrap();
    assert!(url.ends_with(".json"), "got: {url}");
}

#[test]
fn reddit_json_url_strips_fragment_before_appending() {
    let url = reddit_json_url("https://www.reddit.com/r/rust/comments/abc/title/#comment").unwrap();
    assert!(!url.contains('#'), "fragment leaked into JSON url: {url}");
    assert!(url.ends_with(".json"), "got: {url}");
}

#[test]
fn reddit_json_url_handles_url_without_trailing_slash() {
    let url = reddit_json_url("https://www.reddit.com/r/rust/comments/abc/title").unwrap();
    assert!(url.ends_with(".json"), "got: {url}");
}

#[test]
fn reddit_parse_extracts_title() {
    let content = parse_reddit_json(REDDIT_FIXTURE).unwrap();
    assert_eq!(content.title, "Why is Rust so fast?");
}

#[test]
fn reddit_parse_extracts_op_selftext() {
    let content = parse_reddit_json(REDDIT_FIXTURE).unwrap();
    assert!(
        content.selftext.contains("zero-cost abstractions"),
        "selftext: {}",
        content.selftext
    );
}

#[test]
fn reddit_parse_includes_positive_score_comments() {
    let content = parse_reddit_json(REDDIT_FIXTURE).unwrap();
    let joined = content.comments.join(" ");
    assert!(joined.contains("native code"), "comment 1 missing");
    assert!(joined.contains("borrow checker"), "comment 2 missing");
}

#[test]
fn reddit_parse_drops_zero_score_comments() {
    let content = parse_reddit_json(REDDIT_FIXTURE).unwrap();
    let joined = content.comments.join(" ");
    assert!(
        !joined.contains("low-quality reply"),
        "score=0 comment must be dropped"
    );
}

#[test]
fn reddit_parse_drops_negative_score_comments() {
    let content = parse_reddit_json(REDDIT_FIXTURE).unwrap();
    let joined = content.comments.join(" ");
    assert!(
        !joined.contains("downvoted spam"),
        "score<0 comment must be dropped"
    );
}

#[test]
fn reddit_parse_drops_non_t1_children() {
    let content = parse_reddit_json(REDDIT_FIXTURE).unwrap();
    assert_eq!(content.comments.len(), 2, "only 2 t1 comments with score>0");
}

#[test]
fn reddit_parse_handles_empty_selftext_gracefully() {
    let json = r#"[
      {"kind":"Listing","data":{"children":[{"kind":"t3","data":{"title":"Link post","selftext":"","score":10}}]}},
      {"kind":"Listing","data":{"children":[]}}
    ]"#;
    let content = parse_reddit_json(json).unwrap();
    assert_eq!(content.selftext, "");
    assert!(content.comments.is_empty());
}
