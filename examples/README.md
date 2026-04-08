# Example Tooling

The `examples/` directory is used as a lightweight toolbox for local development.

Current script roles:

- `sync_goldens.rs`: generate `markdown` and `aggressive` outputs from the shared fixture corpus
- `metrics.rs`: compare current outputs against curated references
- `gen_golden.rs`: print both output modes for the shared corpus
- `review_corpus.rs`: show which corpus fixtures have the expected curated/generated artifacts
- `recommend_freeze_set.rs`: turn normalized seed URLs into a balanced freeze shortlist
- `sync_freeze_review.rs`: preserve review decisions while refreshing the freeze review sheet
- `review_freeze_progress.rs`: summarize accept/reject/pending counts for the seed review layer
- `prepare_freeze_targets.rs`: turn accepted review rows into a concrete local freeze queue and expected file paths
- `fetch_freeze_targets.rs`: fetch accepted freeze targets into `corpus/frozen/` snapshots
- `bulk_extract_report.rs`: run non-golden bulk extraction checks across shared and frozen corpus fixtures
- `tokenizer_audit.rs`: compare candidate aggressive-mode transforms against the `cl100k` tokenizer
- `gen_baseline.rs`: print a smaller baseline subset
- `print_devto.rs`: quick single-page inspection helper
- `generate_fixtures.rs`: regenerate torture fixtures
- `test_torture.rs`: quick manual torture-fixture smoke run

These are developer utilities, not part of the public CLI contract.
