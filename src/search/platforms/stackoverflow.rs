//! StackExchange (StackOverflow) API
//!
//! Fetches question details and answers from the SE API v2.3.
//! Sorts answers to prioritize "Accepted" and "Highly Voted" content.

use serde::Deserialize;
use url::Url;

/// Content extracted from a StackOverflow question via the SE API.
pub struct SoContent {
    pub title: String,
    pub answers: Vec<SoAnswer>,
}

pub struct SoAnswer {
    pub body_markdown: String,
    pub score: i64,
    pub is_accepted: bool,
}

/// Build the Stack Exchange API URL to fetch a question's answers.
///
/// Uses `withbody` filter so the answer body is included.
/// Responses are gzip-compressed; the HTTP client must decompress them.
pub fn so_answers_url(question_id: u64) -> Result<Url, url::ParseError> {
    Url::parse(&format!(
        "https://api.stackexchange.com/2.3/questions/{question_id}/answers\
         ?order=desc&sort=votes&site=stackoverflow&filter=withbody"
    ))
}

/// Build the SE API URL to fetch a question's details (title + body).
pub fn so_question_url(question_id: u64) -> Result<Url, url::ParseError> {
    Url::parse(&format!(
        "https://api.stackexchange.com/2.3/questions/{question_id}\
         ?site=stackoverflow&filter=withbody"
    ))
}

/// Extract the SO question ID from a URL like
/// `https://stackoverflow.com/questions/12345/some-slug`.
pub fn so_question_id_from_url(url: &Url) -> Option<u64> {
    let mut segs = url.path_segments()?.filter(|s| !s.is_empty());
    // Path: /questions/<id>[/<slug>]
    if segs.next()? != "questions" {
        return None;
    }
    segs.next()?.parse().ok()
}

/// Parse the SE API JSON for questions (to extract the title).
pub fn parse_so_question(json: &str) -> Result<String, serde_json::Error> {
    #[derive(Deserialize)]
    struct Wrapper {
        items: Vec<Question>,
    }
    #[derive(Deserialize)]
    struct Question {
        title: String,
    }
    let w: Wrapper = serde_json::from_str(json)?;
    Ok(w.items
        .into_iter()
        .next()
        .map(|q| q.title)
        .unwrap_or_default())
}

/// Parse the SE API JSON for answers into structured `SoAnswer` objects.
///
/// Answers already contain HTML in `body`; we strip tags to get plain text.
/// The SE API already sorts by votes when `sort=votes` is passed.
pub fn parse_so_answers(json: &str) -> Result<Vec<SoAnswer>, serde_json::Error> {
    #[derive(Deserialize)]
    struct Wrapper {
        items: Vec<Item>,
    }
    #[derive(Deserialize)]
    struct Item {
        body: String,
        score: i64,
        is_accepted: bool,
    }

    let w: Wrapper = serde_json::from_str(json)?;
    let mut answers: Vec<SoAnswer> = w
        .items
        .into_iter()
        .map(|item| SoAnswer {
            body_markdown: strip_html_to_markdown(&item.body),
            score: item.score,
            is_accepted: item.is_accepted,
        })
        .collect();

    // Ensure accepted answer is always first, regardless of API ordering
    answers.sort_by(|a, b| {
        b.is_accepted
            .cmp(&a.is_accepted)
            .then_with(|| b.score.cmp(&a.score))
    });

    Ok(answers)
}

/// Format the extracted SO content as clean Markdown.
pub fn format_so_content(content: &SoContent, mode: crate::verbosity::Verbosity) -> String {
    let mut out = format!("# {}\n\n## Answers\n\n", content.title);
    let limit = match mode.density_tier() {
        1 => 1,
        2 => 3,
        _ => usize::MAX,
    };
    for answer in content.answers.iter().take(limit) {
        let header = if answer.is_accepted {
            format!("### ✅ Accepted Answer [Score: {}]\n\n", answer.score)
        } else {
            format!("### [Score: {}]\n\n", answer.score)
        };
        out.push_str(&header);
        out.push_str(&answer.body_markdown);
        out.push_str("\n\n---\n\n");
    }
    out
}

/// Strip HTML tags to produce rough Markdown.
///
/// SO answer bodies are HTML; we do a lightweight conversion:
/// - `<code>` → backtick spans
/// - `<pre><code>` → fenced blocks
/// - `<p>` → paragraph breaks
/// - Everything else → strip the tag, keep text
fn strip_html_to_markdown(html: &str) -> String {
    let Ok(dom) = tl::parse(html, tl::ParserOptions::default()) else {
        return html.to_owned();
    };
    let parser = dom.parser();
    let mut out = String::new();
    render_node_to_md(dom.children().iter().copied(), parser, &mut out);
    out.trim().to_owned()
}

