# Test Layout

The `tests/` directory is organized by test layer and fixture type.

## Integration test binaries (top-level `.rs`)

- `unit_*`: Layer 1 unit tests (minify, router, normalize)
- `extraction_*`: Layer 2 extraction tests (apostles, generic, torture, metrics)
- `search_*`: Layer 3 search tests (adapters, scoring, fusion, pipeline, eval)
- `contract_*`: Layer 4 CLI/output contract tests
- `fetch.rs` / `crawler.rs`: network-layer behavior tests
- `research_*.rs`: research feature tests

## Fixture-only directories

- `extraction/`: HTML/JSON fixtures for extraction layers
- `search/`: frozen adapter responses and eval JSONL datasets
- `research/`: frozen fixtures for find/wayback/wikidata tests

These fixture directories intentionally contain test data only; Rust test entry points stay at `tests/` root.

## Shared helpers and snapshots

- `common/`: reusable helper modules (no test binaries)
- `snapshots/`: all `insta` golden snapshots (flat, auto-managed)
