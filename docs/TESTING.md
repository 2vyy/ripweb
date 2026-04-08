# Testing

This document covers the full testing strategy for `ripweb`: all four test layers, the corpus and fixture workflow, evaluation criteria, and the golden pipeline.

---

## 1. Philosophy

Never write tests that make real HTTP calls to live websites. Live network calls produce flaky CI and skew benchmark data.

All tests run against local fixtures, frozen snapshots, or mock HTTP servers.

---

## 2. Directory Layout

```
tests/
  *.rs                     # Integration test entry points grouped by subsystem
  fixtures/                # Small hand-authored fixtures used directly by tests
  expected/
    curated/               # Human-curated reference outputs (quality evaluation)
    generated/             # Machine-produced outputs (comparison and inspection)
  snapshots/               # insta snapshots for targeted extraction behavior

corpus/
  frozen/                  # Frozen real-world page snapshots
  reports/                 # Bulk extraction reports
```

`tests/expected/curated/` and `tests/expected/generated/` serve different jobs and must not be conflated:

- **curated** outputs are human-maintained references used to judge extraction quality
- **generated** outputs are machine-produced artifacts used to compare current extractor behavior across modes

---

## 3. Fixture Manifest

The authoritative fixture manifest lives in `src/corpus.rs`.

The manifest controls:

- whether a fixture is curated or generated-only
- whether it participates in metrics
- whether generated outputs should exist for it

Always update `src/corpus.rs` when adding new fixtures.

---

## 4. Four Test Layers

### Layer 1 — Unit Tests (proptest)

Property-based testing on the minification state machine.

Invariants that must hold on any input:

- output is never longer than input
- the state machine is idempotent (running twice produces the same result as running once)
- no `3+` consecutive newlines exist in output
- whitespace inside fenced code blocks is preserved exactly

### Layer 2 — Snapshot Tests (insta)

Snapshot testing over the extraction pipeline using local fixtures.

- run against `tests/fixtures/` and `tests/expected/`
- `insta` catches any changes to extracted output during refactoring
- interactive diff review with `cargo insta review`
- snapshots live in `tests/snapshots/`

Any extraction change that modifies snapshot output requires a deliberate review pass, not a blind accept.

### Layer 3 — Network Simulation (wiremock)

Mock HTTP servers for validating network-layer behavior without touching the internet.

Test targets:

- domain politeness semaphore limits (max 3 concurrent per domain)
- global concurrency limits
- `429` / `503` / `504` retry and backoff logic
- `403` block handling and exit code
- preflight rejection of non-text MIME types
- preflight rejection of oversized responses
- cache hit/miss behavior

### Layer 4 — CPU Benchmarks (criterion)

Throughput benchmarks for the minification state machine.

- measures MB/s of the byte-parsing state machine
- validates that zero-allocation performance is maintained after changes
- reports live in `benches/`

---

## 5. Evaluation Metrics

| Metric | Formula | Goal |
|---|---|---|
| Compression Ratio | `output_tokens / input_tokens` | Maximize reduction while preserving meaning (aim for >80% on bloated sites) |
| Signal-to-Noise | `main_block_tokens / stripped_tokens` | Confirm nuke list accurately targets sidebars, footers, nav |
| SPA Detection | `spa_detected / known_spa_corpus` | Confirm `__NEXT_DATA__` heuristic triggers reliably |

Metrics are measured against the evaluation corpus using `cargo run --example metrics`.

---

## 6. Golden Workflow

Goldens are the canonical expected outputs for curated fixtures. They are generated from the shared fixture corpus and checked into `tests/expected/`.

Useful commands:

```
cargo run --example sync_goldens          # regenerate markdown and aggressive outputs
cargo run --example gen_golden            # print both modes for the shared corpus
cargo run --example metrics               # compare current outputs against curated references
cargo run --example review_corpus         # show which fixtures have expected artifacts
```

When changing extraction behavior:

1. run `cargo run --example sync_goldens` to regenerate
2. review diffs against curated references
3. update curated references intentionally if the change is an improvement

Do not blindly overwrite curated references. They are quality anchors.

---

## 7. Frozen Corpus Workflow

The frozen corpus is for larger-scale stress testing against real pages without requiring live network access.

```
corpus/frozen/    # frozen real-world HTML snapshots
corpus/reports/   # bulk extraction reports
```

Workflow:

```
cargo run --example recommend_freeze_set     # build a balanced shortlist from seed URLs
cargo run --example prepare_freeze_targets   # turn accepted seeds into a local freeze queue
cargo run --example fetch_freeze_targets     # fetch accepted targets into corpus/frozen/
cargo run --example sync_freeze_review       # update review sheet while preserving decisions
cargo run --example review_freeze_progress   # summarize accept/reject/pending counts
cargo run --example bulk_extract_report      # run bulk extraction checks across frozen corpus
cargo run --example tokenizer_audit          # compare aggressive-mode transforms against cl100k
```

The bulk evaluation layer is intentionally separate from goldens. Not every frozen page needs an exact golden; the bulk report catches heuristic failures at scale.

---

## 8. Torture Fixtures

Torture fixtures test edge cases that real-world pages may not cover: deeply nested layouts, malformed HTML, heavily obfuscated SPAs, whitespace-dependent code blocks.

```
cargo run --example generate_fixtures    # regenerate torture fixtures
cargo run --example test_torture         # quick manual smoke run
```

---

## 9. Adding a New Fixture

1. Add the raw HTML file to `tests/fixtures/` (or `corpus/frozen/` for real-world pages)
2. Register it in `src/corpus.rs` with the appropriate flags
3. Run `cargo run --example sync_goldens` to generate initial expected output
4. Review and curate the output manually if it will be a quality reference
5. Run `cargo test` to confirm snapshot baseline is captured

---

## 10. Definition of Done for Extraction Changes

An extraction change is ready when:

- all existing snapshots are reviewed and intentionally updated where needed
- curated references are updated if the change improves quality
- bulk extract report shows no regressions on frozen corpus
- metrics show no signal-to-noise degradation
- no live HTTP calls are introduced
