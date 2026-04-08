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
    torture/               # Edge case fixtures (nested layouts, malformed HTML, etc.)
  snapshots/               # insta snapshots for targeted extraction behavior
```

---

## 3. Four Test Layers

### Layer 1 — Unit Tests (proptest)

Property-based testing on the minification state machine.

Invariants that must hold on any input:

- output is never longer than input
- the state machine is idempotent (running twice produces the same result as running once)
- no `3+` consecutive newlines exist in output
- whitespace inside fenced code blocks is preserved exactly

### Layer 2 — Snapshot Tests (insta)

Snapshot testing over the extraction pipeline using local fixtures.

- run against `tests/fixtures/`
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

## 4. Evaluation Metrics

| Metric | Formula | Goal |
|---|---|---|
| Compression Ratio | `output_tokens / input_tokens` | Maximize reduction while preserving meaning (aim for >80% on bloated sites) |
| Signal-to-Noise | `main_block_tokens / stripped_tokens` | Confirm nuke list accurately targets sidebars, footers, nav |
| SPA Detection | `spa_detected / known_spa_corpus` | Confirm `__NEXT_DATA__` heuristic triggers reliably |

These metrics guide extraction improvements and validation.

---

## 5. Golden Workflow

Extraction behavior is validated through insta snapshots. When you modify extraction code, snapshots capture the expected outputs.

To review and approve snapshot changes:

```
cargo test
cargo insta review
```

This workflow ensures any extraction changes are intentional and validated, not accidental regressions.

---

## 6. Adding a New Fixture

1. Add the raw HTML file to `tests/fixtures/` (or `tests/fixtures/torture/` for edge case testing)
2. Run `cargo test` to capture the snapshot baseline
3. Review the snapshot output with `cargo insta review` and approve if correct

---

## 7. Definition of Done for Extraction Changes

An extraction change is ready when:

- all existing snapshots are reviewed and intentionally updated where needed
- unit tests pass (proptest invariants)
- integration tests pass against fixtures
- no live HTTP calls are introduced
