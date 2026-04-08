# Benchmark Corpus

This directory holds the real-world HTML pages used by `cargo bench`.

## Why this is gitignored

Corpus files are live-scraped pages, often 500 KB–5 MB each. They change over time as sites update. Committing them would bloat the repository and create stale snapshots.

## How to populate it

Fetch each file manually and save it here, matching the names listed in `benches/minify.rs`:

```
corpus/web/
  arxiv_1706.03762.html
  aws_ai_ml.html
  base64_img_bomb.txt
  docs_factory_ai.html
  docs_rs_axum.html
  github_tokio_1879.html
  hn_47326101.html
  hn_47340079.html
  legacy_img_bomb.html
  react_dev_usestate.html
  reddit_wallstreetbets_spacex.html
  stackoverflow_11227809.html
  theverge_gemini_google_maps.html
```

A seed script (`examples/seed_corpus.rs`) is planned for v0.4.

## Running the benchmarks

```sh
cargo bench                      # all three groups: extract, collapse, pipeline
cargo bench -- extract           # only the extraction group
```

Missing corpus files are skipped with a warning — the bench will still compile and run on whatever files are present.
