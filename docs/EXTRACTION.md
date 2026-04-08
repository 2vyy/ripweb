# Extraction

This document explains how the ripweb extractor works, what it is trying to solve, and how it should evolve. For the output guarantees the extractor must uphold, see [OUTPUT_CONTRACT.md](OUTPUT_CONTRACT.md).

---

## 1. Priority Order

The extractor optimizes for these goals in order:

1. correct content selection
2. stable Markdown structure
3. useful page-type awareness
4. measurable evaluation against a frozen corpus
5. token optimization (later, after Markdown path is stable)

`markdown` is the main product surface. `aggressive` is secondary.

---

## 2. Current Pipeline

The generic extractor lives in `src/extract/web.rs`. It runs in these phases:

1. **Charset decode.** Read the HTTP `Content-Type` charset when present, fall back to `<meta charset>` parsing. Use `encoding_rs` to transcode into UTF-8 before passing bytes to the DOM parser.
2. **Parse DOM.** `tl` is used for fast HTML parsing.
3. **Boilerplate stripping.** Hard nuke tags and negative-hint subtrees are removed before candidate scoring.
4. **Candidate selection.** Plausible content roots are scored using text density, link density, structural signals, and page-family hints.
5. **Markdown rendering.** The selected subtree is rendered into a stable Markdown intermediate representation.
6. **SPA fallback.** When visible DOM extraction produces under 100 words, fall back to `__NEXT_DATA__`-style or Vue/Nuxt JSON payloads.

---

## 3. Boilerplate Stripping

Two classes of noise are removed before candidate scoring.

### Hard nuke tags (removed globally)

`nav`, `footer`, `header`, `aside`, `style`, `svg`, `iframe`, `form`, `script`, `noscript`

### Negative-hint subtrees

Some real-world sites place chrome inside generic `div` or `section` wrappers. Tag-name stripping alone is not enough. Subtrees whose `id` or `class` strongly suggests the following are also stripped:

- nav / menu / sidebar
- cookie / modal / popup
- share / social
- related / recommendation
- utility / newsletter / subscribe
- slider / carousel
- sitemap / toolbar

This is especially important on docs sites and travel/media sites with large global navigation shells.

---

## 4. Candidate Selection

The extractor scores content root candidates rather than simply taking the first `<main>` or `<article>`.

Candidate search order:

1. `main`
2. `article`
3. `section` (fallback)
4. `table` (fallback)
5. hinted `div` containers: `content`, `article`, `post`, `doc`, `markdown`, `story`
6. `body`

### Scoring Heuristics

Each candidate is scored using:

- **Base Density**: `word_count`, `paragraphs` (x24), `headings` (x18), `code_fences` (x20), and `list_items` (x10).
- **Penalties**: `link_count` (-x6) and `short_lines` (-x2).
- **Tag Weighting**: Positive for `article` (+80), `main` (+60), `section` (+20), `div` (+10), `table` (+12); negative for `body` (-40).
- **Attribute Hinting**: Positive (+24) and negative (-60) hints in `id` and `class` attributes.
- **Family Adjustments**: Family-specific score adjustments (e.g., higher weight for code fences in `Docs`).

### Identified Weaknesses

- **Lack of Depth Penalization**: Deeply nested boilerplate can sometimes accumulate high scores without being penalized for its distance from the root.
- **Naive Link Penalty**: Using a flat `link_count` penalty doesn't account for the ratio of links to content, which is a stronger signal for boilerplate.
- **Missing Semantic Boosts**: No explicit boost for high-density text nodes with semantic markers beyond basic tag weighting.
- **Family Misclassification**: Sparse pages can be easily misclassified, leading to suboptimal score adjustments.

---

## 5. Markdown Rendering

The renderer preserves:

- headings (mapped from `h1`–`h6`)
- paragraphs
- links (inline Markdown form; see [OUTPUT_CONTRACT.md §4](OUTPUT_CONTRACT.md))
- unordered and ordered lists
- inline code
- fenced code blocks (indentation preserved exactly)
- blockquotes
- simple tables

