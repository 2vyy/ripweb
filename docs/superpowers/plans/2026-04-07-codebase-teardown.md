# Codebase Teardown Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete corpus/golden/examples infrastructure, split `src/extract/web.rs` into focused modules, and extract orchestration from `main.rs` into `src/run.rs`.

**Architecture:** Three phases in order: (1) delete dead infra and fix `lib.rs`, (2) split `extract/web.rs` by concern—each new module added while `web.rs` keeps its originals, then a final swap step removes the originals, (3) create `src/run.rs` and thin `main.rs`. Compile check after every task.

**Tech Stack:** Rust 2024 edition, tokio, tl, insta, cargo

---

## File Map

**Deleted:**
- `examples/` (14 scripts)
- `corpus/` (after `torture/` moved)
- `tests/expected/`
- `benches/html_samples/`
- `src/corpus.rs`

**Moved:**
- `corpus/torture/` → `tests/fixtures/torture/`

**Created:**
- `src/extract/boilerplate.rs` — `NUKE_TAGS`, `NEGATIVE_HINTS`, `tag_attribute()`, `should_strip_subtree()`
- `src/extract/family.rs` — `PageFamily`, `TextStats`, all family detection + scoring
- `src/extract/render.rs` — all `render_*` functions, `cleanup_markdown`, SPA fallback
- `src/extract/candidate.rs` — `ScoredCandidate`, `extract_best_candidate`, `score_candidate`, `score_text`
- `src/run.rs` — `dispatch`, `handle_platform`, `handle_query`, `run_crawler`, format helpers

**Modified:**
- `src/extract/web.rs` — reduced to `WebExtractor` + `extract_from_str` + charset decode
- `src/extract/mod.rs` — add four new submodules
- `src/lib.rs` — add `pub mod run`, remove `pub mod corpus`
- `src/main.rs` — shell only: arg parse, spinner, Ctrl+C, call `run::dispatch`, output
- `docs/TESTING.md` — remove references to deleted examples and corpus commands

---

## Phase 1 — Delete Infrastructure

### Task 1: Delete dead infra, move torture fixtures, fix lib.rs

**Files:**
- Delete: `examples/`, `tests/expected/`, `benches/html_samples/`, `src/corpus.rs`, `corpus/` (post-move)
- Move: `corpus/torture/` → `tests/fixtures/torture/`
- Modify: `src/lib.rs`

- [ ] **Step 1: Move torture fixtures**

```bash
mkdir -p tests/fixtures/torture
cp -r corpus/torture/. tests/fixtures/torture/
```

- [ ] **Step 2: Delete dead infrastructure**

```bash
rm -rf examples/ corpus/ tests/expected/ benches/html_samples/ src/corpus.rs
```

- [ ] **Step 3: Remove corpus module from src/lib.rs**

Replace the entire contents of `src/lib.rs` with:

```rust
pub mod cli;
pub mod config;
pub mod error;
pub mod extract;
pub mod fetch;
pub mod minify;
pub mod router;
pub mod search;

pub use error::RipwebError;
```

- [ ] **Step 4: Verify**

```bash
cargo check && cargo test
```

Expected: clean compile, all tests pass.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: delete corpus/examples/golden infrastructure"
```

---

## Phase 2 — Split extract/web.rs

Modules are added one at a time while `web.rs` keeps its originals. After all four modules exist, a single swap task removes the duplicates from `web.rs`.

### Task 2: Create src/extract/boilerplate.rs

**Files:**
- Create: `src/extract/boilerplate.rs`
- Modify: `src/extract/mod.rs`

- [ ] **Step 1: Create src/extract/boilerplate.rs**

```rust
/// Tags whose entire subtrees are stripped before content extraction.
pub const NUKE_TAGS: &[&str] = &[
    "nav", "footer", "header", "aside", "style", "svg", "iframe", "form", "script", "noscript",
];

pub const NEGATIVE_HINTS: &[&str] = &[
    "nav", "menu", "sidebar", "footer", "header", "cookie", "modal", "popup", "banner", "share",
    "social", "breadcrumb", "comment", "related", "recommend", "promo", "advert", "ad-",
    "utility", "toolbar", "newsletter", "subscribe", "sitemap", "carousel", "slider",
];

/// Extract a named attribute value from an HTML tag.
pub fn tag_attribute(tag: &tl::HTMLTag, name: &str) -> Option<String> {
    tag.attributes()
        .get(name)
        .flatten()
        .map(|value| value.as_utf8_str().to_string())
}

/// Returns true when a tag's id/class hints match a known boilerplate pattern.
pub fn should_strip_subtree(tag: &tl::HTMLTag) -> bool {
    let name = tag.name().as_utf8_str().to_ascii_lowercase();
    if !matches!(
        name.as_str(),
        "div" | "section" | "main" | "article" | "aside" | "ul" | "ol" | "li"
    ) {
        return false;
    }

    let hint_text = [tag_attribute(tag, "id"), tag_attribute(tag, "class")]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();

    if hint_text.is_empty() {
        return false;
    }

    NEGATIVE_HINTS.iter().any(|hint| hint_text.contains(hint))
}
```

- [ ] **Step 2: Add `pub mod boilerplate` to src/extract/mod.rs**

```rust
pub mod boilerplate;
pub mod links;
pub mod web;

