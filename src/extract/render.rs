//! Markdown Rendering
//!
//! Converts a purified DOM subtree into structured Markdown,
//! preserving headings, lists, code blocks, and links.

use super::boilerplate::{NUKE_TAGS, should_strip_subtree};
use crate::minify::strip_tracking;

pub fn render_markdown(node: &tl::Node, parser: &tl::Parser) -> String {
    match node {
        tl::Node::Tag(tag) => render_tag(tag, parser),
        tl::Node::Raw(bytes) => normalize_text(&bytes.as_utf8_str()),
        tl::Node::Comment(_) => String::new(),
    }
}

pub fn render_tag(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let name = tag.name().as_utf8_str().to_ascii_lowercase();
    if should_strip_subtree(tag) || NUKE_TAGS.contains(&name.as_str()) {
        return String::new();
    }
    match name.as_str() {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => render_heading(tag, parser, &name),
        "p" => wrap_block(render_children_inline(tag, parser)),
        "br" => "\n".to_owned(),
        "hr" => "\n\n---\n\n".to_owned(),
        "pre" => render_pre(tag, parser),
        "code" => render_inline_code(tag, parser),
        "a" => render_link(tag, parser),
        "em" | "i" => wrap_inline_marker("*", render_children_inline(tag, parser)),
        "strong" | "b" => wrap_inline_marker("**", render_children_inline(tag, parser)),
        "blockquote" => render_blockquote(tag, parser),
        "ul" => render_list(tag, parser, false),
        "ol" => render_list(tag, parser, true),
        "li" => render_children_inline(tag, parser),
        "img" => render_image(tag),
        "table" => render_table(tag, parser),
        "thead" | "tbody" | "tfoot" | "tr" => render_children_blocks(tag, parser),
        "th" | "td" => render_children_inline(tag, parser),
        "main" | "article" | "body" => render_children_blocks(tag, parser),
        "section" | "div" | "aside" => {
            let rendered = render_children_blocks(tag, parser);
            if prune_sidebar(&rendered, tag, parser) {
                return String::new();
            }
            rendered
        }
        "span" | "small" | "time" | "label" | "summary" | "details" => {
            render_children_inline(tag, parser)
        }
        _ => render_children_blocks(tag, parser),
    }
}

pub fn cleanup_markdown(text: &str) -> String {
    let mut out = String::new();
    let mut blank_run = 0usize;
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            blank_run += 1;
            if blank_run <= 2 && !out.is_empty() {
                out.push('\n');
            }
            continue;
        }
        blank_run = 0;
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(line);
    }
    out.trim().to_owned()
}

fn prune_sidebar(rendered: &str, tag: &tl::HTMLTag, parser: &tl::Parser) -> bool {
    if rendered.len() < 30 {
        return false;
    }
    // Protect blocks containing technical spec tables
    if let Some(mut tables) = tag.query_selector(parser, "table")
        && tables.next().is_some()
    {
        return false;
    }

    let mut link_chars = 0;
    let mut clean_len = 0;
    let mut in_bracket = false;
    let mut in_paren = false;
    let mut prev = '\0';

    for c in rendered.chars() {
        if c == '[' && prev != '!' {
            in_bracket = true;
            clean_len += 1;
        } else if c == ']' && in_bracket {
            in_bracket = false;
            clean_len += 1;
        } else if c == '(' && prev == ']' {
            in_paren = true;
        } else if c == ')' && in_paren {
            in_paren = false;
        } else if in_paren {
            // skip url characters in density calculation
            continue;
        } else {
            clean_len += 1;
            if in_bracket {
                link_chars += 1;
            }
        }
        prev = c;
    }

    let saturation = if clean_len > 0 {
        link_chars as f64 / clean_len as f64
    } else {
        0.0
    };

    if saturation > 0.4 {
        let clean = cleanup_markdown(rendered);
        let stats = crate::extract::candidate::score_text(&clean);
        let is_sidebar_list = stats.list_items >= 4 && stats.paragraphs <= 2;
        let is_listing = stats.paragraphs >= 3 && stats.short_lines >= 4;

        if is_listing && !is_sidebar_list {
            return false;
        }
        return true;
    }
    false
}

