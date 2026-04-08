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

The extraction pipeline is split into a **Router** phase and a **Fetcher** phase to optimize for structured data whenever possible.

1. **Routing.** `src/router.rs` classifies the input URL. If it matches a known high-fidelity platform (GitHub, Reddit, Wikipedia, etc.), it is routed to a specialized extractor.
2. **Probe Sequence (Generic only).** If routed to `Generic`, we attempt to find non-HTML representations:
    - **Markdown suffix**: Try `url + ".md"` or `url + "/index.html.md"`.
    - **llms.txt**: Try site-level `llms.txt` or `.well-known/llms.txt` indexes.
3. **Fetching.**
    - If a probe hits, return the Markdown immediately.
    - Otherwise, fetch the HTML or call the platform's REST API.
4. **Extraction.**
    - **Platform-Specific**: Parse the REST JSON/XML response into structured Markdown.
    - **Generic**: Run the HTML pipeline (stripping, candidate scoring, rendering).
5. **Post-Processing.** Apply family-aware rules (e.g., ranking forum answers, stripping sidebars).
6. **Fallback.** If generic extraction produces < 150 words, proxy the request through **Jina.ai Reader** as a last resort.

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

This is especially important on docs sites and travel/media sites with large global navigation shells. We heavily target Amazon-style e-commerce noise here (`similarities`, `customers-who`).

### Block-Level Pruning (Link Saturation)

During DOM-to-Markdown rendering, block elements (`div`, `section`, `aside`) are dynamically evaluated for **link saturation**. If a block's `link_chars / total_chars  > 0.4`, it is dropped as navigational/recommendation boilerplate.
- **Product Safety**: Technical `<table>` elements within product pages are shielded from density limits to ensure specification preservation.
- **Listing Safety**: Pages classified as `Listing` are exempted from density reduction to preserve index fidelity.

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
| `Docs` | many headings, code fences, internal links, doc/reference hints | active, scoring influence, sidebar pruning |
| `Article` | long prose paragraphs, article/story hints, moderate link density | active, scoring influence |
| `Listing` | repeated result-card structures, high clustered link density | active, scoring influence & pruning exemption |
| `Product` | price patterns, spec tables, product schema hints | active, scoring influence & carousel blanking |
| `Generic` | fallback when no stronger family detected | active |

### Planned families

| Family | Primary signals |
|---|---|
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

See [CONFIGURATION.md](CONFIGURATION.md) for how domain hints are configured.

---

## 8. Platform-Specific Extractors (v0.5)

To maximize signal-to-noise ratio, `ripweb` uses keyless REST APIs for these high-value platforms:

- **Wikipedia**: Uses the REST v1 summary API (`/api/rest_v1/page/summary/`). Returns clean article extracts with metadata.
- **StackOverflow**: Uses Stack Exchange API v2.3. Fetches question details and answers in parallel. Answers are ranked by `Accepted` status and then `Score` descending.
- **ArXiv**: Uses the Atom export query API. Extracts title, authors, published date, and paper abstract.
- **Reddit**: Appends `.json` to thread URLs to get the structured comment tree (filtering for score > 0).
- **HackerNews**: Uses the Algolia HN API (`/api/v1/items/`) for consistent comment retrieval.
- **GitHub**: Uses the `raw.githubusercontent.com` proxy for READMEs and the REST API for public Issues/Comments.
- **YouTube**: Uses oEmbed for video metadata and the `timedtext` API for full timestamped transcripts extracted from the page's embedded `captionTracks` JSON.
- **Twitter/X**: Uses the unauthenticated `publish.twitter.com` oEmbed endpoint to retrieve and clean tweet text from the embedded blockquote.
- **TikTok**: Uses the public oEmbed endpoint for the highest-fidelity keyless metadata available (title and creator).

---

## 9. Probe Sequence

Sites built with modern documentation tools (e.g., Mintlify, nbdev) often serve native Markdown. `ripweb` probes for these before attempting to scrape HTML:

1. **Suffix Probe**: `<url>.md`
2. **Index Probe**: `/llms.txt` or `/.well-known/llms.txt`

The probe only accepts `text/markdown` or `text/plain` responses to avoid false positives from HTML "Not Found" pages.

---

## 10. Universal Fallback (Jina.ai)

When local extraction fails (under 150 words extracted) and no specialized API is available, `ripweb` proxies the request via `https://r.jina.ai/`. This reader handles complex JS-heavy rendering and provides high-quality Markdown for edge cases.

---

---

## 11. Lessons from Comparable Tools

Mozilla Readability, jusText, goose, and Trafilatura show that generic extraction improves when these are combined:

- boilerplate removal
- content density heuristics
- link-density heuristics
- page-family aware extraction rules
- deterministic fixture-based evaluation

Borrow the ideas, not necessarily the exact implementation.

---

## 12. Current Weak Spots

The extractor is currently weakest on:

- pages whose best content root is nested inside wrapper-heavy layout chrome
- single-page applications utilizing heavy client-side Shadow DOMs (requires headless browser fallback)
- sparse generic utility pages (e.g. login walls) where standard text heuristics fail to lock onto a central `main` block

---

## 13. Evaluation

Extraction changes must be validated against the frozen corpus before merging. See [TESTING.md](TESTING.md) for the full evaluation workflow.

Key evaluation files:

- `src/corpus.rs` — fixture manifest
- `corpus/reports/bulk_extract_report.md` — latest bulk run
- `tests/expected/curated/` — human-curated reference outputs
- `tests/snapshots/` — insta snapshots
