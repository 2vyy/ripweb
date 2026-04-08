# Frozen Fixtures

This directory is the bridge between reviewed seed URLs and bulk parser evaluation.

Each accepted row in [freeze_review.csv](/home/ivy/Projects/ripweb/corpus/seeds/freeze_review.csv) should eventually become a local HTML snapshot at:

`corpus/frozen/<corpus_bucket>/<fixture_name>.html`

Examples:

- `corpus/frozen/health/harvard_gut_health.html`
- `corpus/frozen/programming/wikipedia_large_language_model.html`
- `corpus/frozen/product/walmart_ip_man_box_set_blu_ray.html`

## Workflow

1. Mark a row in `corpus/seeds/freeze_review.csv` as `decision=accept`
2. Fill in:
   - `fixture_name`
   - `corpus_bucket`
   - `fetch_status`
3. Run `cargo run --example prepare_freeze_targets`
4. Inspect:
   - `corpus/frozen/fetch_targets.csv`
   - `corpus/frozen/status.md`
5. Fetch snapshots with `cargo run --example fetch_freeze_targets`
6. Save or verify the actual HTML snapshot at the expected local path
7. Update `fetch_status` to `frozen`
8. Run `cargo run --example bulk_extract_report`

If a site returns a bot wall or other unusable interstitial, keep the local HTML if it helps debugging, but mark the review row with a non-`frozen` status like `failed`. The bulk report only includes rows marked `fetch_status=frozen`.

Useful flags for the fetch helper:

- `--dry-run`: show what would be fetched without hitting the network
- `--limit N`: fetch only the first `N` pending targets
- `--fixture <name>`: fetch one specific target
- `--refresh`: refetch even if the local file already exists

## Important Boundary

This repo does not need curated Markdown goldens for most of these pages.

The frozen corpus is mainly for:

- crash detection
- empty-output detection
- suspicious-structure detection
- markdown vs aggressive size comparisons
- real-world extractor regression checks

Only a small subset of especially valuable pages should later graduate into curated references under `tests/expected/curated/`.
