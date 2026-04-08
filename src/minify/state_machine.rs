/// Whitespace character categories understood by the state machine.
#[derive(Clone, Copy, PartialEq, Eq)]
enum WsKind {
    None,
    Space, // spaces / tabs / \r only
    Line(u8),  // one or two pending newlines
}

/// State of the minification pass.
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Normal,
    /// Inside a Markdown fenced code block (``` ... ```).
    /// We pass bytes through verbatim to preserve indentation.
    CodeFence,
}

// ── Base64 / hash detection ───────────────────────────────────────────────────

/// Minimum token length before we consider it "might be base64 / a hash".
const B64_MIN_LEN: usize = 80;
/// Hex hashes (SHA-1 = 40, SHA-256 = 64) — shorter strings are real words.
const HEX_MIN_LEN: usize = 40;
const HEX_MAX_LEN: usize = 64;

/// Returns `true` if every character is in the base64 alphabet (incl. `=` padding).
fn is_base64_token(s: &str) -> bool {
    s.len() >= B64_MIN_LEN
        && s.bytes().all(|b| {
            b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'='
        })
}

/// Returns `true` if the token looks like a lowercase or uppercase hex hash.
fn is_hex_hash(s: &str) -> bool {
    let n = s.len();
    (HEX_MIN_LEN..=HEX_MAX_LEN).contains(&n)
        && s.bytes().all(|b| b.is_ascii_hexdigit())
}

// ── Main state machine ────────────────────────────────────────────────────────

/// Single-pass whitespace collapser and token annihilator.
///
/// Guarantees:
/// * Output is never longer than input.
/// * Content inside Markdown ``` fences is emitted verbatim (indentation preserved).
/// * Runs of whitespace are collapsed to one space (or one `\n` if the run
///   contained a single newline / one `\n\n` if the run contained two or more).
/// * Tokens matching base64 patterns (≥ 80 chars) are replaced with
///   `[BASE64_TRUNCATED]`.
/// * Hex strings 40–64 chars long are replaced with `[SHA_TRUNCATED]`.
pub fn collapse(input: &str) -> String {
    let simplified = simplify_markdown_for_aggressive(input);
    let collapsed = collapse_whitespace_and_tokens(&simplified);
    let baseline = input.trim();
    if collapsed.len() > baseline.len() {
        baseline.to_owned()
    } else {
        collapsed
    }
}

fn collapse_whitespace_and_tokens(input: &str) -> String {
    let mut out = String::with_capacity(input.len().saturating_sub(input.len() / 4));
    let mut state = State::Normal;
    let mut pending_ws = WsKind::None;
    let mut fence_ticks: u8 = 0; // consecutive backticks seen
    let mut token = String::new();

    macro_rules! flush_token {
        () => {
            if !token.is_empty() {
                if is_base64_token(&token) {
                    out.push_str("[BASE64_TRUNCATED]");
                } else if is_hex_hash(&token) {
                    out.push_str("[SHA_TRUNCATED]");
                } else {
                    out.push_str(&token);
                }
                token.clear();
            }
        };
    }

    macro_rules! flush_ws {
        () => {
            match pending_ws {
                WsKind::None => {}
                WsKind::Space => out.push(' '),
                WsKind::Line(count) => {
                    for _ in 0..count.min(2) {
                        out.push('\n');
                    }
                }
            }
            pending_ws = WsKind::None;
        };
    }

    for ch in input.chars() {
        match state {
            // ── Inside a code fence — emit everything verbatim ──────────────
            State::CodeFence => {
                out.push(ch);
                if ch == '`' {
                    fence_ticks += 1;
                    if fence_ticks == 3 {
                        state = State::Normal;
                        fence_ticks = 0;
                    }
                } else {
                    fence_ticks = 0;
                }
            }

            // ── Normal text ──────────────────────────────────────────────────
            State::Normal => {
                if ch == '`' {
                    fence_ticks += 1;
                    if fence_ticks == 3 {
                        // Entering a code fence: flush buffered work first.
                        flush_token!();
                        flush_ws!();
                        out.push_str("```");
                        fence_ticks = 0;
                        state = State::CodeFence;
                    }
                } else {
                    if fence_ticks > 0 {
                        for _ in 0..fence_ticks {
                            token.push('`');
                        }
                        fence_ticks = 0;
                    }

                    if ch == '\n' || ch == '\r' || ch == '\t' || ch == ' ' {
                        flush_token!();
                        if ch == '\n' {
                            pending_ws = match pending_ws {
                                WsKind::Line(count) => WsKind::Line((count + 1).min(2)),
                                _ => WsKind::Line(1),
                            };
                        } else if pending_ws == WsKind::None {
                            pending_ws = WsKind::Space;
                        }
                    } else {
                        flush_ws!();
                        token.push(ch);
                    }
                }
            }
        }
    }

    // Drain anything remaining.
    if state == State::Normal && fence_ticks > 0 {
        for _ in 0..fence_ticks {
            token.push('`');
        }
    }
    flush_token!();
    // Do NOT emit trailing whitespace.

    out.trim_start_matches('\n').to_owned()
}

fn simplify_markdown_for_aggressive(input: &str) -> String {
    let mut out = String::new();
    let mut in_code_fence = false;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            out.push_str(line);
            out.push('\n');
            continue;
        }

        if in_code_fence {
            out.push_str(line);
            out.push('\n');
            continue;
        }

        if is_low_value_ui_line(trimmed) {
            continue;
        }

        let mut simplified = strip_decorative_heading_anchor(line);
        simplified = simplify_low_value_links(&simplified);
        out.push_str(simplified.trim_end());
        out.push('\n');
    }

    out
}

