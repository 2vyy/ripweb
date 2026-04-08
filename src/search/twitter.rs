//! Twitter/X Tweet Extraction
//!
//! Fetches tweet text and author metadata via the public oEmbed API.

use serde::Deserialize;
use url::Url;

/// Returns true if this is an individual tweet URL (has a `/status/` segment).
/// Profile pages and other X URLs fall back to Generic since there's no keyless API.
pub fn is_tweet_url(url: &Url) -> bool {
    url.path_segments()
        .map(|mut s| s.any(|seg| seg == "status"))
        .unwrap_or(false)
}

/// Build the X/Twitter syndication oEmbed endpoint URL.
/// Works for both `twitter.com` and `x.com` URLs.
pub fn twitter_oembed_url(tweet_url: &str) -> String {
    let encoded: String = url::form_urlencoded::byte_serialize(tweet_url.as_bytes()).collect();
    format!("https://publish.twitter.com/oembed?url={}", encoded)
}

/// Parse the oEmbed JSON and format tweet text as Markdown.
///
/// The `html` field contains an embed blockquote — we strip the tags to
/// extract the tweet text.
pub fn parse_twitter_oembed(json: &str) -> Result<String, serde_json::Error> {
    let oembed: TwitterOembed = serde_json::from_str(json)?;
    Ok(format_tweet(&oembed))
}

fn format_tweet(oembed: &TwitterOembed) -> String {
    let text = strip_blockquote_html(&oembed.html);
    format!(
        "## @{}\n\n{}\n\n[Source]({})",
        oembed.author_name, text, oembed.url
    )
}

/// Strip HTML tags from the blockquote embed to get plain tweet text.
///
/// The oEmbed `html` field looks like:
/// `<blockquote>Tweet text <a href="...">link</a>...<a href="url">pic.twitter.com/...</a></blockquote>`
fn strip_blockquote_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }

    // Collapse whitespace runs and trim
    out.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned()
}

// ── Serde types ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TwitterOembed {
    html: String,
    author_name: String,
    url: String,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_tweet_url_true_for_status_path() {
        let url = Url::parse("https://x.com/rustlang/status/1234567890").unwrap();
        assert!(is_tweet_url(&url));
    }

    #[test]
    fn is_tweet_url_false_for_profile() {
        let url = Url::parse("https://x.com/rustlang").unwrap();
        assert!(!is_tweet_url(&url));
    }

    #[test]
    fn is_tweet_url_works_for_twitter_com() {
        let url = Url::parse("https://twitter.com/user/status/999").unwrap();
        assert!(is_tweet_url(&url));
    }

    #[test]
    fn strip_blockquote_html_extracts_text() {
        let html = r#"<blockquote class="twitter-tweet"><p lang="en">Hello world! Check this out</p>&mdash; User <a href="...">@user</a> <a href="https://t.co/...">pic.twitter.com/abc</a></blockquote>"#;
        let text = super::strip_blockquote_html(html);
        assert!(text.contains("Hello world!"));
        // Should not contain HTML tags
        assert!(!text.contains('<'));
    }

    #[test]
    fn parse_twitter_oembed_formats_markdown() {
        let json = r#"{
            "html": "<blockquote><p>Excited to announce Rust 2.0!</p>&mdash; The Rust Team</blockquote>",
            "author_name": "rustlang",
            "url": "https://twitter.com/rustlang/status/1234"
        }"#;
        let result = parse_twitter_oembed(json).unwrap();
        assert!(result.starts_with("## @rustlang"));
        assert!(result.contains("Excited to announce"));
        assert!(result.contains("[Source]"));
    }
}