use crate::error::RipwebError;

pub trait Extractor {
    fn extract(bytes: &[u8], content_type: Option<&str>) -> Result<String, RipwebError>;
}
```

- [ ] **Step 3: cargo check**

```bash
cargo check
```

Expected: clean (web.rs still has its own copies — no conflict yet).

- [ ] **Step 4: Commit**

```bash
git add src/extract/boilerplate.rs src/extract/mod.rs
git commit -m "refactor: add extract/boilerplate.rs"
```

---

### Task 3: Create src/extract/family.rs

**Files:**
- Create: `src/extract/family.rs`
- Modify: `src/extract/mod.rs`

`TextStats` lives here because it is the primary input to family classification. `candidate.rs` will import it from here.

- [ ] **Step 1: Create src/extract/family.rs**

```rust
use crate::config::family_hint_for_host;
use super::boilerplate::tag_attribute;
use url::Url;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageFamily {
    Docs,
    Article,
    Product,
    Generic,
}

#[derive(Default)]
pub struct TextStats {
    pub word_count: usize,
    pub headings: usize,
    pub paragraphs: usize,
    pub code_fences: usize,
    pub list_items: usize,
    pub link_count: usize,
    pub short_lines: usize,
}

const DOC_HINTS: &[&str] = &[
    "doc", "docs", "reference", "api", "manual", "guide", "guides", "tutorial", "learn",
    "developer", "developers", "readthedocs", "gitbook", "docusaurus",
];
const ARTICLE_HINTS: &[&str] = &[
    "article", "post", "story", "blog", "news", "entry", "content", "prose",
];
const PRODUCT_HINTS: &[&str] = &[
    "product", "pdp", "buybox", "price", "pricing", "spec", "specs", "sku", "item",
    "details", "purchase", "cart", "merchant", "offer",
];

pub fn host_family_hint(source_url: &str) -> Option<PageFamily> {
    let host = Url::parse(source_url).ok()?.host_str()?.to_ascii_lowercase();
    match family_hint_for_host(&host)? {
        "docs" => Some(PageFamily::Docs),
        "article" => Some(PageFamily::Article),
        "product" => Some(PageFamily::Product),
        _ => Some(PageFamily::Generic),
    }
}

pub fn classify_candidate_family(
    tag: &tl::HTMLTag,
    rendered: &str,
    stats: &TextStats,
    url_family: PageFamily,
) -> PageFamily {
    if url_family != PageFamily::Generic {
        return url_family;
    }

    let hint_text = [tag_attribute(tag, "id"), tag_attribute(tag, "class")]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();

    let code_heavy = stats.code_fences > 0
        || (stats.headings >= 3 && stats.list_items >= 3 && stats.link_count >= 8);
    let prose_heavy = stats.paragraphs >= 3 && stats.word_count >= 180;
    let price_markers = count_price_markers(rendered);
    let spec_markers = count_spec_markers(rendered);
    let productish =
        price_markers > 0 && (spec_markers > 0 || stats.list_items >= 2 || stats.headings >= 1);

    if DOC_HINTS.iter().any(|hint| hint_text.contains(hint)) || code_heavy {
        return PageFamily::Docs;
    }
    if PRODUCT_HINTS.iter().any(|hint| hint_text.contains(hint)) || productish {
        return PageFamily::Product;
    }
    if ARTICLE_HINTS.iter().any(|hint| hint_text.contains(hint)) || prose_heavy {
        return PageFamily::Article;
    }

    PageFamily::Generic
}

pub fn family_score_adjustment(
    family: PageFamily,
    stats: &TextStats,
    price_markers: usize,
    spec_markers: usize,
) -> i64 {
    match family {
        PageFamily::Docs => {
            (stats.headings as i64) * 12
                + (stats.code_fences as i64) * 18
                + (stats.list_items as i64) * 4
                - (stats.short_lines as i64)
        }
        PageFamily::Article => {
            (stats.paragraphs as i64) * 14
                + (stats.word_count as i64 / 20)
                - (stats.link_count as i64) * 2
        }
        PageFamily::Product => {
            (stats.headings as i64) * 16
                + (stats.list_items as i64) * 12
                + (price_markers as i64) * 40
                + (spec_markers as i64) * 22
                - (stats.link_count as i64) * 4
        }
        PageFamily::Generic => 0,
    }
}

pub fn count_price_markers(text: &str) -> usize {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.contains('$')
                || trimmed.contains("Current price")
                || trimmed.contains("Sale price")
                || trimmed.contains("Price when purchased")
        })
        .count()
}

