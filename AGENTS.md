# AGENTS.md — ripweb Coding Agent Guide

This file is the authoritative instruction set for any coding agent (Claude, Cursor, Copilot, etc.) working on the `ripweb` codebase. Read it entirely before making any tool calls or code modifications.

---

## 0. Document Authority Hierarchy

When two documents conflict, the higher entry wins:

```
DESIGN_DOCUMENT.md   ← product requirements, what to build and why
Implementation.md    ← how to build it, concrete Rust patterns
AGENTS.md            ← how to work in this codebase (this file)
docs/*               ← historical reference only, may be outdated
```

Do not implement anything from `docs/` that conflicts with `DESIGN_DOCUMENT.md`. Do not consult `docs/superpowers/plans/` — those planning files are superseded.

---

## 1. What ripweb Is (Agent Mental Model)

`ripweb` is a deterministic, no-AI-inside Unix pipe that fetches web content and emits clean Markdown for LLM consumption. An LLM calls ripweb like a tool; ripweb does not contain a model.

The pipeline flows in one direction:

```
CLI input
  → router.rs          (classify: URL? query? platform? generic?)
  → search/*           (if query: execute engines, fuse, score)
  → fetch/*            (HTTP, cache, preflight, politeness)
  → extract/*          (HTML/JSON → Markdown)
  → research/*         (--find, --tables, --track post-processing)
  → minify/*           (whitespace + URL minification)
  → stdout
```

Every stage is deterministic. Same input + same flags = same output within cache TTL. There are no stochastic components, no LLM calls, no randomness.

---

## 2. Error Handling — Zero Tolerance

**`unwrap()` and `expect()` are banned in all `src/` code without exception.**

- All fallible operations return `Result<T, RipwebError>` or `anyhow::Result<T>`.
- Use `?` to propagate. Use `map_err` or `context()` (from `anyhow`) to add context.
- Network errors, parse errors, and I/O errors are all graceful — log to stderr, set appropriate exit code, continue or exit cleanly.
- Exit codes are contractual. Do not use `std::process::exit()` except in `main.rs` after all cleanup is done.

```rust
// BAD
let body = response.text().unwrap();

// GOOD
let body = response.text().await.context("failed to read response body")?;
```

The `src/error.rs` `RipwebError` enum is the canonical error type for library code. `anyhow::Error` is acceptable in binary code (`src/main.rs`, `src/bin/eval.rs`) and test helpers.

---

## 3. Code Style — 2026 Rust Standards

### Formatting and linting
- All code must pass `cargo fmt` with `rustfmt.toml` settings.
- All code must pass `cargo clippy --all-targets --all-features -- -D warnings`. Zero warnings tolerated.
- Do not add `#[allow(clippy::*)]` suppressions without a comment explaining why the lint is wrong in this specific case.

### Async
- Use `tokio` for all async. Do not introduce a second async runtime.
- Mark async functions that could be sync as sync — do not add `async` speculatively.
- Concurrent operations use `tokio::join!`, `FuturesUnordered`, or `tokio::spawn`. Do not use `std::thread` for I/O work.

### Types
- Prefer `&str` over `String` in function parameters when ownership is not needed.
- Prefer `impl Trait` in function arguments over generic `<T: Trait>` where the single-dispatch is not needed.
- Newtype wrappers for domain concepts that should not be mixed (e.g. `struct NormalizedUrl(String)` rather than passing raw `String` URL strings around).
- Derive `Clone` only when actually needed. Audit before adding.

### Dependencies
- Introduce no new dependencies without a comment in `Cargo.toml` explaining why the stdlib or an existing crate is insufficient.
- Do not add crates that pull in a Tokio-incompatible async runtime, a Python/JVM runtime, or an ML inference engine.
- Preferred lightweight alternatives: `tl` for HTML parsing (already used), `regex` for patterns, `serde_json` for JSON, `chrono` for dates, `url` for URL parsing.

---

## 4. Module Responsibilities

Never add code to a module that belongs in another. If you are unsure, check the pipeline diagram in §1.

| Module | Owns | Does NOT own |
|---|---|---|
| `src/cli.rs` | Clap struct, flag definitions, validation | Business logic, I/O |
| `src/router.rs` | Input classification, platform dispatch decision | Fetching, extracting |
| `src/fetch/` | HTTP, cache, preflight, politeness, probing | Parsing content |
| `src/search/` | Engine clients, scoring, fusion, platform APIs | Extraction, rendering |
| `src/extract/` | HTML/JSON → Markdown conversion | Fetching, scoring |
| `src/research/` | Post-extraction research features (find, track, batch, wayback, wikidata) | Fetching, extraction |
| `src/minify/` | Whitespace and URL minification | Parsing, semantic processing |
| `src/run.rs` | Top-level orchestration, wiring stages together | Implementation of any stage |
| `src/bin/eval.rs` | Eval binary subcommands (cache/recall/tune/domains) | Library logic |

