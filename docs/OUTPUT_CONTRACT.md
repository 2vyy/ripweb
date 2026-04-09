# Output Contract

This document is the **canonical specification** for every byte `ripweb` emits. It defines exactly what each `--mode` guarantees in terms of structure, content density, token count, and platform-specific behavior. All tests, benchmarks, and downstream LLM pipelines must conform to this contract.

---

## 1. Design Goal

`ripweb` modulates information density **solely** to optimize two metrics:
- **Token efficiency** (measured with production tokenizers: `cl100k_base` for GPT-4o-class models and `o200k_base` for newer frontier models)
- **LLM parseability** (consistent, semantic Markdown that maximizes retrieval quality in RAG pipelines)

Higher modes trade token budget for richer context. Every mode is deterministic, versioned, and benchmarked against a frozen corpus.

---

## 2. Output Modes

| Mode              | Approx. Tokens per Result | CLI Flag                  | Best For                              | Jina Fallback | Search Depth |
|-------------------|---------------------------|---------------------------|---------------------------------------|---------------|--------------|
| **omega-compact** | 40–80                    | `--mode omega-compact`    | Ultra-fast search, massive result lists, tiny context windows | No            | 1            |
| **compact**       | 120–250                  | `--mode compact`          | Quick link scanning, minimal RAG      | No            | 1            |
| **balanced**      | 350–650                  | `--mode balanced` (default) | Everyday research and most LLM pipelines | No            | 1            |
| **detailed**      | 800–1,800                | `--mode detailed`         | Contextual understanding without full pages | No            | 1            |
| **verbose**       | 2k–6k                    | `--mode verbose`          | Deep single-page analysis             | Optional      | 1            |
| **omega-verbose** | 8k–25k+                  | `--mode omega-verbose`    | Maximum signal on every top result (your requested "omega verbose query lists") | Yes (always)  | 1 (queries) / optional 2 with `--depth 2` |
| **aggressive**    | 15k+                     | `--mode aggressive`       | JS-heavy, protected, or transcript-heavy pages | Yes (forced)  | 1            |

**Token counting note**: Token counts are calculated post-rendering using the exact tokenizer your downstream LLM uses. A header is always emitted:
```md
# ripweb output
# Mode: balanced • Estimated tokens: 4,872 (cl100k_base) • 8 results
```

---

## 3. Mode-Specific Content Rules

### omega-compact & compact
- **Generic Web / Search (SERP)**: `- [Page Title](URL)` only.
- **Platforms**:
  - GitHub: Issue Titles + Numbers + Labels
  - Reddit: Post Title + [Score/Subreddit]
  - Wikipedia: Definition sentence only
  - YouTube: Title / Author / Duration
  - StackOverflow: Question Title + Link

### balanced & detailed
- **Generic Web**: Title + URL + first ~2000 characters of extracted text (balanced) or full snippet + key excerpts (detailed).
- **Search (SERP)**: `- [Title](URL)\n> {snippet}` (balanced) or detailed cards with engine, date, and longer snippets (detailed).
- **Platforms**:
  - GitHub: Issue Title + OP Description
  - Reddit: Post Body + Top 2 Upvoted Comments
  - Wikipedia: Full Lead Section + Infobox data
  - YouTube: Basic Meta + Video Description
  - StackOverflow: Question Title + Highest Voted Answer

### verbose & omega-verbose
- **Generic Web**: Full structured Markdown extraction from the page.
- **Search (SERP)**: Full extraction on **every** top result (omega-verbose) or per-result full content (verbose).
- **Platforms**:
  - GitHub: Issue Title + OP Description + All Comments
  - Reddit: Post Body + Full Comment Tree
  - Wikipedia: Full Article Markdown
  - YouTube: Basic Meta + Full Transcripts (if available)
  - StackOverflow: Question Title + All Answers

### aggressive
- Forces Jina Reader rehydration on every page.
- Maximum density: full transcripts, comments, tables, code blocks, and any available structured data.
- Used when `--mode aggressive` is explicitly requested or when the page requires JS rendering.

---

## 4. Global Output Structure

Every multi-result output (search or crawl) separates documents with a standardized source delimiter:

```md
# --- [Source: https://example.com/page] ---
```

- Delimiters appear **before** each page’s content.
- URLs are normalized (fragments removed, UTM/tracking parameters stripped).
- For single-URL inputs, the delimiter is still emitted for consistency.

---

## 5. Element Contract (All Modes)

When content is emitted, the following rules are **strictly enforced**:

### Headings
- `h1` → `#`, `h2` → `##`, … `h6` → `######`
- Always separated by blank lines (`\n\n`).

### Code Blocks
- Rendered as fenced Markdown blocks (````language`).
- Indentation and internal whitespace are preserved exactly.

### Links
- Rendered as `[label](url)`.
- If no label exists, the URL itself is used as the label.

### Lists & Tables
- Bullet/numbered lists and tables are preserved with full semantic fidelity.

---

## 6. Content Inclusion Rules

**Always Keep**:
- Main article / documentation body
- API signatures and code examples
- High-voted answers and verified comments
- Semantic structure (headings, tables, lists, code)

**Always Drop**:
- Navigation menus, headers, footers, cookie banners
- Social share widgets and “Recommended for you” sections
- E-commerce noise (“Customers also bought”, sponsored carousels)
- Analytics scripts, presentational SVGs, iframes

---

## 7. Evaluation Criteria

All extraction changes are judged on three axes (in order of priority):
1. **Structural Fidelity** — Headings, lists, tables, and code blocks must remain valid, parseable Markdown.
2. **Signal Retention** — All information required for the selected mode must be present.
3. **Noise Removal** — Boilerplate must be cleanly stripped without losing context.

This contract is versioned and referenced in every integration test and benchmark run.

---

**Last updated**: April 2026  
**Status**: NOT YET ENFORCED
```