pub fn extract_body_markdown(dom: &tl::VDom) -> String {
    let parser = dom.parser();
    if let Some(text) = dom
        .query_selector("body")
        .and_then(|mut hits| hits.next())
        .and_then(|handle| handle.get(parser))
        .map(|node| render_markdown(node, parser))
    {
        return text;
    }
    dom.nodes()
        .iter()
        .map(|n| render_markdown(n, parser))
        .filter(|s| !s.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn extract_next_data(dom: &tl::VDom) -> Option<String> {
    let parser = dom.parser();
    let mut iter = dom.query_selector(r#"script[id="__NEXT_DATA__"]"#)?;
    let handle = iter.next()?;
    let node = handle.get(parser)?;
    let tag = node.as_tag()?;
    let raw_json = tag.inner_text(parser);
    let value: serde_json::Value = serde_json::from_str(&raw_json).ok()?;
    let mut parts: Vec<String> = Vec::new();
    collect_json_strings(&value, &mut parts);
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

pub fn render_children_blocks(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let mut out = String::new();
    for handle in tag.children().top().iter() {
        let Some(child) = handle.get(parser) else {
            continue;
        };
        let rendered = render_markdown(child, parser);
        if rendered.is_empty() {
            continue;
        }
        if !out.is_empty() && !out.ends_with('\n') && !rendered.starts_with('\n') {
            out.push(' ');
        }
        out.push_str(&rendered);
    }
    out
}

pub fn render_children_inline(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let parts = tag
        .children()
        .top()
        .iter()
        .filter_map(|handle| handle.get(parser))
        .map(|child| render_markdown(child, parser))
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>();
    normalize_inline_spacing(&parts.join(" "))
}

fn render_heading(tag: &tl::HTMLTag, parser: &tl::Parser, name: &str) -> String {
    let level = name
        .strip_prefix('h')
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1)
        .clamp(1, 6);
    let text = render_children_inline(tag, parser);
    if text.is_empty() {
        String::new()
    } else {
        format!("\n\n{} {}\n\n", "#".repeat(level), text)
    }
}

fn render_pre(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    // Try to extract language from a child <code class="language-*"> element
    let lang = tag
        .children()
        .top()
        .iter()
        .find_map(|handle| handle.get(parser))
        .and_then(|node| node.as_tag())
        .filter(|child| child.name().as_utf8_str().eq_ignore_ascii_case("code"))
        .and_then(|code_tag| super::boilerplate::tag_attribute(code_tag, "class"))
        .and_then(|cls| {
            cls.split_whitespace()
                .find(|c| c.starts_with("language-"))
                .map(|c| c.trim_start_matches("language-").to_owned())
        });
    let lang_str = lang.as_deref().unwrap_or("");
    let code = tag.inner_text(parser).replace('\r', "");
    let trimmed = code.trim_matches('\n');
    if trimmed.trim().is_empty() {
        String::new()
    } else {
        format!("\n\n```{lang_str}\n{}\n```\n\n", trimmed)
    }
}

fn render_inline_code(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let code = normalize_text(&tag.inner_text(parser));
    if code.is_empty() {
        String::new()
    } else {
        format!("`{code}`")
    }
}

fn render_link(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let text = render_children_inline(tag, parser);
    let href = tag
        .attributes()
        .get("href")
        .flatten()
        .map(|v| v.as_utf8_str().to_string())
        .unwrap_or_default();
    if href.is_empty() {
        return text;
    }
    let href = strip_tracking(&href);
    let label = if text.is_empty() { href.clone() } else { text };
    format!("[{label}]({href})")
}

fn render_blockquote(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let inner = cleanup_markdown(&render_children_blocks(tag, parser));
    if inner.is_empty() {
        return String::new();
    }
    let quoted = inner
        .lines()
        .map(|line| {
            if line.is_empty() {
                ">".to_owned()
            } else {
                format!("> {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("\n\n{quoted}\n\n")
}

fn render_list(tag: &tl::HTMLTag, parser: &tl::Parser, ordered: bool) -> String {
    let mut items = Vec::new();
    for handle in tag.children().top().iter() {
        let Some(child) = handle.get(parser) else {
            continue;
        };
        let Some(li) = child.as_tag() else { continue };
        if li.name().as_utf8_str().to_ascii_lowercase() != "li" {
            continue;
        }
        let content = cleanup_markdown(&render_children_blocks(li, parser));
        if content.is_empty() {
            continue;
        }
        let marker = if ordered {
            format!("{}. ", items.len() + 1)
        } else {
            "- ".to_owned()
        };
        items.push(indent_block(&content, &marker, "  "));
    }
    if items.is_empty() {
        String::new()
    } else {
        format!("\n\n{}\n\n", items.join("\n"))
    }
}

fn render_image(tag: &tl::HTMLTag) -> String {
    tag.attributes()
        .get("alt")
        .flatten()
        .map(|alt| normalize_text(&alt.as_utf8_str()))
        .filter(|alt| !alt.is_empty())
        .map(|alt| format!("[Image: {alt}]"))
        .unwrap_or_default()
}

fn render_table(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let rows = collect_table_rows(tag, parser);
    if rows.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n\n");
    for row in rows {
        out.push_str("| ");
        out.push_str(&row.join(" | "));
        out.push_str(" |\n");
    }
    out.push('\n');
    out
}

fn collect_table_rows(tag: &tl::HTMLTag, parser: &tl::Parser) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for handle in tag.children().top().iter() {
        let Some(child) = handle.get(parser) else {
            continue;
        };
        let Some(child_tag) = child.as_tag() else {
            continue;
        };
        let name = child_tag.name().as_utf8_str().to_ascii_lowercase();
        match name.as_str() {
            "thead" | "tbody" | "tfoot" => rows.extend(collect_table_rows(child_tag, parser)),
            "tr" => {
                let mut cells = Vec::new();
                for cell_handle in child_tag.children().top().iter() {
                    let Some(cell_node) = cell_handle.get(parser) else {
                        continue;
                    };
                    let Some(cell_tag) = cell_node.as_tag() else {
                        continue;
                    };
                    let cell_name = cell_tag.name().as_utf8_str().to_ascii_lowercase();
                    if !matches!(cell_name.as_str(), "th" | "td") {
                        continue;
                    }
                    let cell = render_children_inline(cell_tag, parser);
                    if !cell.is_empty() {
                        cells.push(cell);
                    }
                }
                if !cells.is_empty() {
                    rows.push(cells);
                }
            }
            _ => {}
        }
    }
    rows
}

fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_inline_spacing(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == ' ' {
            let next = chars.peek().copied();
            if out.ends_with(' ')
                || matches!(
                    next,
                    Some(',' | '.' | ';' | ':' | '!' | '?' | ')' | ']' | '}')
                )
                || matches!(out.chars().last(), Some('(' | '[' | '{' | '\n'))
            {
                continue;
            }
        }
        out.push(ch);
    }
    out.trim().to_owned()
}

fn wrap_inline_marker(marker: &str, text: String) -> String {
    if text.is_empty() {
        String::new()
    } else {
        format!("{marker}{text}{marker}")
    }
}

fn wrap_block(text: String) -> String {
    if text.is_empty() {
        String::new()
    } else {
        format!("\n\n{text}\n\n")
    }
}

fn indent_block(text: &str, first_prefix: &str, rest_prefix: &str) -> String {
    text.lines()
        .enumerate()
        .map(|(idx, line)| {
            if idx == 0 {
                format!("{first_prefix}{line}")
            } else if line.is_empty() {
                rest_prefix.to_owned()
            } else {
                format!("{rest_prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_content_leaf(s: &str) -> bool {
    s.len() > 20 && s.bytes().filter(|&b| b == b' ').count() >= 2
}

fn collect_json_strings(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(s) => {
            let trimmed = s.trim().to_owned();
            if is_content_leaf(&trimmed) {
                out.push(trimmed);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_json_strings(v, out);
            }
        }
        serde_json::Value::Object(map) => {
            for v in map.values() {
                collect_json_strings(v, out);
            }
        }
        _ => {}
    }
}