fn is_low_value_ui_line(line: &str) -> bool {
    matches!(line, "Copy item path" | "Expand description")
}

fn strip_decorative_heading_anchor(line: &str) -> String {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return line.to_owned();
    }

    line.replace("[§](#", "§](#")
        .split_once("§](#")
        .and_then(|(prefix, rest)| rest.split_once(')').map(|(_, suffix)| format!("{prefix}{suffix}")))
        .unwrap_or_else(|| line.to_owned())
}

fn simplify_low_value_links(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let bytes = line.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] != b'[' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }

        let Some(label_end) = line[i + 1..].find(']') else {
            out.push('[');
            i += 1;
            continue;
        };
        let label_end = i + 1 + label_end;
        if bytes.get(label_end + 1) != Some(&b'(') {
            out.push('[');
            i += 1;
            continue;
        }
        let Some(href_end_rel) = line[label_end + 2..].find(')') else {
            out.push('[');
            i += 1;
            continue;
        };
        let href_end = label_end + 2 + href_end_rel;
        let label = &line[i + 1..label_end];
        let href = &line[label_end + 2..href_end];

        if should_inline_link_label_only(label, href) {
            out.push_str(label);
        } else {
            out.push_str(&line[i..=href_end]);
        }
        i = href_end + 1;
    }

    out
}

fn should_inline_link_label_only(label: &str, href: &str) -> bool {
    if label.is_empty() || href.is_empty() {
        return false;
    }
    if href.starts_with('#') {
        return true;
    }
    if label == href {
        return true;
    }
    // Internal relative documentation links tend to be high-overhead and low-value in aggressive mode.
    !href.contains("://") && !href.starts_with('/') && !href.starts_with("mailto:")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_spaces() {
        assert_eq!(collapse("hello   world"), "hello world");
    }

    #[test]
    fn collapses_mixed_whitespace_to_space() {
        assert_eq!(collapse("a \t b"), "a b");
    }

    #[test]
    fn newline_wins_over_space_in_whitespace_run() {
        // A run containing a newline must collapse to \n, not a space.
        assert_eq!(collapse("a   \n   b"), "a\nb");
    }

    #[test]
    fn output_never_longer_than_input() {
        let input = "hello   world\n\n\nfoo";
        assert!(collapse(input).len() <= input.len());
    }

    #[test]
    fn idempotent_on_already_collapsed_text() {
        let text = "hello world\nfoo bar";
        assert_eq!(collapse(text), collapse(&collapse(text)));
    }

    #[test]
    fn preserves_double_newlines_for_paragraph_breaks() {
        assert_eq!(collapse("a\n\nb"), "a\n\nb");
    }

    #[test]
    fn collapses_three_or_more_newlines_to_two() {
        let result = collapse("a\n\n\n\nb");
        assert_eq!(result, "a\n\nb");
    }

    #[test]
    fn no_trailing_whitespace() {
        let result = collapse("hello   ");
        assert!(!result.ends_with(' '), "trailing space: {result:?}");
    }

    #[test]
    fn base64_token_is_truncated() {
        // 88-char base64 string
        let b64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/ABCDEFGHIJKLMNOPQRST==";
        let input = format!("before {b64} after");
        let result = collapse(&input);
        assert!(result.contains("[BASE64_TRUNCATED]"), "got: {result}");
        assert!(!result.contains(b64), "raw base64 leaked");
    }

    #[test]
    fn short_base64_like_token_is_kept() {
        // 16 chars — below the 80-char threshold, should pass through
        let token = "SGVsbG8gV29ybGQ=";
        assert_eq!(collapse(token), token);
    }

    #[test]
    fn sha256_hash_is_truncated() {
        let sha = "a3f5b2c1d4e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2";
        let input = format!("commit {sha} updated");
        let result = collapse(&input);
        assert!(result.contains("[SHA_TRUNCATED]"), "got: {result}");
    }

    #[test]
    fn code_fence_content_is_preserved_verbatim() {
        let input = "text before\n```\n    indented_code()\n    more = true\n```\nafter";
        let result = collapse(input);
        assert!(result.contains("    indented_code()"), "indentation lost: {result}");
        assert!(result.contains("    more = true"), "indentation lost: {result}");
    }

    #[test]
    fn fence_entry_emits_exactly_three_backticks() {
        let result = collapse("before ```\ncode\n``` after");
        assert!(result.contains("```"), "missing fence: {result}");
        assert!(!result.contains("``````"), "duplicated fence ticks: {result}");
    }

    #[test]
    fn empty_input_produces_empty_output() {
        assert_eq!(collapse(""), "");
    }

    #[test]
    fn pure_whitespace_produces_empty_output() {
        assert_eq!(collapse("   \n\t  \r\n  "), "");
    }

    #[test]
    fn strips_decorative_heading_anchor_in_aggressive_mode() {
        let input = "## [§](#routing) Routing";
        assert_eq!(collapse(input), "## Routing");
    }

    #[test]
    fn strips_low_value_internal_markdown_links() {
        let input = "See [`Router`](struct.Router.html) and [section](#routing).";
        assert_eq!(collapse(input), "See `Router` and section.");
    }

    #[test]
    fn keeps_external_markdown_links() {
        let input = "See [Fetch API](https://example.com/docs?id=42).";
        assert_eq!(collapse(input), input);
    }

    #[test]
    fn drops_copy_ui_lines_outside_code_fences() {
        let input = "Copy item path\n\n# Title\n\nExpand description\nBody";
        assert_eq!(collapse(input), "# Title\n\nBody");
    }
}