`src/run.rs` is the only file that imports from multiple pipeline stages. All other files import only from stages earlier in the pipeline or from utilities. No circular imports.

---

## 5. The `search/platforms/` vs `search/` Distinction

Platform APIs (arxiv, github, hackernews, reddit, stackoverflow, wikipedia, youtube, semantic_scholar, openalex) live in `src/search/platforms/`. These are **content extractors** invoked by URL pattern — they take a URL and return extracted Markdown.

Search engine clients (searxng, duckduckgo, marginalia) live directly in `src/search/`. These are **query executors** — they take a query string and return a list of `SearchResult`.

Do not put a new platform extractor directly in `src/search/`. Do not put a new search engine client inside `src/search/platforms/`.

---

## 6. Verbosity and Output Format

The output system has two orthogonal axes, both defined in `src/verbosity.rs`:

```rust
pub enum Verbosity {
    Compact,   // titles and URLs only
    Standard,  // summary + key content (~2000 chars per source) [default]
    Full,      // complete content, all comments, full transcripts
}

pub enum OutputFormat {
    Md,          // clean Markdown [default]
    Plain,       // Markdown stripped, code indentation preserved
    Structured,  // Markdown + YAML metadata header per source
}
```

These are the **only** valid values. There is no `Verbose`, `Minimal`, `Aggressive`, or numeric equivalent. Any code that accepts a string for verbosity must validate against exactly these three values and return `RipwebError::InvalidArgument` for anything else.

---

## 7. Output Contract — Non-Negotiable Rules

1. **`stdout` is data only.** Zero log lines, zero progress output, zero ANSI escape codes on stdout. Violating this breaks every downstream pipe.
2. **`stderr` is for humans and agents reading diagnostics.** Warnings, progress, error messages all go to `eprintln!()`.
3. **Source delimiters** separate multi-source output: `\n\n# --- [Source: <url>] ---\n\n`. URLs in delimiters are normalised (fragments stripped, tracking params removed).
4. **`--format structured`** prepends a YAML-like metadata block before each source's content.
5. **`--format plain`** strips Markdown syntax characters but preserves code block content and indentation exactly, separated by blank lines.
6. **Exit codes are contractual:**
   - `0` — success, content on stdout
   - `1` — configuration or argument error
   - `2` — network failure
   - `3` — blocked (403 / persistent 429)
   - `4` — no content extractable

---

## 8. Testing — Layer Rules

The test suite has five layers. Respect the boundaries strictly.

| Layer | Location | Network | LLM | Fixtures |
|---|---|---|---|---|
| L1 Unit | inline `#[cfg(test)]` in `src/` | ✗ | ✗ | ✗ |
| L2 Extraction | `tests/extraction_*.rs` | ✗ | ✗ | `tests/extraction/` |
| L3 Search | `tests/search_*.rs` | wiremock only | ✗ | `tests/search/` |
| L4 Contract | `tests/contract_*.rs` | wiremock only | ✗ | compiled binary |
| L5 Eval | `cargo run --bin eval` | ✓ (cache phase only) | ✗ | `corpus/cache/` |

**L5 never runs in `cargo test`.** It is a separate binary invoked explicitly. Do not add live network calls to any L1–L4 test.

### Snapshot tests (insta)
- Every extraction output change requires a snapshot update via `cargo insta review`. Do not auto-accept.
- Snapshot file naming convention: `<test_file>__<fixture_name>` — e.g. `extraction_apostles__github_issue`.
- Never use the verbose double-prefix scheme (`apostle_extraction__apostle_snapshot_*`). Rename on contact.
- Commit `.snap` files alongside the code change that caused them.
- CI runs `cargo insta test --check` — it fails if any snapshots are pending review. This is intentional.

### Network mocking (wiremock)
- All HTTP in L3 and L4 tests goes through a `wiremock::MockServer`.
- `MockServer::start().await` in test setup, mount fixtures from `tests/search/adapters/` or `tests/extraction/apostles/`.
- Never hardcode a URL like `https://en.wikipedia.org` in a test that makes a real call.

### Research feature tests
- Rust test files: `tests/research_find.rs`, `tests/research_wayback.rs`, `tests/research_wikidata.rs`, `tests/research_batch.rs`
- Fixture data: `tests/research/find_fixtures/`, `tests/research/wayback_fixtures/`, `tests/research/wikidata_fixtures/`
- All research tests are L2 — offline, frozen fixtures, no live calls.

---

## 9. Research Features — Implementation Rules

The `src/research/` module contains post-extraction primitives for multi-hop research loops. All features in this module are deterministic and contain no AI.

