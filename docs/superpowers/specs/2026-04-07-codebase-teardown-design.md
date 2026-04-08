---
date: 2026-04-07
topic: codebase-teardown
status: approved
---

# Codebase Teardown Design

## Goal

Eliminate the corpus/golden/examples infrastructure that has become a maintenance burden. Restructure `src/` for clarity. Leave a lean codebase focused on getting the core tool working well, with a simple fixture-based test layer and no files to manually maintain at scale.

---

## Deletions

### `examples/` — entire directory deleted

All 14 scripts removed:
- sync_goldens, metrics, gen_golden, review_corpus
- recommend_freeze_set, sync_freeze_review, review_freeze_progress
- prepare_freeze_targets, fetch_freeze_targets
- bulk_extract_report, tokenizer_audit
- gen_baseline, print_devto, generate_fixtures, test_torture

None of these are part of the public CLI. They will be re-added individually if actually needed later.

### `corpus/` — partially deleted

| Path | Action |
|---|---|
| `corpus/frozen/` | deleted |
| `corpus/seeds/` | deleted |
| `corpus/reports/` | deleted |
| `corpus/web/` | deleted |
| `corpus/README.md` | deleted |
| `corpus/torture/` | **moved** to `tests/fixtures/torture/` |
| `corpus/` directory | gone |

### `tests/expected/` — entire directory deleted

Curated and generated golden outputs removed. Insta snapshots in `tests/snapshots/` are the regression layer going forward.

### `benches/html_samples/` — deleted

HTML samples that fed the corpus pipeline. The benchmark file `benches/minify.rs` is kept.

### `src/corpus.rs` — deleted

Fixture manifest. Dead code with no corpus.

---

## `src/` Restructure

### `src/main.rs` — thin shell only

Responsibility: parse args, set up tracing, call `lib::run()`, map result to exit code, handle signals.
Target: < 100 lines. All orchestration logic moves to `lib.rs`.

### `src/lib.rs` — top-level orchestration

Contains `run()`: the main pipeline (fetch → extract → render → output).
Everything currently in `main.rs` beyond shell concerns moves here.

### `src/extract/web.rs` split

Current: 1031 lines, all concerns mixed together.

New layout:

```
src/extract/
  mod.rs           unchanged
  links.rs         unchanged
  web.rs           coordinator only, calls the four modules below (~100 lines)
  boilerplate.rs   hard nuke tags + negative-hint subtree stripping
  candidate.rs     content root scoring (word count, link density, class hints)
  family.rs        page-family detection (Docs, Article, Generic, ...)
  render.rs        Markdown rendering (headings, paragraphs, links, code, tables)
```

Dependency direction: `family.rs` is called by both `candidate.rs` (scoring adjustments) and `render.rs` (inclusion rules). `family.rs` has no dependencies on the others.

### Everything else in `src/` — untouched

`fetch/`, `minify/`, `search/`, `cli.rs`, `config.rs`, `router.rs`, `error.rs`

---

## Tests After Restructure

```
tests/
  fixtures/
    torture/        moved from corpus/torture/
    *.html          existing small hand-written snippets
  snapshots/        insta snapshots, untouched
  *.rs              integration test entry points, untouched
```

No golden files. No expected/ directory. Tests run small HTML fixtures through the extractor and use insta snapshots for regression detection. `cargo insta review` is the intentional change-acceptance workflow.

---

## What Is Not Changed

- `fetch/` module structure
- `minify/` module
- `search/` module
- `cli.rs`, `config.rs`, `router.rs`, `error.rs`
- `benches/minify.rs`
- `tests/snapshots/`
- `docs/`
- `config/ripweb.toml`
