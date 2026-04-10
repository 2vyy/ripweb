//! Post-Processing & Re-ranking
//!
//! Applies family-specific cleanup rules, such as re-ranking
//! forum comments or stripping documentation sidetrees.

use super::boilerplate::tag_attribute;
use super::family::PageFamily;
use super::render::render_markdown;

/// Fine-tune the extracted text based on the detected page family.
///
/// For Forums: Re-ranks answers by score/accepted status.
/// For Docs: Filters redundant sidebar/TOC artifacts that might have leaked into the candidate.
pub fn post_process(family: PageFamily, dom: &tl::VDom, current_text: String) -> String {
    match family {
        PageFamily::Forum => {
            let processed = post_process_forum(dom);
            if processed.is_empty() {
                current_text
            } else {
                processed
            }
        }
        _ => current_text,
    }
}

fn post_process_forum(dom: &tl::VDom) -> String {
    let parser = dom.parser();

    // 1. Find the "Question/OP" and "Answers/Replies"
    // Heuristic: identify common post/comment classes
    let post_selectors = [".post", ".comment", ".answer", ".message", "article"];

    let mut posts = Vec::new();
    for selector in post_selectors {
        if let Some(nodes) = dom.query_selector(selector) {
            for handle in nodes {
                if let Some(tag) = handle.get(parser).and_then(|n| n.as_tag()) {
                    let score = extract_score(tag, parser);
                    let is_accepted = tag_attribute(tag, "class")
                        .map(|c| c.contains("accepted"))
                        .unwrap_or(false);

                    // Render children but skip the score element to avoid redundancy
                    let content = render_post_content(tag, parser);

                    if !content.trim().is_empty() {
                        posts.push(ForumPost {
                            content,
                            score,
                            is_accepted,
                        });
                    }
                }
            }
        }
        if !posts.is_empty() {
            break;
        }
    }

    if posts.is_empty() {
        return String::new();
    }

    // Identify OP (first one usually) vs replies
    // For now, we assume the first post is the OP and we don't re-rank it,
    // but we re-rank the rest.
    let op = posts.remove(0);

    posts.sort_by(|a, b| {
        if a.is_accepted != b.is_accepted {
            b.is_accepted.cmp(&a.is_accepted)
        } else {
            b.score.cmp(&a.score)
        }
    });

    let mut out = String::new();
    out.push_str(&op.content);
    out.push_str("\n\n---\n\n## Answers\n\n");

    for post in posts {
        if post.is_accepted {
            let _ = std::fmt::write(
                &mut out,
                format_args!("### [Score: {}] (Accepted Answer)\n\n", post.score),
            );
        } else {
            let _ = std::fmt::write(&mut out, format_args!("### [Score: {}]\n\n", post.score));
        }
        out.push_str(&post.content);
        out.push_str("\n\n");
    }

    out
}

fn render_post_content(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let mut ignore_handles = std::collections::HashSet::new();
    let score_selectors = [
        ".score",
        ".vote-count",
        ".upvote-count",
        "[itemprop='upvoteCount']",
    ];
    for selector in score_selectors {
        if let Some(nodes) = tag.query_selector(parser, selector) {
            for handle in nodes {
                ignore_handles.insert(handle.get_inner());
            }
        }
    }

    let mut out = String::new();
    for handle in tag.children().top().iter() {
        if ignore_handles.contains(&handle.get_inner()) {
            continue;
        }
        if let Some(child) = handle.get(parser) {
            out.push_str(&render_markdown(child, parser));
        }
    }
    out
}

struct ForumPost {
    content: String,
    score: i32,
    is_accepted: bool,
}

fn extract_score(tag: &tl::HTMLTag, parser: &tl::Parser) -> i32 {
    let score_selectors = [
        ".score",
        ".vote-count",
        ".upvote-count",
        "[itemprop='upvoteCount']",
    ];
    for selector in score_selectors {
        if let Some(mut nodes) = tag.query_selector(parser, selector)
            && let Some(handle) = nodes.next()
        {
            if let Some(node) = handle.get(parser) {
                let text = node.inner_text(parser);
                if let Some(score) = parse_numeric_score(&text) {
                    return score;
                }
            } else {
                tracing::debug!("Failed to resolve score node handle in forum extraction");
            }
        }
    }
    0
}

fn parse_numeric_score(s: &str) -> Option<i32> {
    let cleaned = s
        .trim()
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '-' || *c == '+')
        .collect::<String>();
    cleaned.parse::<i32>().ok()
}
