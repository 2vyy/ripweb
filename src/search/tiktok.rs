//! TikTok Video & Creator Meta
//!
//! Uses the TikTok public oEmbed platform to extract creator metadata,
//! titles, and video descriptions.

use serde::Deserialize;
use url::Url;

/// Returns true if this URL points to an individual TikTok video.
/// Format: `tiktok.com/@username/video/ID`
pub fn is_tiktok_video_url(url: &Url) -> bool {
    url.path_segments()
        .map(|mut s| s.any(|seg| seg == "video"))
        .unwrap_or(false)
}

/// Build the TikTok oEmbed endpoint URL.
pub fn tiktok_oembed_url(video_url: &str) -> String {
    let encoded: String = url::form_urlencoded::byte_serialize(video_url.as_bytes()).collect();
    format!("https://www.tiktok.com/oembed?url={}", encoded)
}

/// Parse TikTok oEmbed JSON and format as Markdown.
pub fn parse_tiktok_oembed(json: &str, verbosity: u8) -> Result<String, serde_json::Error> {
    let oembed: TiktokOembed = serde_json::from_str(json)?;
    Ok(format_tiktok(&oembed, verbosity))
}

fn format_tiktok(oembed: &TiktokOembed, verbosity: u8) -> String {
    let mut out = format!(
        "# {}\n\n**Creator:** [@{}]({})\n",
        oembed.title, oembed.author_unique_id, oembed.author_url
    );

    // Include the description/caption in the body if available and different from title
    if verbosity >= 2
        && let Some(description) = &oembed.description
    {
        let description = description.trim();
        if !description.is_empty() && description != oembed.title {
            out.push('\n');
            out.push_str(description);
        }
    }

    out
}

// ── Serde types ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TiktokOembed {
    title: String,
    author_unique_id: String,
    author_url: String,
    #[serde(default)]
    description: Option<String>,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_tiktok_video_url_true_for_video_path() {
        let url = Url::parse("https://www.tiktok.com/@username/video/1234567890").unwrap();
        assert!(is_tiktok_video_url(&url));
    }

    #[test]
    fn is_tiktok_video_url_false_for_profile() {
        let url = Url::parse("https://www.tiktok.com/@username").unwrap();
        assert!(!is_tiktok_video_url(&url));
    }

    #[test]
    fn parse_tiktok_oembed_formats_markdown() {
        let json = r#"{
            "title": "Check out this cool Rust trick 🦀",
            "author_unique_id": "rustacean42",
            "author_url": "https://www.tiktok.com/@rustacean42"
        }"#;
        let result = parse_tiktok_oembed(json, 2).unwrap();
        assert!(result.starts_with("# Check out this cool Rust trick"));
        assert!(result.contains("[@rustacean42]"));
    }

    #[test]
    fn parse_tiktok_oembed_includes_description_when_different() {
        let json = r#"{
            "title": "Short Title",
            "author_unique_id": "user",
            "author_url": "https://www.tiktok.com/@user",
            "description": "Longer caption text with #hashtags"
        }"#;
        let result = parse_tiktok_oembed(json, 2).unwrap();
        assert!(result.contains("Longer caption text"));
    }
}
