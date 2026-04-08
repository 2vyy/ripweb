# Current Priorities

This page tracks the state of the `ripweb` engine and its roadmap.

---

## Current State

- **Structural Rework Complete**: Migrated from OutputModes into a unified `verbosity` (1-3) scale.
- **SERP Direct Mode**: Querying now returns the Search Engine Results Page directly, with modulated density based on verbosity.
- **Platform Extractions**: High-fidelity handlers for **Wikipedia, Reddit, HackerNews, GitHub, YouTube, StackOverflow, TikTok.**
- **Panic Eradication**: System-wide removal of `unwrap()` and `expect()`. Robust error handling via `thiserror` and `anyhow`.
- **AST Semantic Pruning**: Block-level link-saturation logic using `tl` AST analysis (replaces string heuristics).
- **Privacy by Default**: Cloud proxies (Jina) restricted behind `--allow-cloud` flag.
- **Probe Sequence**: Native `.md` and `llms.txt` lookup.

---

## Next Steps

1. **Amazon & Commerce**: Add dedicated product spec and review extraction for the `amazon.com` family.
2. **Enhanced YouTube**: Extract more video metadata (category, tags, publish date).
3. **Smart Rate Limiting**: Intelligent MASQ updates based on WAF response headers.

---

## Complete (v0.6)

- [x] Unify Output Density under `verbosity` parameter.
- [x] Direct SERP output for Search Engines.
- [x] GitHub Issues & Comments retrieval.
- [x] Jina Reader integration for V3 Generic.
- [x] Panic Eradication: Remove all architectural `unwrap()` / `expect()`.
- [x] AST Semantic Pruning: Mathematical link-saturation heuristics.
- [x] Privacy by Default: `--allow-cloud` opt-in for Jina.
- [x] Update documentation and source headers.

---

## Known Weak Spots

- **Crawl Efficiency**: Parallel fetching is conservative; need more aggressive pipelining.
- **HTML Heuristics**: Link-density scoring still misidentifies some documentation sidebars as boilerplate.
- **Rate Limits**: Non-API platforms (Reddit/Wikipedia) are prone to blocks; need smarter proxy rotation or MASQ updates.
