# Output Contract

This document defines exactly what `ripweb` guarantees to emit. It is the source of truth for extraction, rendering, minification, tests, and benchmarks. If any of those disagree with this file, this file wins.

---

## 1. Design Goal

`ripweb` has two related but distinct jobs:

- preserve useful structure for LLM research and downstream parsing
- optionally compress that structure aggressively when token efficiency matters more than presentation

The default contract is not "smallest possible output". It is "structured, trustworthy, and useful for downstream parsing."

For the current phase of the project, Markdown extraction quality takes priority over token optimization experiments.

---

## 2. Output Modes

### `markdown` (default)

Guarantees:

- document structure is preserved when meaningfully represented in the source
- headings, paragraphs, links, inline code, fenced code blocks, lists, and source boundaries are preserved
- obvious page chrome and tracking junk are removed
- output is readable by both humans and LLMs
- output is stable enough for snapshot and golden testing

This mode is the canonical intermediate representation for generic web extraction. It is the main product surface.

### `aggressive`

Operates on top of the Markdown representation.

Guarantees:

- main information content is preserved as much as practical
- paragraph breaks and fenced code blocks are preserved
- token count is reduced aggressively through whitespace collapse and large-token truncation

This mode is allowed to reduce presentation quality if it materially improves token efficiency. It must not drive core parser decisions until the Markdown path is stable.

A transform that merely looks shorter is not valid. OpenAI tokenizer measurements against the evaluation corpus decide whether a transform belongs in aggressive mode.

---

## 3. Global Output Structure

### Single page

```md
# Title

Intro paragraph.

## Section

Body content.
```

### Multi-page crawl

Every page is separated with a source delimiter:

```md
# --- [Source: https://example.com/page] ---
```

Rules:

- source delimiters appear before page content, never after
- source URLs have fragments removed
- source URLs have tracking parameters stripped when safe
- delimiters are part of the stable public output contract and must not be omitted or reformatted

---

## 4. Element Contract

### Titles and headings

- page titles and HTML headings map to Markdown headings
- `h1` → `#`, `h2` → `##`, down through `h6`
- headings remain separate blocks; they are never flattened into body text

### Paragraphs

- prose paragraphs remain separate blocks
- paragraph separation is `\n\n`
- aggressive mode may collapse `3+` blank lines to `\n\n` but must preserve `\n\n`

### Links

- links are rendered inline as Markdown links when both label and target exist
- tracking parameters are stripped when doing so does not change the semantic destination
- if link text is missing, the URL is used as the label

Preferred form:

```md
[Fetch API](https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API)
```

Not permitted in default mode:

- link dictionaries at the bottom of the page
- XML wrappers around links
- dropping the URL entirely when it carries real value

### Lists

- unordered lists use `-`
- ordered lists use `1.`, `2.`, `3.`
- list items remain grouped as lists; they are never flattened into prose

### Inline code

- inline code uses backticks
- aggressive mode preserves inline code markers when practical

### Code blocks

- preformatted code renders as fenced Markdown code blocks
- indentation inside fenced blocks is preserved exactly
- aggressive mode must not collapse whitespace inside fenced code blocks
- whitespace collapsing must pause while iterating over `<pre>`, `<code>`, or Markdown code block content

### Blockquotes

- blockquotes use Markdown `>`

### Tables

- a table that can be preserved in Markdown without major corruption is preserved
- a table that is too malformed or purely presentational degrades gracefully to readable text rather than emitting broken pseudo-tables

### Images

- decorative images are ignored
- meaningful images may be represented as text placeholders using alt text when that adds value
- binary image payloads are not part of the text contract

---

## 5. Content Inclusion Rules

Prefer keeping:

- main article, documentation, or tutorial content
- meaningful headings and section labels
- code examples
- important links
- list structure
- author and date metadata when nearby and clearly content-relevant

Prefer dropping:

- navigation
- headers and footers
- cookie banners
- repeated sidebars
- social share blocks
- recommendation rails
- forms and unrelated controls
- analytics and tracking junk
- script and style content unless used as an explicit extraction fallback

---

## 6. Page-Family Direction

The extraction system must evolve toward broad page-family handling rather than one-off site hacks.

Families and their inclusion rules:

| Family | Keep | Drop |
|---|---|---|
| article / blog / news | title, byline, body, inline links | related rails, share blocks, comments by default |
| docs / reference | heading hierarchy, code, API signatures, callouts | sidebar trees, TOC clones, copy buttons, version pickers |
| search / listing | result title, destination URL, snippet, ranking | query controls, filters, nav/footer chrome |
| product / commerce | title, price, rating, short description, specs | recommendation rails, carousels, store widgets |
| forum / discussion | original post, top-ranked answers, score/author metadata | long related-link rails, low-signal replies |
| utility / interstitial | compact semantic summary | pretending it is an article |

The extractor identifies page family before applying extraction rules. See [EXTRACTION.md](EXTRACTION.md) for how family detection works.

---

## 7. SPA and Structured Data Fallbacks

Fallback extraction from sources like `__NEXT_DATA__` is allowed when the visible DOM is too sparse (under 100 words of extracted text).

Rules:

- fallback content is normalized into the same public output contract
- JSON string harvesting prefers prose over metadata
- fallback output does not bypass output-mode guarantees

---

## 8. Aggressive Mode Rules

Allowed transforms:

- collapse low-value whitespace outside fenced code
- preserve paragraph separators (`\n\n`)
- truncate giant base64 blobs and long hashes to `[BASE64_TRUNCATED]`
- strip low-value URL tracking parameters
- prefer concise, machine-friendly output over presentation polish

Implementation note: use a zero-allocation, single-pass state machine. Maintain a `last_char_was_whitespace` state variable. No multi-pass regex.

Forbidden transforms:

- merging separate pages without delimiters
- corrupting fenced code blocks
- destroying all paragraph boundaries
- discarding links indiscriminately
- producing unreadable output

---

## 9. Non-Goals

The default contract is not trying to be:

- perfect visual reproduction of the source page
- full HTML fidelity
- a browser DOM export
- XML-first by default

XML-like wrapping may exist as a future optional mode (`--xml-wrap`) but is not the canonical output format.

---

## 10. Evaluation Criteria

Any change to extraction or rendering is judged on:

- **structural fidelity** — headings, paragraphs, lists, links, and code preserved correctly
- **signal retention** — important information kept
- **noise removal** — navigation, boilerplate, and repetition dropped
- **page-family fitness** — parser behaved correctly for the type of page
- **token efficiency** — especially in `aggressive` mode, and only after the Markdown path is healthy
- **stability** — snapshots and goldens remain intentionally reviewable

---

## 11. Future Extension Points

Possible future modes:

- XML-wrapped mode for downstream agent workflows
- per-platform output adapters
- additional structured research modes

Constraint: all future modes must be derived from a stable semantic intermediate representation, not from ad hoc string munging.
