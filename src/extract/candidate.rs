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
        total_text_len: text.len(),
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

pub fn extract_best_candidate(dom: &tl::VDom, family: PageFamily) -> String {
    let parser = dom.parser();
    let mut best: Option<ScoredCandidate> = None;

    for selector in CONTENT_ROOTS {
        if let Some(hits) = dom.query_selector(selector) {
            for handle in hits {
                let Some(node) = handle.get(parser) else { continue };
                if let Some(tag) = node.as_tag() {
                    consider_candidate(&mut best, score_candidate(tag, parser, family, 0));
                }
            }
        }
    }
    for selector in FALLBACK_CANDIDATE_TAGS {
        if let Some(hits) = dom.query_selector(selector) {
            for handle in hits.take(MAX_FALLBACK_CANDIDATES_PER_SELECTOR) {
                let Some(node) = handle.get(parser) else { continue };
                if let Some(tag) = node.as_tag() {
                    consider_candidate(&mut best, score_candidate(tag, parser, family, 1));
                }
            }
        }
    }
    for selector in HINTED_DIV_SELECTORS {
        if let Some(hits) = dom.query_selector(selector) {
            for handle in hits.take(MAX_FALLBACK_CANDIDATES_PER_SELECTOR) {
                let Some(node) = handle.get(parser) else { continue };
                if let Some(tag) = node.as_tag() {
                    // hinted divs can be at any depth; assign a moderate baseline depth
                    consider_candidate(&mut best, score_candidate(tag, parser, family, 2));
                }
            }
        }
    }
    if let Some(body_handle) = dom.query_selector("body").and_then(|mut hits| hits.next()) {
        if let Some(body_tag) = body_handle.get(parser).and_then(|node| node.as_tag()) {
            consider_candidate(&mut best, score_candidate(body_tag, parser, family, 5));
        }
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
    family: PageFamily,
    // depth: approximate search-depth from document root — penalises wrapper-heavy nesting.
    // Pass 0 for `<main>`/`<article>` (highest priority), larger values for fallbacks.
    depth: u32,
) -> Option<ScoredCandidate> {
    let name = tag.name().as_utf8_str().to_ascii_lowercase();
    if should_strip_subtree(tag) || NUKE_TAGS.contains(&name.as_str()) {
        return None;
    }

    let text = cleanup_markdown(&render_tag(tag, parser));
    if text.is_empty() { return None; }

    let stats = score_text(&text);
    if stats.word_count == 0 { return None; }

    let candidate_family = classify_candidate_family(tag, &text, &stats, family);
    let price_markers = count_price_markers(&text);
    let spec_markers = count_spec_markers(&text);

    let mut score = stats.word_count as i64;
    score += (stats.paragraphs as i64) * 24;
    score += (stats.headings as i64) * 18;
    score += (stats.code_fences as i64) * 20;
    score += (stats.list_items as i64) * 10;

    // Link density penalty: prioritize nodes with high text-to-link ratio
    let link_ratio = if stats.total_text_len > 0 {
        (stats.link_count as f64 * 30.0) / stats.total_text_len as f64
    } else {
        0.0
    };
    if link_ratio > 0.4 {
        score -= (score as f64 * (link_ratio.min(1.0))) as i64;
    }
    score -= (stats.link_count as i64) * 4;
    score -= (stats.short_lines as i64) * 2;

    // Depth penalty: candidates deeper than the immediate document children
    // are likely nested inside layout wrappers. Each level past depth 3 costs points.
    let depth_penalty = (depth.saturating_sub(3) as i64) * 12;
    score -= depth_penalty;

    score += match name.as_str() {
        "article" => {
            let mut boost = 80;
            if stats.word_count > 150 { boost += 40; }
            boost
        }
        "main" => {
            let mut boost = 60;
            if stats.word_count > 150 { boost += 30; }
            boost
        }
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

    if hint_text.contains("main-content") || hint_text.contains("primary-content") {
        score += 35;
    }

    for hint in POSITIVE_HINTS {
        if hint_text.contains(hint) { score += 24; }
    }
    for hint in NEGATIVE_HINTS {
        if hint_text.contains(hint) { score -= 60; }
    }

    score += family_score_adjustment(candidate_family, &stats, price_markers, spec_markers);

    Some(ScoredCandidate { score, text })
}

fn ordered_list_prefix(line: &str) -> bool {
    let digits = line.bytes().take_while(|b| b.is_ascii_digit()).count();
    digits > 0 && line[digits..].starts_with(". ")
}
