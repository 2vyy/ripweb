# Corpus Layout

This directory contains shared input data used across tests, examples, metrics, and benchmarks.

## Structure

- `web/`: representative real-world HTML and text inputs used for extraction quality and evaluation
- `torture/`: adversarial fixtures used to probe parser limits and failure cases
- `seeds/`: raw search-result URL imports, freeze recommendations, and human review metadata
- `frozen/`: locally stored HTML snapshots promoted from accepted seed review rows
- `reports/`: generated corpus-scale evaluation outputs such as bulk extract reports

## Why this exists

This corpus is intentionally separate from `benches/` so benchmark inputs can also be reused by:

- examples
- generated-output tooling
- curated golden evaluation
- integration tests

The benchmark folder should contain benchmarks. The corpus folder should contain data.

## Workflow Layers

There are three distinct ways corpus data is used:

1. Curated references
   These are a small, human-reviewed subset used for exact quality evaluation and curated expected outputs.
2. Bulk stress/eval fixtures
   These are local HTML snapshots used for large-scale parser checks without requiring exact goldens.
3. Torture fixtures
   These are targeted adversarial pages used to probe edge cases and failure modes.

The important rule is that most pages should never become curated goldens. A large corpus is most useful when it powers:

- crash detection
- empty-output detection
- structure/length heuristics
- markdown vs aggressive size comparisons
- regression spotting across categories and domains

Use `tests/expected/curated/` for the small curated layer. Use `corpus/frozen/` plus `corpus/reports/` for the larger stress/eval layer.

The seed-review workflow is:

`corpus/seeds/` -> `corpus/frozen/` -> `corpus/reports/`

That path matters because the URL collection is only raw material until pages are actually reviewed and frozen locally.