pub fn count_spec_markers(text: &str) -> usize {
    const SPEC_HINTS: &[&str] = &[
        "specifications",
        "specs",
        "product details",
        "about this item",
        "key features",
        "warranty",
        "dimensions",
        "brand",
        "model",
        "isbn",
        "sku",
    ];
    text.lines()
        .filter(|line| {
            let lower = line.trim().to_ascii_lowercase();
            SPEC_HINTS.iter().any(|hint| lower.contains(hint))
        })
        .count()
}
```

- [ ] **Step 2: Add `pub mod family` to src/extract/mod.rs**

```rust
pub mod boilerplate;
pub mod family;
pub mod links;
pub mod web;

use crate::error::RipwebError;

pub trait Extractor {
    fn extract(bytes: &[u8], content_type: Option<&str>) -> Result<String, RipwebError>;
}
```

- [ ] **Step 3: cargo check**

```bash
cargo check
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src/extract/family.rs src/extract/mod.rs
git commit -m "refactor: add extract/family.rs"
```

---

### Task 4: Create src/extract/render.rs

**Files:**
- Create: `src/extract/render.rs`
- Modify: `src/extract/mod.rs`

Owns all Markdown rendering, cleanup, and SPA fallback. Depends only on `boilerplate` and `crate::minify`.

- [ ] **Step 1: Create src/extract/render.rs with the following contents**

```rust
use crate::minify::strip_tracking;
use super::boilerplate::{NUKE_TAGS, should_strip_subtree};

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
        "main" | "article" | "section" | "div" | "body" => render_children_blocks(tag, parser),
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
    if parts.is_empty() { None } else { Some(parts.join("\n\n")) }
}