fn render_node_to_md(
    iter: impl Iterator<Item = tl::NodeHandle>,
    parser: &tl::Parser,
    out: &mut String,
) {
    for handle in iter {
        let Some(node) = handle.get(parser) else {
            continue;
        };
        match node {
            tl::Node::Raw(b) => out.push_str(&b.as_utf8_str()),
            tl::Node::Tag(tag) => {
                let name = tag.name().as_utf8_str().to_ascii_lowercase();
                match name.as_str() {
                    "pre" => {
                        out.push_str("\n```\n");
                        render_node_to_md(tag.children().top().iter().copied(), parser, out);
                        out.push_str("\n```\n");
                    }
                    "code" => {
                        out.push('`');
                        render_node_to_md(tag.children().top().iter().copied(), parser, out);
                        out.push('`');
                    }
                    "p" | "div" | "br" => {
                        out.push_str("\n\n");
                        render_node_to_md(tag.children().top().iter().copied(), parser, out);
                    }
                    "strong" | "b" => {
                        out.push_str("**");
                        render_node_to_md(tag.children().top().iter().copied(), parser, out);
                        out.push_str("**");
                    }
                    "em" | "i" => {
                        out.push('_');
                        render_node_to_md(tag.children().top().iter().copied(), parser, out);
                        out.push('_');
                    }
                    "a" => {
                        let href = tag
                            .attributes()
                            .get("href")
                            .flatten()
                            .map(|v| v.as_utf8_str().to_string())
                            .unwrap_or_default();
                        out.push('[');
                        render_node_to_md(tag.children().top().iter().copied(), parser, out);
                        out.push_str(&format!("]({href})"));
                    }
                    "li" => {
                        out.push_str("\n- ");
                        render_node_to_md(tag.children().top().iter().copied(), parser, out);
                    }
                    _ => {
                        render_node_to_md(tag.children().top().iter().copied(), parser, out);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn so_question_id_from_stackoverflow_url() {
        let url = Url::parse("https://stackoverflow.com/questions/57430839/some-slug").unwrap();
        assert_eq!(so_question_id_from_url(&url), Some(57430839));
    }

    #[test]
    fn so_question_id_returns_none_for_non_question_url() {
        let url = Url::parse("https://stackoverflow.com/tags/rust").unwrap();
        assert_eq!(so_question_id_from_url(&url), None);
    }

    #[test]
    fn parse_so_answers_sorts_accepted_first() {
        let json = r#"{"items": [
            {"body": "<p>High score but not accepted</p>", "score": 100, "is_accepted": false},
            {"body": "<p>Accepted answer</p>", "score": 42, "is_accepted": true}
        ]}"#;
        let answers = parse_so_answers(json).unwrap();
        assert!(answers[0].is_accepted);
        assert_eq!(answers[0].score, 42);
    }

    #[test]
    fn so_api_urls_include_question_id_and_expected_filters() {
        let answers = so_answers_url(57430839).unwrap();
        let question = so_question_url(57430839).unwrap();
        assert!(
            answers
                .as_str()
                .contains("/2.3/questions/57430839/answers?order=desc&sort=votes")
        );
        assert!(answers.as_str().contains("filter=withbody"));
        assert!(
            question
                .as_str()
                .contains("/2.3/questions/57430839?site=stackoverflow&filter=withbody")
        );
    }

    #[test]
    fn so_question_id_returns_none_for_non_numeric_id() {
        let url = Url::parse("https://stackoverflow.com/questions/not-a-number/slug").unwrap();
        assert_eq!(so_question_id_from_url(&url), None);
    }

    #[test]
    fn parse_so_question_defaults_to_empty_when_items_missing() {
        let title = parse_so_question(r#"{"items":[]}"#).unwrap();
        assert!(title.is_empty());
    }

    #[test]
    fn parse_so_answers_renders_common_html_tags_to_markdown() {
        let json = r#"{"items":[
            {
                "body":"<p>Hello <strong>world</strong> and <em>friends</em> <a href=\"https://example.com\">link</a></p><pre><code>fn main() {}</code></pre><ul><li>first item</li></ul><div>Block</div><br/>",
                "score":100,
                "is_accepted":false
            },
            {
                "body":"<p>Accepted answer</p>",
                "score":1,
                "is_accepted":true
            }
        ]}"#;

        let answers = parse_so_answers(json).unwrap();
        assert!(
            answers[0].is_accepted,
            "accepted answer should be sorted first"
        );
        assert!(answers[1].body_markdown.contains("**world**"));
        assert!(answers[1].body_markdown.contains("_friends_"));
        assert!(
            answers[1]
                .body_markdown
                .contains("[link](https://example.com)")
        );
        assert!(answers[1].body_markdown.contains("```"));
        assert!(answers[1].body_markdown.contains("- first item"));
        assert!(answers[1].body_markdown.contains("Block"));
    }

    #[test]
    fn strip_html_to_markdown_falls_back_to_input_on_parse_failure() {
        let malformed = "<div><p>unterminated";
        let rendered = strip_html_to_markdown(malformed);
        assert!(!rendered.trim().is_empty());
    }

    #[test]
    fn format_so_content_respects_mode_answer_limits() {
        let content = SoContent {
            title: "Rust question".into(),
            answers: vec![
                SoAnswer {
                    body_markdown: "accepted".into(),
                    score: 10,
                    is_accepted: true,
                },
                SoAnswer {
                    body_markdown: "second".into(),
                    score: 9,
                    is_accepted: false,
                },
                SoAnswer {
                    body_markdown: "third".into(),
                    score: 8,
                    is_accepted: false,
                },
                SoAnswer {
                    body_markdown: "fourth".into(),
                    score: 7,
                    is_accepted: false,
                },
            ],
        };

        let compact = format_so_content(&content, crate::verbosity::Verbosity::Compact);
        assert_eq!(compact.matches("### ").count(), 1);
        assert!(compact.contains("✅ Accepted Answer"));

        let balanced = format_so_content(&content, crate::verbosity::Verbosity::Standard);
        assert_eq!(balanced.matches("### ").count(), 3);
        assert!(!balanced.contains("fourth"));

        let verbose = format_so_content(&content, crate::verbosity::Verbosity::Full);
        assert_eq!(verbose.matches("### ").count(), 4);
        assert!(verbose.contains("fourth"));
    }
}