### `--find`
- Operates on **extracted Markdown**, not raw HTML.
- Splits on blank lines, table row boundaries (`|`), and list item boundaries (`-`, `*`).
- Returns all-match blocks first; falls back to partial-match ranked by count; emits exit code 4 on no match.
- Writes matched term list to `--track` session log.

### `--track`
- Appends one JSONL line per invocation to the specified file.
- Write errors are non-fatal — log to stderr, do not set exit code.
- Never reads from the session file during execution. Read is the LLM's job.

### `--batch`
- Reads URLs from stdin (one per line, `https://` only — skip others with stderr warning).
- Runs full fetch+extract pipeline concurrently via `FuturesUnordered`.
- Emits results as they complete (not in input order).
- Worst exit code across all URLs is the batch exit code.
- `--max-pages` budget applies. Truncate with a stderr warning.

### `--wikidata`
- Sends SPARQL to `https://query.wikidata.org/sparql` with `Accept: application/sparql-results+json`.
- 30-second timeout (SPARQL on large graphs is slow — 10s causes false failures).
- 400 response → exit code 1 with endpoint error message on stderr.
- Timeout → exit code 2.
- URI bindings rendered as `[QID](wikidata_url)` Markdown links.

### `--as-of`
- Two sequential calls: CDX API (`archive.org/wayback/available`) → snapshot fetch.
- Cache key is `original_url + requested_date`, not the Wayback URL.
- Prepend archived snapshot metadata header to output.
- No snapshot found → exit code 4 with clear stderr message.

### `--site`
- Appends `site:<domain>` to the query before engine dispatch.
- Post-filters results to reject any URL not matching the domain.
- If post-filter produces empty results, emit a stderr warning (do not silently fall back to unscoped search without warning).

### `--tables`
- Sets `ExtractOptions::tables_priority = true`.
- In `candidate.rs`: raises `<table>` element score from +12 to +80.
- In `render.rs`: exempts table blocks from link-saturation pruning.

---

## 10. Eval Binary Rules

`src/bin/eval.rs` implements four subcommands. It is never part of `cargo test`.

```bash
cargo run --bin eval -- cache   --input <_ref.jsonl> --out corpus/cache/
cargo run --bin eval -- recall  --cache corpus/cache/ --at-k 10
cargo run --bin eval -- tune    --cache corpus/cache/ --delta 0.1 --patience 3
cargo run --bin eval -- domains --inputs <_ref1.jsonl> <_ref2.jsonl> --out corpus/CORPUS_DOMAINS.md
```

- `cache` is the only subcommand that touches the live network. Run it once; commit `corpus/cache/`.
- `recall` and `tune` are fully offline — they operate on cached JSON files.
- `tune` outputs a TOML weight block to stdout, ready to paste into `config/ripweb.toml`.
- `ScoringWeights` must be injectable at runtime (not compile-time constants) for `tune` to work. See `Implementation.md §10.4`.
- `eval/benchmarks.jsonl` is the internal search quality benchmark — separate from the `_ref` corpus. Both are valid; neither replaces the other.

---

## 11. Adding a New Platform Extractor

Follow this checklist exactly:

1. Create `src/search/platforms/<name>.rs`
2. Add `pub mod <name>;` to `src/search/platforms/mod.rs`
3. Add a URL match arm in `src/router.rs`
4. Create a frozen fixture in `tests/extraction/apostles/<name>.[html|json]` and `<name>.meta`
5. Add a snapshot test in `tests/extraction_apostles.rs`
6. Run `cargo insta review` and commit the `.snap` file
7. Add the source to the supported sources table in `DESIGN_DOCUMENT.md §4`
8. Add minimum signal requirements to the table in `DESIGN_DOCUMENT.md §9.3`

Do not skip steps 7 and 8 — undocumented features create confusion for future agents.

---

## 12. Adding a New Search Engine

1. Create `src/search/<engine>.rs`
2. Add `pub mod <engine>;` to `src/search/mod.rs`
3. Add a frozen response fixture in `tests/search/adapters/<engine>_response.[html|json]`
4. Add a parser test in `tests/search_adapters.rs`
5. Wire the engine into `src/search/pipeline.rs` alongside existing engines
6. Document the engine in `DESIGN_DOCUMENT.md §3`

---

## 13. Pre-Commit Checklist

Before marking any task complete:

- [ ] `cargo fmt` — no formatting changes remain
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` — zero warnings
- [ ] `cargo test` — all tests pass
- [ ] `cargo insta test --check` — no pending snapshots
- [ ] `cargo deny check` — no license or advisory violations
- [ ] Did I add `unwrap()` or `expect()`? (Remove them)
- [ ] Did I add a live network call in an L1–L4 test? (Replace with wiremock)
- [ ] Did I add stdout output that isn't data? (Move to stderr)
- [ ] Did I add a new feature without updating `DESIGN_DOCUMENT.md`? (Update it)
- [ ] Did I add a new platform extractor without a fixture and snapshot? (Add them)