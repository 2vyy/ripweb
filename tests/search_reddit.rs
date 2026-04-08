use ripweb::search::reddit::{parse_reddit_json, reddit_json_url};

const FIXTURE: &str = r#"[
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

// ── URL transformation ────────────────────────────────────────────────────────

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

// ── JSON parsing ──────────────────────────────────────────────────────────────

#[test]
fn parse_extracts_title() {
    let content = parse_reddit_json(FIXTURE).unwrap();
    assert_eq!(content.title, "Why is Rust so fast?");
}

#[test]
fn parse_extracts_op_selftext() {
    let content = parse_reddit_json(FIXTURE).unwrap();
    assert!(
        content.selftext.contains("zero-cost abstractions"),
        "selftext: {}",
        content.selftext
    );
}

#[test]
fn parse_includes_positive_score_comments() {
    let content = parse_reddit_json(FIXTURE).unwrap();
    let joined = content.comments.join(" ");
    assert!(joined.contains("native code"), "comment 1 missing");
    assert!(joined.contains("borrow checker"), "comment 2 missing");
}

#[test]
fn parse_drops_zero_score_comments() {
    let content = parse_reddit_json(FIXTURE).unwrap();
    let joined = content.comments.join(" ");
    assert!(!joined.contains("low-quality reply"), "score=0 comment must be dropped");
}

#[test]
fn parse_drops_negative_score_comments() {
    let content = parse_reddit_json(FIXTURE).unwrap();
    let joined = content.comments.join(" ");
    assert!(!joined.contains("downvoted spam"), "score<0 comment must be dropped");
}

#[test]
fn parse_drops_non_t1_children() {
    // The "more" kind entry must not appear in comments.
    let content = parse_reddit_json(FIXTURE).unwrap();
    assert_eq!(content.comments.len(), 2, "only 2 t1 comments with score>0");
}

#[test]
fn parse_handles_empty_selftext_gracefully() {
    let json = r#"[
      {"kind":"Listing","data":{"children":[{"kind":"t3","data":{"title":"Link post","selftext":"","score":10}}]}},
      {"kind":"Listing","data":{"children":[]}}
    ]"#;
    let content = parse_reddit_json(json).unwrap();
    assert_eq!(content.selftext, "");
    assert!(content.comments.is_empty());
}
