//! Generic Web Extraction
//!
//! High-level interface for extracting content from generic
//! HTML pages using heuristic-based scoring and rendering.

use super::Extractor;
use super::candidate::{extract_best_candidate, word_count};
use super::family::{PageFamily, detect_family, url_family_hint};
use super::render::{cleanup_markdown, extract_next_data};
use crate::error::RipwebError;
use encoding_rs::Encoding;

pub struct WebExtractor;

const MAX_INPUT_BYTES: usize = 5 * 1024 * 1024;

impl Extractor for WebExtractor {
    fn extract(bytes: &[u8], content_type: Option<&str>) -> Result<String, RipwebError> {
        if bytes.len() > MAX_INPUT_BYTES {
            return Err(RipwebError::InputTooLarge(bytes.len()));
        }
        let html = decode_charset(bytes, content_type);
        Ok(extract_from_str(&html, None, false))
    }
}

impl WebExtractor {
    pub fn extract_with_url(
        bytes: &[u8],
        content_type: Option<&str>,
        source_url: Option<&str>,
    ) -> Result<String, RipwebError> {
        Self::extract_with_url_options(bytes, content_type, source_url, false)
    }

    pub fn extract_with_url_options(
        bytes: &[u8],
        content_type: Option<&str>,
        source_url: Option<&str>,
        tables_priority: bool,
    ) -> Result<String, RipwebError> {
        if bytes.len() > MAX_INPUT_BYTES {
            return Err(RipwebError::InputTooLarge(bytes.len()));
        }
        let html = decode_charset(bytes, content_type);
        Ok(extract_from_str(&html, source_url, tables_priority))
    }
}

fn extract_from_str(html: &str, source_url: Option<&str>, tables_priority: bool) -> String {
    let dom = match tl::parse(html, tl::ParserOptions::default()) {
        Ok(d) => d,
        Err(_) => return String::new(),
    };

    let url_hint = source_url
        .and_then(url_family_hint)
        .unwrap_or(PageFamily::Generic);
    let family = detect_family(&dom, url_hint);

    let text = extract_best_candidate(&dom, family, tables_priority);
    let text = super::postprocess::post_process(family, &dom, text);

    if word_count(&text) < 100
        && let Some(spa) = extract_next_data(&dom).filter(|s| word_count(s) > word_count(&text))
    {
        return cleanup_markdown(&spa);
    }

    text
}

fn decode_charset(bytes: &[u8], content_type: Option<&str>) -> String {
    let encoding = content_type
        .and_then(charset_from_content_type)
        .or_else(|| charset_from_meta(bytes))
        .unwrap_or(encoding_rs::UTF_8);
    let (cow, _, _) = encoding.decode(bytes);
    cow.into_owned()
}

fn charset_from_content_type(ct: &str) -> Option<&'static Encoding> {
    ct.split(';').skip(1).find_map(|param| {
        let param = param.trim();
        let (key, val) = param.split_once('=')?;
        if key.trim().eq_ignore_ascii_case("charset") {
            Encoding::for_label(val.trim().trim_matches('"').as_bytes())
        } else {
            None
        }
    })
}

fn charset_from_meta(bytes: &[u8]) -> Option<&'static Encoding> {
    let head = &bytes[..bytes.len().min(4096)];
    let head_str = String::from_utf8_lossy(head);
    let lower = head_str.to_ascii_lowercase();
    let idx = lower.find("charset")?;
    let after = lower[idx + 7..].trim_start();
    let after = after.strip_prefix('=')?;
    let after = after.trim_start().trim_start_matches('"');
    let end = after
        .find(|c: char| c == '"' || c == '\'' || c == ';' || c.is_whitespace())
        .unwrap_or(after.len().min(32));
    let label = &after[..end];
    Encoding::for_label(label.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_count_empty() {
        assert_eq!(word_count(""), 0);
        assert_eq!(word_count("   \n\t  "), 0);
    }

    #[test]
    fn word_count_basic() {
        assert_eq!(word_count("hello world foo"), 3);
    }

    #[test]
    fn charset_from_content_type_parses_label() {
        let enc = charset_from_content_type("text/html; charset=Shift_JIS");
        assert!(enc.is_some());
        assert_eq!(enc.unwrap().name(), "Shift_JIS");
    }

    #[test]
    fn charset_from_content_type_handles_quoted_value() {
        let enc = charset_from_content_type("text/html; charset=\"utf-8\"");
        assert!(enc.is_some());
        assert_eq!(enc.unwrap().name(), "UTF-8");
    }

    #[test]
    fn charset_from_meta_detects_utf8() {
        let html = b"<head><meta charset=\"utf-8\"></head>";
        let enc = charset_from_meta(html);
        assert!(enc.is_some());
        assert_eq!(enc.unwrap().name(), "UTF-8");
    }
}