pub fn render_children_blocks(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let mut out = String::new();
    for handle in tag.children().top().iter() {
        let Some(child) = handle.get(parser) else { continue };
        let rendered = render_markdown(child, parser);
        if rendered.is_empty() { continue }
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
    if text.is_empty() { String::new() } else { format!("\n\n{} {}\n\n", "#".repeat(level), text) }
}

fn render_pre(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let code = tag.inner_text(parser).replace('\r', "");
    let trimmed = code.trim_matches('\n');
    if trimmed.trim().is_empty() { String::new() } else { format!("\n\n```\n{}\n```\n\n", trimmed) }
}

fn render_inline_code(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let code = normalize_text(&tag.inner_text(parser));
    if code.is_empty() { String::new() } else { format!("`{code}`") }
}

fn render_link(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let text = render_children_inline(tag, parser);
    let href = tag
        .attributes()
        .get("href")
        .flatten()
        .map(|v| v.as_utf8_str().to_string())
        .unwrap_or_default();
    if href.is_empty() { return text; }
    let href = strip_tracking(&href);
    let label = if text.is_empty() { href.clone() } else { text };
    format!("[{label}]({href})")
}

fn render_blockquote(tag: &tl::HTMLTag, parser: &tl::Parser) -> String {
    let inner = cleanup_markdown(&render_children_blocks(tag, parser));
    if inner.is_empty() { return String::new(); }
    let quoted = inner
        .lines()
        .map(|line| if line.is_empty() { ">".to_owned() } else { format!("> {line}") })
        .collect::<Vec<_>>()
        .join("\n");
    format!("\n\n{quoted}\n\n")
}

fn render_list(tag: &tl::HTMLTag, parser: &tl::Parser, ordered: bool) -> String {
    let mut items = Vec::new();
    for handle in tag.children().top().iter() {
        let Some(child) = handle.get(parser) else { continue };
        let Some(li) = child.as_tag() else { continue };
        if li.name().as_utf8_str().to_ascii_lowercase() != "li" { continue }
        let content = cleanup_markdown(&render_children_blocks(li, parser));
        if content.is_empty() { continue }
        let marker = if ordered { format!("{}. ", items.len() + 1) } else { "- ".to_owned() };
        items.push(indent_block(&content, &marker, "  "));
    }
    if items.is_empty() { String::new() } else { format!("\n\n{}\n\n", items.join("\n")) }
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
    if rows.is_empty() { return String::new(); }
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
        let Some(child) = handle.get(parser) else { continue };
        let Some(child_tag) = child.as_tag() else { continue };
        let name = child_tag.name().as_utf8_str().to_ascii_lowercase();
        match name.as_str() {
            "thead" | "tbody" | "tfoot" => rows.extend(collect_table_rows(child_tag, parser)),
            "tr" => {
                let mut cells = Vec::new();
                for cell_handle in child_tag.children().top().iter() {
                    let Some(cell_node) = cell_handle.get(parser) else { continue };
                    let Some(cell_tag) = cell_node.as_tag() else { continue };
                    let cell_name = cell_tag.name().as_utf8_str().to_ascii_lowercase();
                    if !matches!(cell_name.as_str(), "th" | "td") { continue }
                    let cell = render_children_inline(cell_tag, parser);
                    if !cell.is_empty() { cells.push(cell); }
                }
                if !cells.is_empty() { rows.push(cells); }
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
                || matches!(next, Some(',' | '.' | ';' | ':' | '!' | '?' | ')' | ']' | '}'))
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
    if text.is_empty() { String::new() } else { format!("{marker}{text}{marker}") }
}

fn wrap_block(text: String) -> String {
    if text.is_empty() { String::new() } else { format!("\n\n{text}\n\n") }
}

fn indent_block(text: &str, first_prefix: &str, rest_prefix: &str) -> String {
    text.lines()
        .enumerate()
        .map(|(idx, line)| {
            if idx == 0 { format!("{first_prefix}{line}") }
            else if line.is_empty() { rest_prefix.to_owned() }
            else { format!("{rest_prefix}{line}") }
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
            if is_content_leaf(&trimmed) { out.push(trimmed); }
        }
        serde_json::Value::Array(arr) => { for v in arr { collect_json_strings(v, out); } }
        serde_json::Value::Object(map) => { for v in map.values() { collect_json_strings(v, out); } }
        _ => {}
    }
}
```

- [ ] **Step 2: Add `pub mod render` to src/extract/mod.rs**

```rust
pub mod boilerplate;
pub mod family;
pub mod links;
pub mod render;
pub mod web;

use crate::error::RipwebError;

pub trait Extractor {
    fn extract(bytes: &[u8], content_type: Option<&str>) -> Result<String, RipwebError>;
}
```

- [ ] **Step 3: cargo check**

```bash
cargo check
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src/extract/render.rs src/extract/mod.rs
git commit -m "refactor: add extract/render.rs"
```

---

### Task 5: Create src/extract/candidate.rs

**Files:**
- Create: `src/extract/candidate.rs`
- Modify: `src/extract/mod.rs`

Owns content root selection: scoring constants, `ScoredCandidate`, `extract_best_candidate`, `score_candidate`, `score_text`, `word_count`.

- [ ] **Step 1: Create src/extract/candidate.rs**

```rust
use super::boilerplate::{NUKE_TAGS, NEGATIVE_HINTS, tag_attribute, should_strip_subtree};
use super::family::{PageFamily, TextStats, classify_candidate_family, family_score_adjustment, count_price_markers, count_spec_markers};
use super::render::{render_tag, cleanup_markdown};

const CONTENT_ROOTS: &[&str] = &["main", "article"];
const FALLBACK_CANDIDATE_TAGS: &[&str] = &["section", "table"];
const MAX_FALLBACK_CANDIDATES_PER_SELECTOR: usize = 32;
const POSITIVE_HINTS: &[&str] = &[
    "article", "content", "main", "post", "entry", "body", "text", "doc", "docs", "markdown",
    "prose", "story",
];
const HINTED_DIV_SELECTORS: &[&str] = &[
    r#"div[id*="content"]"#,  r#"div[class*="content"]"#,
    r#"div[id*="article"]"#,  r#"div[class*="article"]"#,
    r#"div[id*="post"]"#,     r#"div[class*="post"]"#,
    r#"div[id*="entry"]"#,    r#"div[class*="entry"]"#,
    r#"div[id*="doc"]"#,      r#"div[class*="doc"]"#,
    r#"div[id*="markdown"]"#, r#"div[class*="markdown"]"#,
    r#"div[id*="prose"]"#,    r#"div[class*="prose"]"#,
    r#"div[id*="story"]"#,    r#"div[class*="story"]"#,
];

pub struct ScoredCandidate {
    pub score: i64,
    pub text: String,
}

pub fn word_count(s: &str) -> usize {
    s.split_whitespace().count()
}

pub fn score_text(text: &str) -> TextStats {
    let mut stats = TextStats {
        word_count: word_count(text),
        ..TextStats::default()
    };
    let mut in_code_fence = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            stats.code_fences += 1;
            in_code_fence = !in_code_fence;
            continue;
        }
        if in_code_fence { continue }
        if trimmed.starts_with('#') { stats.headings += 1; }
        if trimmed.starts_with("- ") || ordered_list_prefix(trimmed) { stats.list_items += 1; }
        if !trimmed.is_empty() && trimmed.len() < 35 { stats.short_lines += 1; }
        stats.link_count += trimmed.matches("](").count();
    }
    stats.paragraphs = text
        .split("\n\n")
        .filter(|chunk| !chunk.trim().is_empty())
        .count();
    stats
}

pub fn extract_best_candidate(dom: &tl::VDom, source_url: Option<&str>) -> String {
    let parser = dom.parser();
    let mut best: Option<ScoredCandidate> = None;
    let url_family = source_url
        .and_then(|u| super::family::host_family_hint(u))
        .unwrap_or(PageFamily::Generic);

    for selector in CONTENT_ROOTS {
        if let Some(hits) = dom.query_selector(selector) {
            for handle in hits {
                let Some(node) = handle.get(parser) else { continue };
                if let Some(tag) = node.as_tag() {
                    consider_candidate(&mut best, score_candidate(tag, parser, url_family));
                }
            }
        }
    }
    for selector in FALLBACK_CANDIDATE_TAGS {
        if let Some(hits) = dom.query_selector(selector) {
            for handle in hits.take(MAX_FALLBACK_CANDIDATES_PER_SELECTOR) {
                let Some(node) = handle.get(parser) else { continue };
                if let Some(tag) = node.as_tag() {
                    consider_candidate(&mut best, score_candidate(tag, parser, url_family));
                }
            }
        }
    }
    for selector in HINTED_DIV_SELECTORS {
        if let Some(hits) = dom.query_selector(selector) {
            for handle in hits.take(MAX_FALLBACK_CANDIDATES_PER_SELECTOR) {
                let Some(node) = handle.get(parser) else { continue };
                if let Some(tag) = node.as_tag() {
                    consider_candidate(&mut best, score_candidate(tag, parser, url_family));
                }
            }
        }
    }
    if let Some(body) = dom
        .query_selector("body")
        .and_then(|mut hits| hits.next())
        .and_then(|handle| handle.get(parser))
        .and_then(|node| node.as_tag())
    {
        consider_candidate(&mut best, score_candidate(body, parser, url_family));
    }

    best.map(|c| c.text)
        .unwrap_or_else(|| cleanup_markdown(&super::render::extract_body_markdown(dom)))
}

fn consider_candidate(best: &mut Option<ScoredCandidate>, candidate: Option<ScoredCandidate>) {
    let Some(candidate) = candidate else { return };
    if best.as_ref().is_none_or(|current| candidate.score > current.score) {
        *best = Some(candidate);
    }
}

fn score_candidate(
    tag: &tl::HTMLTag,
    parser: &tl::Parser,
    url_family: PageFamily,
) -> Option<ScoredCandidate> {
    let name = tag.name().as_utf8_str().to_ascii_lowercase();
    if should_strip_subtree(tag) || NUKE_TAGS.contains(&name.as_str()) {
        return None;
    }

    let text = cleanup_markdown(&render_tag(tag, parser));
    if text.is_empty() { return None; }

    let stats = score_text(&text);
    if stats.word_count == 0 { return None; }

    let family = classify_candidate_family(tag, &text, &stats, url_family);
    let price_markers = count_price_markers(&text);
    let spec_markers = count_spec_markers(&text);

    let mut score = stats.word_count as i64;
    score += (stats.paragraphs as i64) * 24;
    score += (stats.headings as i64) * 18;
    score += (stats.code_fences as i64) * 20;
    score += (stats.list_items as i64) * 10;
    score -= (stats.link_count as i64) * 6;
    score -= (stats.short_lines as i64) * 2;

    score += match name.as_str() {
        "article" => 80,
        "main" => 60,
        "section" => 20,
        "div" => 10,
        "table" => 12,
        "body" => -40,
        _ => 0,
    };

    let hint_text = [tag_attribute(tag, "id"), tag_attribute(tag, "class")]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();

    for hint in POSITIVE_HINTS {
        if hint_text.contains(hint) { score += 24; }
    }
    for hint in NEGATIVE_HINTS {
        if hint_text.contains(hint) { score -= 60; }
    }

    score += family_score_adjustment(family, &stats, price_markers, spec_markers);

    Some(ScoredCandidate { score, text })
}

fn ordered_list_prefix(line: &str) -> bool {
    let digits = line.bytes().take_while(|b| b.is_ascii_digit()).count();
    digits > 0 && line[digits..].starts_with(". ")
}
```

- [ ] **Step 2: Add `pub mod candidate` to src/extract/mod.rs**

```rust
pub mod boilerplate;
pub mod candidate;
pub mod family;
pub mod links;
pub mod render;
pub mod web;

use crate::error::RipwebError;

pub trait Extractor {
    fn extract(bytes: &[u8], content_type: Option<&str>) -> Result<String, RipwebError>;
}
```

- [ ] **Step 3: cargo check**

```bash
cargo check
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src/extract/candidate.rs src/extract/mod.rs
git commit -m "refactor: add extract/candidate.rs"
```

---

### Task 6: Replace web.rs with coordinator and move tests

All four modules now exist. Replace `web.rs` with a thin coordinator that calls into them. Move the existing classification tests to `family.rs`.

**Files:**
- Overwrite: `src/extract/web.rs`

- [ ] **Step 1: Replace src/extract/web.rs**

```rust
use crate::error::RipwebError;
use encoding_rs::Encoding;
use super::Extractor;
use super::candidate::extract_best_candidate;
use super::render::{cleanup_markdown, extract_next_data};
use super::candidate::word_count;

pub struct WebExtractor;

const MAX_INPUT_BYTES: usize = 5 * 1024 * 1024;

impl Extractor for WebExtractor {
    fn extract(bytes: &[u8], content_type: Option<&str>) -> Result<String, RipwebError> {
        if bytes.len() > MAX_INPUT_BYTES {
            return Err(RipwebError::InputTooLarge(bytes.len()));
        }
        let html = decode_charset(bytes, content_type);
        Ok(extract_from_str(&html, None))
    }
}

impl WebExtractor {
    pub fn extract_with_url(
        bytes: &[u8],
        content_type: Option<&str>,
        source_url: Option<&str>,
    ) -> Result<String, RipwebError> {
        if bytes.len() > MAX_INPUT_BYTES {
            return Err(RipwebError::InputTooLarge(bytes.len()));
        }
        let html = decode_charset(bytes, content_type);
        Ok(extract_from_str(&html, source_url))
    }
}

fn extract_from_str(html: &str, source_url: Option<&str>) -> String {
    let dom = match tl::parse(html, tl::ParserOptions::default()) {
        Ok(d) => d,
        Err(_) => return String::new(),
    };
    let text = extract_best_candidate(&dom, source_url);

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
```

- [ ] **Step 2: Add classification tests to src/extract/family.rs**

Append the following `#[cfg(test)]` block to the end of `src/extract/family.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::render::{render_tag, cleanup_markdown};
    use crate::extract::candidate::score_text;

    #[test]
    fn classifies_docs_candidates() {
        let html = r#"<html><body>
          <div class="docs reference-content">
            <h1>API Reference</h1><h2>Example</h2>
            <pre><code>fn main() { println!("hi"); }</code></pre>
            <ul><li>Item</li><li>Other</li></ul>
          </div></body></html>"#;
        let dom = tl::parse(html, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();
        let tag = dom.query_selector("div")
            .and_then(|mut it| it.next())
            .and_then(|h| h.get(parser))
            .and_then(|n| n.as_tag())
            .unwrap();
        let rendered = cleanup_markdown(&render_tag(tag, parser));
        let stats = score_text(&rendered);
        assert_eq!(classify_candidate_family(tag, &rendered, &stats, PageFamily::Generic), PageFamily::Docs);
    }

    #[test]
    fn classifies_article_candidates() {
        let html = r#"<html><body>
          <article class="story-body">
            <h1>Story Title</h1>
            <p>One two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twenty-one twenty-two twenty-three twenty-four.</p>
            <p>Another paragraph with enough prose to look like a real article rather than a sparse card or utility block.</p>
            <p>A third paragraph keeps the article heuristic clearly on the prose-heavy side for classification.</p>
          </article></body></html>"#;
        let dom = tl::parse(html, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();
        let tag = dom.query_selector("article")
            .and_then(|mut it| it.next())
            .and_then(|h| h.get(parser))
            .and_then(|n| n.as_tag())
            .unwrap();
        let rendered = cleanup_markdown(&render_tag(tag, parser));
        let stats = score_text(&rendered);
        assert_eq!(classify_candidate_family(tag, &rendered, &stats, PageFamily::Generic), PageFamily::Article);
    }

    #[test]
    fn classifies_product_candidates() {
        let html = r#"<html><body>
          <section class="product-details buybox">
            <h1>Ip Man 1-4 (Box Set) (Blu-ray)</h1>
            <p>Current price is USD$22.99</p>
            <h2>Key item features</h2>
            <ul><li>Action, Biography, Drama</li><li>Movie &amp; tv media format: Blu-ray</li></ul>
            <h2>Specifications</h2>
            <table><tr><th>Director</th><td>Wilson Yip</td></tr><tr><th>Resolution</th><td>1080p</td></tr></table>
          </section></body></html>"#;
        let dom = tl::parse(html, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();
        let tag = dom.query_selector("section")
            .and_then(|mut it| it.next())
            .and_then(|h| h.get(parser))
            .and_then(|n| n.as_tag())
            .unwrap();
        let rendered = cleanup_markdown(&render_tag(tag, parser));
        let stats = score_text(&rendered);
        assert_eq!(classify_candidate_family(tag, &rendered, &stats, PageFamily::Generic), PageFamily::Product);
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test
```

Expected: all tests pass including the three classification tests now in `family.rs`.

- [ ] **Step 4: Commit**

```bash
git add src/extract/web.rs src/extract/family.rs
git commit -m "refactor: replace web.rs with thin coordinator, move tests to family.rs"
```

---

## Phase 3 — Thin main.rs

### Task 7: Create src/run.rs and gut main.rs

**Files:**
- Create: `src/run.rs`
- Overwrite: `src/main.rs`
- Modify: `src/lib.rs`

`src/run.rs` takes everything from `main.rs` except the shell concerns (spinner, tracing setup, stdout write, Ctrl+C, exit code handling). `main.rs` becomes setup + call + output only.

- [ ] **Step 1: Create src/run.rs**

```rust
use std::sync::Arc;
use std::time::Duration;

use crate::{
    cli::{Cli, OutputMode},
    error::RipwebError,
    fetch::{
        cache::Cache,
        crawler::{format_output, Crawler, CrawledPage, CrawlerConfig},
        llms_txt::fetch_llms_txt,
        politeness::DomainSemaphores,
        RetryConfig,
    },
    minify::collapse,
    router::{route, PlatformRoute, Route},
    search::{
        duckduckgo,
        github,
        hackernews::{hn_api_url, parse_hn_json},
        reddit::{parse_reddit_json, reddit_json_url},
    },
};

pub async fn dispatch(
    cli: &Cli,
    input: &str,
    client: &Arc<rquest::Client>,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    let effective = if cli.force_url && !input.starts_with("http://") && !input.starts_with("https://") {
        format!("https://{input}")
    } else {
        input.to_owned()
    };

    let route = if cli.force_query {
        Route::Query(effective)
    } else {
        route(&effective)
    };

    match route {
        Route::Query(q) => handle_query(client, &q, cli, retry, sems, cache).await,
        Route::Url(platform) => handle_platform(client, platform, cli, retry, sems, cache).await,
    }
}

pub fn apply_output_mode(text: String, mode: OutputMode) -> String {
    match mode {
        OutputMode::Markdown => text.trim().to_owned(),
        OutputMode::Aggressive => collapse(text.trim()),
    }
}

async fn handle_platform(
    client: &Arc<rquest::Client>,
    platform: PlatformRoute,
    cli: &Cli,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    match platform {
        PlatformRoute::GitHub { owner, repo } => {
            let text = github::fetch_readme(client, &owner, &repo)
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?;
            Ok((text, 1))
        }
        PlatformRoute::Reddit { url } => {
            let json_url = reddit_json_url(&url)
                .ok_or_else(|| RipwebError::Config(format!("invalid Reddit URL: {url}")))?;
            let body = client
                .get(&json_url)
                .send()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?
                .text()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?;
            let content = parse_reddit_json(&body)
                .map_err(|e| RipwebError::Network(format!("Reddit JSON parse: {e}")))?;
            Ok((format_reddit(&content), 1))
        }
        PlatformRoute::HackerNews { item_id } => {
            let api = hn_api_url(&item_id);
            let body = client
                .get(api.as_str())
                .send()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?
                .text()
                .await
                .map_err(|e| RipwebError::Network(e.to_string()))?;
            let content = parse_hn_json(&body)
                .map_err(|e| RipwebError::Network(format!("HN JSON parse: {e}")))?;
            Ok((format_hn(&content), 1))
        }
        PlatformRoute::Generic(url) => {
            if let Some(llms) = fetch_llms_txt(client, &url).await {
                return Ok((llms, 1));
            }
            run_crawler(client, url, cli, retry, sems, cache).await
        }
        _ => Err(RipwebError::Config("unhandled platform route".into())),
    }
}

async fn handle_query(
    client: &Arc<rquest::Client>,
    query: &str,
    cli: &Cli,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    let urls = duckduckgo::search(client, query, 3)
        .await
        .map_err(|e| RipwebError::Network(e.to_string()))?;

    let mut all_pages: Vec<CrawledPage> = Vec::new();

    for url_str in urls {
        if all_pages.len() >= cli.max_pages { break; }

        let url = url::Url::parse(&url_str)
            .map_err(|e| RipwebError::Config(format!("DDG returned invalid URL: {e}")))?;

        let pages = if let Some(llms) = fetch_llms_txt(client, &url).await {
            vec![CrawledPage { url: url_str, content: llms }]
        } else {
            let remaining = cli.max_pages.saturating_sub(all_pages.len());
            Crawler::new(
                Arc::clone(client),
                sems.clone(),
                cache.clone(),
                RetryConfig { max_retries: 2, base_delay: retry.base_delay },
                CrawlerConfig { max_depth: cli.max_depth, max_pages: remaining },
            )
            .crawl(url)
            .await
        };

        all_pages.extend(pages);
    }

    let count = all_pages.len();
    Ok((format_output(&all_pages), count))
}

async fn run_crawler(
    client: &Arc<rquest::Client>,
    url: url::Url,
    cli: &Cli,
    retry: RetryConfig,
    sems: DomainSemaphores,
    cache: Option<Arc<Cache>>,
) -> Result<(String, usize), RipwebError> {
    let crawler = Crawler::new(
        Arc::clone(client),
        sems,
        cache,
        RetryConfig { max_retries: 2, base_delay: retry.base_delay },
        CrawlerConfig { max_depth: cli.max_depth, max_pages: cli.max_pages },
    );
    let pages = crawler.crawl(url).await;
    let count = pages.len();
    Ok((format_output(&pages), count))
}

fn format_reddit(c: &crate::search::reddit::RedditContent) -> String {
    let mut out = format!("# {}\n\n{}", c.title, c.selftext);
    if !c.comments.is_empty() {
        out.push_str("\n\n## Comments\n\n");
        out.push_str(&c.comments.join("\n\n---\n\n"));
    }
    out
}

fn format_hn(c: &crate::search::hackernews::HnContent) -> String {
    let mut out = format!("# {}", c.title);
    if let Some(text) = &c.text {
        out.push_str(&format!("\n\n{text}"));
    }
    if !c.comments.is_empty() {
        out.push_str("\n\n## Comments\n\n");
        out.push_str(&c.comments.join("\n\n---\n\n"));
    }
    out
}
```

- [ ] **Step 2: Add `pub mod run` to src/lib.rs**

```rust
pub mod cli;
pub mod config;
pub mod error;
pub mod extract;
pub mod fetch;
pub mod minify;
pub mod router;
pub mod run;
pub mod search;

pub use error::RipwebError;
```

- [ ] **Step 3: Replace src/main.rs with the thin shell**

```rust
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use is_terminal::IsTerminal;
use tiktoken_rs::cl100k_base;
use tracing_subscriber::EnvFilter;

use ripweb::{
    cli::Cli,
    error::RipwebError,
    fetch::{cache::Cache, client::build_client, politeness::DomainSemaphores, RetryConfig},
    run::{dispatch, apply_output_mode},
};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    setup_tracing(cli.verbose);

    if cli.clean_cache {
        if let Some(dirs) = directories::ProjectDirs::from("", "", "ripweb") {
            let dir = dirs.cache_dir();
            match std::fs::remove_dir_all(dir) {
                Ok(()) => eprintln!("Cache cleared: {}", dir.display()),
                Err(e) if e.kind() == io::ErrorKind::NotFound => eprintln!("Cache already empty."),
                Err(e) => { eprintln!("Error clearing cache: {e}"); std::process::exit(1); }
            }
        } else {
            eprintln!("Could not determine XDG cache directory.");
        }
        return;
    }

    let input = match &cli.query_or_url {
        Some(s) => s.as_str(),
        None => { eprintln!("Error: a URL or query is required."); std::process::exit(1); }
    };

    let is_tty = io::stdout().is_terminal();
    let spinner: Option<ProgressBar> = if is_tty {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.enable_steady_tick(Duration::from_millis(80));
        pb.set_message("Fetching…");
        Some(pb)
    } else {
        None
    };

    let client = Arc::new(match build_client() {
        Ok(c) => c,
        Err(e) => { finish_spinner(&spinner); eprintln!("Error building HTTP client: {e}"); std::process::exit(1); }
    });

    let result = tokio::select! {
        r = dispatch(&cli, input, &client, RetryConfig::default(), DomainSemaphores::new(3), Cache::xdg().map(Arc::new)) => r,
        _ = tokio::signal::ctrl_c() => {
            finish_spinner(&spinner);
            let _ = io::stdout().flush();
            let _ = writeln!(io::stdout());
            std::process::exit(0);
        }
    };

    finish_spinner(&spinner);

    let (text, page_count) = match result {
        Ok(pair) => pair,
        Err(e) => { eprintln!("Error: {e}"); std::process::exit(e.exit_code()); }
    };

    let text = apply_output_mode(text, cli.mode);

    if text.trim().is_empty() {
        eprintln!("Error: {}", RipwebError::NoContent);
        std::process::exit(4);
    }

    if cli.stat {
        let tokens = cl100k_base()
            .map(|bpe| bpe.encode_with_special_tokens(&text).len())
            .unwrap_or(0);
        let size_mb = text.len() as f64 / 1_048_576.0;
        write_stdout(&format!("Pages: {page_count} | Raw Size: {size_mb:.2} MB | Tokens: {tokens}\n"));
        return;
    }

    if cli.copy {
        match arboard::Clipboard::new().and_then(|mut b| b.set_text(&text).map(|_| ())) {
            Ok(()) => eprintln!("Copied to clipboard."),
            Err(e) => { eprintln!("Clipboard error: {e}"); std::process::exit(1); }
        }
        return;
    }

    write_stdout(&text);
    write_stdout("\n");
}

fn write_stdout(text: &str) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    if let Err(e) = handle.write_all(text.as_bytes()) {
        if e.kind() == io::ErrorKind::BrokenPipe { std::process::exit(0); }
        eprintln!("stdout write error: {e}");
        std::process::exit(1);
    }
}

fn finish_spinner(spinner: &Option<ProgressBar>) {
    if let Some(pb) = spinner { pb.finish_and_clear(); }
}

fn setup_tracing(verbose: u8) {
    let level = match verbose { 0 => "warn", 1 => "info", 2 => "debug", _ => "trace" };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("ripweb={level}")));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .init();
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/run.rs src/lib.rs src/main.rs
git commit -m "refactor: extract orchestration into src/run.rs, thin main.rs"
```

---

### Task 8: Update docs/TESTING.md

**Files:**
- Modify: `docs/TESTING.md`

Remove the "Frozen Corpus Workflow" section (Task 7 in the current file) and the "Torture Fixtures" section (Task 8) since the corpus management commands no longer exist. Update the "Example Tooling" references.

- [ ] **Step 1: Open docs/TESTING.md and remove sections 7 and 8**

Delete the entire "## 7. Frozen Corpus Workflow" section (the `corpus/frozen/`, `corpus/reports/` workflow with all the `cargo run --example` commands).

Delete the entire "## 8. Torture Fixtures" section (`generate_fixtures`, `test_torture` commands).

Renumber "## 9. Adding a New Fixture" → "## 7." and "## 10. Definition of Done" → "## 8."

Update "## 2. Directory Layout" to remove `corpus/` block and update `tests/fixtures/` to note that `torture/` subdirectory exists there.

- [ ] **Step 2: cargo test to confirm nothing broke**

```bash
cargo test
```

- [ ] **Step 3: Commit**

```bash
git add docs/TESTING.md
git commit -m "docs: update TESTING.md to reflect removed corpus/examples infrastructure"
```
