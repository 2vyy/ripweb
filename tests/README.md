# Test Layout

The `tests/` directory is split by responsibility:

- top-level `*.rs`: integration test entry points grouped by subsystem
- `fixtures/`: small hand-authored fixtures used directly by tests
- `expected/curated/`: human-curated reference outputs
- `expected/generated/`: outputs generated from the shared corpus for inspection and comparison
- `snapshots/`: `insta` snapshots for targeted extraction behavior

## Curated vs generated

- curated outputs are human-maintained references used for quality evaluation
- generated outputs are machine-produced artifacts used to compare current extractor behavior across modes

These serve different jobs and should not be conflated.

## Corpus manifest

The authoritative fixture manifest lives in [src/corpus.rs](/home/ivy/Projects/ripweb/src/corpus.rs).

That manifest decides:

- whether a fixture is curated or generated-only
- whether it participates in metrics
- whether generated outputs should exist for it

Useful commands:

- `cargo run --example sync_goldens`
- `cargo run --example review_corpus`

## How This Relates To Bulk Stress Testing

The curated and generated outputs in `tests/expected/` are only one layer of evaluation.

For larger-scale parser stress testing, use the corpus workflow instead:

- review and freeze real pages into `corpus/frozen/`
- run `cargo run --example bulk_extract_report`
- inspect `corpus/reports/` for heuristic failures

That bulk evaluation layer is intentionally separate so we do not need exact goldens for every real-world page.
