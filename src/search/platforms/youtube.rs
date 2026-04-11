//! YouTube Metadata & Transcripts
//!
//! Extracts video metadata (Title, Channel) and localized timed-text
//! transcripts for deep content retrieval.

use serde::Deserialize;
use url::Url;

/// Extract the YouTube video ID from supported URL formats:
/// - `youtube.com/watch?v=ID`
/// - `youtu.be/ID`
/// - `youtube.com/shorts/ID`
pub fn youtube_video_id(url: &Url) -> Option<String> {
    match url.host_str() {
        Some("youtu.be") => {
            // Path is just /ID
            url.path_segments()?
                .next()
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        }
        Some("www.youtube.com") | Some("youtube.com") => {
            // /watch?v=ID  or  /shorts/ID
            if let Some(v) = url
                .query_pairs()
                .find(|(k, _)| k == "v")
                .map(|(_, v)| v.into_owned())
            {
                return Some(v);
            }
            let mut segs = url.path_segments()?.filter(|s| !s.is_empty());
            match segs.next()? {
                "shorts" | "embed" | "v" => segs.next().map(str::to_owned),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Build the YouTube oEmbed endpoint URL.
pub fn youtube_oembed_url(video_url: &str) -> String {
    let encoded: String = url::form_urlencoded::byte_serialize(video_url.as_bytes()).collect();
    format!("https://www.youtube.com/oembed?url={}&format=json", encoded)
}

/// Parse oEmbed JSON response into a minimal Markdown header.
pub fn parse_youtube_oembed(json: &str) -> Result<YoutubeOembed, serde_json::Error> {
    serde_json::from_str(json)
}

/// Extract the caption track base URL from an embedded YouTube watch-page body.
///
/// The watch page embeds JavaScript containing a `captionTracks` array inside
/// the `ytInitialPlayerResponse` object. We locate the first English (or any)
/// `baseUrl` without running JS.
pub fn extract_caption_url(page_body: &str) -> Option<String> {
    // The blob looks like: "captionTracks":[{"baseUrl":"https://...","name":...}]
    let marker = "\"captionTracks\":";
    let start = page_body.find(marker)? + marker.len();
    let slice = &page_body[start..];

    // Find the `baseUrl` value within the following bracket block
    let base_marker = "\"baseUrl\":\"";
    let base_start = slice.find(base_marker)? + base_marker.len();
    let base_slice = &slice[base_start..];
    let end = base_slice.find('"')?;
    let raw_url = &base_slice[..end];

    // The URL has JSON-escaped unicode sequences (\u0026 → &), unescape them
    let url = raw_url.replace("\\u0026", "&");
    Some(url)
}

/// Parse the timedtext caption XML into a clean transcript string.
///
/// Format: `<text start="12.5" dur="1.8">Hello &amp; world</text>`
pub fn parse_caption_xml(xml: &str) -> String {
    let mut lines: Vec<(f64, String)> = Vec::new();

    for tag in xml.split("<text ") {
        // Extract `start` attribute
        let start_sec = tag.find("start=\"").and_then(|i| {
            let s = &tag[i + 7..];
            s.find('"').and_then(|e| s[..e].parse::<f64>().ok())
        });

        // Extract text content between `>` and `</text>`
        let text = tag
            .find('>')
            .and_then(|i| tag[i + 1..].find("</text>").map(|e| &tag[i + 1..i + 1 + e]));

        if let (Some(sec), Some(raw)) = (start_sec, text) {
            let clean = decode_xml_entities(raw.trim());
            if !clean.is_empty() {
                lines.push((sec, clean));
            }
        }
    }

    lines
        .into_iter()
        .map(|(sec, text)| {
            let h = (sec as u64) / 3600;
            let m = ((sec as u64) % 3600) / 60;
            let s = (sec as u64) % 60;
            if h > 0 {
                format!("**[{h:02}:{m:02}:{s:02}]** {text}")
            } else {
                format!("**[{m:02}:{s:02}]** {text}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format oEmbed metadata + optional transcript into Markdown based on verbosity.
pub fn format_youtube_content(
    oembed: &YoutubeOembed,
    transcript: Option<&str>,
    mode: crate::verbosity::Verbosity,
) -> String {
    let mut out = format!(
        "# {}\n\n**Channel:** [{}]({})\n",
        oembed.title, oembed.author_name, oembed.author_url
    );

    if let Some(tx) = transcript
        && !tx.is_empty()
    {
        match mode.density_tier() {
            1 => {} // V1: No transcript
            2 => {
                // V2: Truncate transcript to ~500 chars roughly
                out.push_str("\n## Transcript Snippet\n\n");
                let snippet: String = tx.chars().take(500).collect();
                out.push_str(&snippet);
                if tx.len() > 500 {
                    out.push_str("... (truncated)");
                }
            }
            _ => {
                // V3: Full transcript
                out.push_str("\n## Transcript\n\n");
                out.push_str(tx);
            }
        }
    }
    out
}

// ── XML entity decoder ────────────────────────────────────────────────────────

fn decode_xml_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
}

// ── Serde types ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct YoutubeOembed {
    pub title: String,
    pub author_name: String,
    pub author_url: String,
    #[allow(dead_code)]
    pub thumbnail_url: Option<String>,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_id_from_watch_url() {
        let url = Url::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
        assert_eq!(youtube_video_id(&url).as_deref(), Some("dQw4w9WgXcQ"));
    }

    #[test]
    fn video_id_from_short_url() {
        let url = Url::parse("https://youtu.be/dQw4w9WgXcQ").unwrap();
        assert_eq!(youtube_video_id(&url).as_deref(), Some("dQw4w9WgXcQ"));
    }

    #[test]
    fn video_id_from_shorts_url() {
        let url = Url::parse("https://www.youtube.com/shorts/abc123").unwrap();
        assert_eq!(youtube_video_id(&url).as_deref(), Some("abc123"));
    }

    #[test]
    fn video_id_returns_none_for_channel() {
        let url = Url::parse("https://www.youtube.com/@rustlang").unwrap();
        assert_eq!(youtube_video_id(&url), None);
    }

    #[test]
    fn parse_caption_xml_formats_timestamps() {
        let xml = r#"<?xml version="1.0"?>
<transcript>
<text start="0" dur="2.5">Hello world</text>
<text start="75" dur="1.8">How are you</text>
<text start="3661" dur="1.0">Long video</text>
</transcript>"#;
        let result = parse_caption_xml(xml);
        assert!(result.contains("**[00:00]** Hello world"));
        assert!(result.contains("**[01:15]** How are you"));
        assert!(result.contains("**[01:01:01]** Long video"));
    }

    #[test]
    fn parse_caption_xml_decodes_entities() {
        let xml = r#"<text start="1" dur="1">Hello &amp; world &lt;3&gt;</text>"#;
        let result = parse_caption_xml(xml);
        assert!(result.contains("Hello & world <3>"));
    }

    #[test]
    fn extract_caption_url_finds_base_url() {
        let page = r#"{"captionTracks":[{"baseUrl":"https://example.com/api?v=1\u00261=en","name":{"simpleText":"English"}}]}"#;
        let url = extract_caption_url(page).unwrap();
        assert!(url.contains("https://example.com/api?v=1&1=en"));
    }

    #[test]
    fn parse_oembed_parses_json() {
        let json = r#"{
            "title": "Never Gonna Give You Up",
            "author_name": "RickAstleyVEVO",
            "author_url": "https://www.youtube.com/@RickAstleyVEVO",
            "thumbnail_url": "https://i.ytimg.com/vi/dQw4w9WgXcQ/hqdefault.jpg"
        }"#;
        let oembed = parse_youtube_oembed(json).unwrap();
        assert_eq!(oembed.title, "Never Gonna Give You Up");
        assert_eq!(oembed.author_name, "RickAstleyVEVO");
    }

    #[test]
    fn format_youtube_content_with_transcript() {
        let oembed = YoutubeOembed {
            title: "Test Video".into(),
            author_name: "Test Channel".into(),
            author_url: "https://youtube.com/@test".into(),
            thumbnail_url: None,
        };
        let out = format_youtube_content(
            &oembed,
            Some("**[00:00]** Hello"),
            crate::verbosity::Verbosity::Full,
        );
        assert!(out.starts_with("# Test Video"));
        assert!(out.contains("## Transcript"));
        assert!(out.contains("**[00:00]** Hello"));
    }

    #[test]
    fn format_youtube_content_without_transcript() {
        let oembed = YoutubeOembed {
            title: "No Captions".into(),
            author_name: "Channel".into(),
            author_url: "https://youtube.com/@c".into(),
            thumbnail_url: None,
        };
        let out = format_youtube_content(&oembed, None, crate::verbosity::Verbosity::Full);
        assert!(!out.contains("## Transcript"));
    }
}
