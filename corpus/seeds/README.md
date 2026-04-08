# Seed URL Imports

This directory stores raw and normalized search-result URL seeds collected from external spreadsheets.

Files:
- `search_results_urls.csv`: raw spreadsheet export
- `search_results_urls.normalized.csv`: parsed URL seeds with category metadata and preserved raw cell values
- `freeze_candidates.csv`: balanced shortlist of fetch/freeze candidates derived from the normalized seed set
- `freeze_review.csv`: human-reviewed decisions layered on top of the generated shortlist

Import summary:
- raw rows: 182
- normalized entries: 913
- invalid / non-URL cells skipped: 2

Per-category counts:
- `programming`: 157
- `news`: 152
- `shopping`: 169
- `science`: 154
- `cooking`: 61
- `finance`: 66
- `health`: 37
- `sports`: 60
- `travel`: 28
- `legal`: 29

Skip reasons:
- `no_url_match`: 2

Selection workflow:
1. raw spreadsheet export lands here
2. normalized CSV preserves category, source row, parsed URL, trailing note, and raw cell
3. `cargo run --example recommend_freeze_set` writes `freeze_candidates.csv`
4. `cargo run --example sync_freeze_review` creates or updates `freeze_review.csv` while preserving prior decisions
5. edit `freeze_review.csv` by filling in `decision`, `decision_reason`, `fixture_name`, `corpus_bucket`, and `fetch_status`
6. `cargo run --example review_freeze_progress` summarizes review status
7. `cargo run --example prepare_freeze_targets` turns accepted review rows into a concrete local freeze queue
8. `cargo run --example fetch_freeze_targets` fetches the HTML snapshots into those local paths
9. once accepted pages are frozen locally under `corpus/frozen/<corpus_bucket>/<fixture_name>.html`, `cargo run --example bulk_extract_report` can include them in bulk parser evaluation

Decision values:
- `pending`
- `accept`
- `reject`

The generated candidate file is a recommendation layer, not a final corpus fixture list.

Field notes:
- `fixture_name`: stable local fixture slug, for example `harvard_gut_health`
- `corpus_bucket`: target folder under `corpus/frozen/`, for example `health`
- `fetch_status`: suggested values are `pending`, `frozen`, `failed`, `skipped`

## What This Data Is For

The reviewed seed set is not meant to become a giant library of curated goldens.

Instead, it supports a larger bulk stress/evaluation layer:

- freeze a diverse set of real search-result pages locally
- run extraction and minification over all of them
- flag suspicious outputs without requiring exact expected text
- only promote a small number of especially valuable fixtures into curated references later

That means most accepted rows should stop at `corpus/frozen/` and be evaluated by reports and heuristics rather than hand-maintained Markdown outputs.

## Bulk Stress-Test Workflow

1. accept rows in `freeze_review.csv`
2. assign `fixture_name` and `corpus_bucket`
3. freeze each accepted page to `corpus/frozen/<corpus_bucket>/<fixture_name>.html`
4. run `cargo run --example prepare_freeze_targets` to confirm the expected file paths
5. run `cargo run --example fetch_freeze_targets`
6. mark `fetch_status=frozen`
7. run `cargo run --example bulk_extract_report`
8. inspect `corpus/reports/bulk_extract_report.md` and `corpus/reports/bulk_extract_report.csv`

The bulk report is designed to answer questions like:

- did extraction return nothing?
- did we likely choose the wrong content container?
- is the output mostly links or boilerplate?
- is aggressive mode actually shorter than Markdown mode?

This is the scalable way to stress-test the parser across many real pages without creating golden standards for all of them.