Target: a stable, useful Markdown representation for LLM use and downstream parsing. Not perfect HTML fidelity.

---

## 6. Page Families

The extractor classifies pages into families before applying rendering rules. Family detection currently influences candidate scoring. It is evolving toward fully separate rendering pipelines per family.

### Current families

| Family | Primary signals | Status |
|---|---|---|
| `Docs` | many headings, code fences, internal links, doc/reference hints | active, scoring influence |
| `Article` | long prose paragraphs, article/story hints, moderate link density | active, scoring influence |
| `Generic` | fallback when no stronger family detected | active |

### Planned families

| Family | Primary signals |
|---|---|
| `Listing` | repeated result-card structures, high clustered link density |
| `Product` | price patterns, spec tables, product schema hints |
| `Forum` | repeated answer/comment blocks, score/vote markers, author+timestamp clusters |
| `Utility` | forms, login walls, redirects, error pages, CAPTCHA interstitials |

### Family detection signals

**Docs:**

- many headings
- fenced or preformatted code
- many internal relative links
- doc/reference/tutorial class or id hints

Examples: MDN, docs.rs, React docs, framework API documentation

**Article:**

- longer prose paragraphs
- article/story/post/news hints
- moderate link density
- clear body flow

**Listing:**

- repeated sibling block structure
- medium-length repeated units
- high overall link density clustered into repeated units

**Product:**

- price patterns
- spec tables or bullet lists
- repeated recommendation cards
- product schema or commerce hints

Examples: Amazon, Walmart, Target

**Forum:**

- repeated answer/comment blocks
- score/vote markers
- author + timestamp clusters

Examples: Stack Overflow, Reddit threads, discussion forums

**Utility:**

- forms, login inputs
- interstitial/redirect patterns
- minimal content, high boilerplate

---

## 7. Domain Hints vs Site-Specific Parsers

The preferred order of sophistication:

1. family-level heuristics
2. domain hints (from `config/ripweb.toml`)
3. site-specific extractors

Domain hints allow known sites to skip generic family detection:

- `docs.rs`, `developer.mozilla.org`, `react.dev` → `docs` family
- `amazon.com`, `walmart.com`, `target.com` → `product` family

Site-specific parsers are reserved for cases where family-level extraction still fails materially. Do not add a site-specific extractor before exhausting family-level improvements.

See [CONFIGURATION.md](CONFIGURATION.md) for how domain hints are configured.

---

## 8. Lessons from Comparable Tools

Mozilla Readability, jusText, goose, and Trafilatura show that generic extraction improves when these are combined:

- boilerplate removal
- content density heuristics
- link-density heuristics
- page-family aware extraction rules
- deterministic fixture-based evaluation

Borrow the ideas, not necessarily the exact implementation.

---

## 9. Current Weak Spots

The extractor is currently weakest on:

- docs pages with duplicated sidebar trees
- search/listing pages that should not be rendered like articles
- product pages where recommendation cards swamp the main content
- forum/discussion pages where repeated comment blocks need ranking and truncation rules
- pages whose best content root is nested inside wrapper-heavy layout chrome

---

## 10. Implementation Order

1. Strengthen generic candidate scoring with better density and link heuristics
2. Add `Listing` family detection
3. Add `Listing` family rendering rules
4. Add `Product` family detection and rendering rules
5. Add `Forum` family detection and rendering rules
6. Strengthen `Docs` classification and docs-specific sidebar stripping
7. Revisit aggressive mode only after the Markdown path is stable

---

## 11. Evaluation

Extraction changes must be validated against the frozen corpus before merging. See [TESTING.md](TESTING.md) for the full evaluation workflow.

Key evaluation files:

- `FORMAT.md` → now [OUTPUT_CONTRACT.md](OUTPUT_CONTRACT.md)
- `src/corpus.rs` — fixture manifest
- `corpus/reports/bulk_extract_report.md` — latest bulk run
- `tests/expected/curated/` — human-curated reference outputs
- `tests/snapshots/` — insta snapshots
