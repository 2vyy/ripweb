# Output Contract

This document defines exactly what `ripweb` guarantees to emit based on the selected **Output Mode** and its corresponding **Density Tier**. It is the source of truth for extraction, rendering, tests, and benchmarks.

---

## 1. Design Goal

`ripweb` modulates information density to balance **context window usage** against **detail requirements**.

The CLI provides 7 modes that map into 3 underlying Density Tiers:

- **Tier 1 (Nucleus)**: High-speed discovery. Minimalist, list-based output for broad scanning. Modes: `omega-compact`, `compact`.
- **Tier 2 (Signal)**: Standard research. Balanced snippets, summaries, and capped prose. Modes: `balanced`, `detailed`.
- **Tier 3 (Full Context)**: Exhaustive detail. Full structured content, transcripts, and comments. Modes: `verbose`, `omega-verbose`, `aggressive`.

---

## 2. Density Tiers (Verbosity Levels)

### Tier 1: Nucleus
**Goal**: Identify sources and core headlines with near-zero token overhead.

- **Generic Web**: `- [Page Title](URL)` only.
- **Search (SERP)**: List of `- [Title](URL)`.
- **Platforms**:
    - **GitHub**: List of Issue Titles + Numbers + Labels.
    - **Reddit**: Post Title + [Score/Subreddit].
    - **Wikipedia**: Definition sentence only.
    - **YouTube**: Basic Meta (Title/Author/Duration).
    - **StackOverflow**: Question Title + Link.

### Tier 2: Signal (Default)
**Goal**: Understand the value of a source via snippets and primary summaries.

- **Generic Web**: Title + URL + first ~2000 characters of extracted text.
- **Search (SERP)**: `- [Title](URL) \n  > {snippet}`.
- **Platforms**:
    - **GitHub**: Issue Title + OP's Description.
    - **Reddit**: Post Body + Top 2 Upvoted Comments.
    - **Wikipedia**: Full Lead Section + Infobox data.
    - **YouTube**: Basic Meta + Video Description.
    - **StackOverflow**: Question Title + Highest Voted Answer.

### Tier 3: Full Context
**Goal**: Comprehensive data retrieval for deep troubleshooting or analysis.

- **Generic Web**: Full rehydrated Markdown (forced via **Jina Reader** proxy).
- **Search (SERP)**: Detailed cards with Engine, Date, and longer Snippets.
- **Platforms**:
    - **GitHub**: Issue Title + OP Description + **All Comments**.
    - **Reddit**: Post Body + **Full Comment Tree**.
    - **Wikipedia**: **Full Article** Markdown (Generic fetch).
    - **YouTube**: Basic Meta + **Full Transcripts** (if available).
    - **StackOverflow**: Question Title + **All Answers**.

---

## 3. Global Output Structure

### Source Delimiters
Every document/page in a multi-page crawl or search is separated with a source delimiter:

```md
# --- [Source: https://example.com/page] ---
```

- Source delimiters appear **before** page content.
- URLs are normalized (fragments removed, tracking parameters stripped).

---

## 4. Element Contract (General)

Regardless of output mode, when prose is emitted, it follows these rules:

### Headings
- `h1` → `#`, `h2` → `##`, down to `h6`.
- Headings remain separate blocks (`\n\n` separation).

### Code Blocks
- Preformatted code renders as fenced Markdown code blocks (```).
- Indentation and internal whitespace are preserved exactly.

### Links
- Rendered as `[label](url)`.
- If label is missing, URL is used as label.

---

## 5. Content Inclusion Rules

**Keep**:
- Main article/documentation body.
- API signatures and code examples.
- High-voted answers and verified comments.

**Drop**:
- Navigation menus, headers/footers, and cookie banners.
- Social share widgets and "Recommended for you" rails.
- E-commerce Noise: "Customers also bought", "Similar items", and sponsored carousels.
- Analytics scripts and presentational SVG/iframes.

---

## 6. Evaluation Criteria

Extraction changes are judged on:
1. **Structural Fidelity**: Headings, lists, and code blocks must remain valid Markdown.
2. **Signal Retention**: Key information for the given density tier must be present.
3. **Noise Removal**: Boilerplate must be cleanly stripped.
