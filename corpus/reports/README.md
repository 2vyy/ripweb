# Bulk Evaluation Reports

This directory stores generated reports for bulk parser evaluation.

These reports are not curated goldens. They are a stress/evaluation layer used to answer questions like:

- did extraction crash?
- did extraction return empty output?
- is output suspiciously flat or link-heavy?
- is aggressive mode actually reducing output size?

Primary command:

- `cargo run --example bulk_extract_report`

Outputs:

- `bulk_extract_report.csv`
- `bulk_extract_report.md`
- `tokenizer_audit.csv`
- `tokenizer_audit.md`

Current data sources:

- the shared curated corpus from `src/corpus.rs`
- any accepted rows in `corpus/seeds/freeze_review.csv` whose `fetch_status` is `frozen`

Expected workflow:

1. review seed candidates
2. accept a subset
3. freeze their HTML locally
4. point them to `fixture_name` + `corpus_bucket`
5. run the bulk extract report
6. only promote a small number of especially valuable cases into curated goldens

## Report Semantics

This report is intentionally heuristic.

It is meant to scale across many pages, so it does not try to say "this Markdown is correct." Instead it highlights suspicious rows that deserve manual inspection.

Current statuses:

- `ok`: no heuristic flags fired
- `needs_review`: extraction completed but one or more quality flags fired
- `missing_fixture`: a reviewed fixture was expected locally but no frozen HTML file was found

Current flags:

- `empty_output`: extractor returned no Markdown
- `too_short`: output is probably implausibly short for a real content page
- `link_heavy`: output may be dominated by navigation or boilerplate links
- `flat_structure`: long output with little heading/paragraph structure
- `output_longer_than_input`: suspicious expansion during extraction
- `aggressive_not_smaller`: aggressive mode failed to reduce size

These heuristics are not correctness proofs. They are triage signals for large-scale parser review.

## Tokenizer Audit

Use:

- `cargo run --example tokenizer_audit`

This report is separate from extraction-quality evaluation. Its job is to answer:

- which aggressive-mode transforms actually reduce `cl100k` token counts?
- which transforms merely look shorter to humans?
- which transforms are neutral or even make things worse?

That audit should drive aggressive-mode design more than formatting instinct alone.
