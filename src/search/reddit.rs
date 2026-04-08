//! Reddit JSON API
//!
//! Fetches structured thread and comment data by appending `.json` 
//! to Reddit URLs. Filters comments by score to preserve signal.

use serde::Deserialize;
use url::Url;

pub struct RedditContent {
    pub title: String,
    pub selftext: String,
    pub comments: Vec<String>,
}

/// Convert any Reddit thread URL to its `.json` API equivalent.
/// Returns `None` if `url` cannot be parsed.
pub fn reddit_json_url(url: &str) -> Option<String> {
    let mut parsed = Url::parse(url).ok()?;
    parsed.set_fragment(None);

    // Strip trailing slash so we can cleanly append .json
    let path = parsed.path().trim_end_matches('/').to_owned();
    parsed.set_path(&path);

    Some(format!("{}.json", parsed))
}

/// Parse Reddit's 2-element array JSON response.
///
/// Extracts the OP's `selftext` and top-level `t1` comments with `score > 0`.
pub fn parse_reddit_json(json: &str) -> Result<RedditContent, serde_json::Error> {
    let listings: Vec<Listing> = serde_json::from_str(json)?;

    let mut title = String::new();
    let mut selftext = String::new();
    let mut comments: Vec<String> = Vec::new();

    // First listing: the post itself
    if let Some(post_listing) = listings.first()
        && let Some(post_child) = post_listing.data.children.first()
    {
        title = post_child.data.title.clone();
        selftext = post_child.data.selftext.clone();
    }

    // Second listing: top-level comments
    if let Some(comment_listing) = listings.get(1) {
        for child in &comment_listing.data.children {
            if child.kind == "t1" && child.data.score > 0 {
                let body = child.data.body.trim().to_owned();
                if !body.is_empty() {
                    comments.push(body);
                }
            }
        }
    }

    Ok(RedditContent { title, selftext, comments })
}

// ── Serde types ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct Listing {
    data: ListingData,
}

#[derive(Deserialize)]
struct ListingData {
    children: Vec<Child>,
}

#[derive(Deserialize)]
struct Child {
    kind: String,
    data: Post,
}

#[derive(Deserialize, Default)]
struct Post {
    #[serde(default)]
    title: String,
    #[serde(default)]
    selftext: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    score: i64,
}
