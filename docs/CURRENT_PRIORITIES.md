# Current Priorities

This page tracks the state of the `ripweb` engine and its roadmap.

---

## Current State

- **Structural Rework Complete**: Migrated from OutputModes into a unified `verbosity` (1-3) scale.
- **SERP Direct Mode**: Querying now returns the Search Engine Results Page directly, with modulated density based on verbosity.
- **Platform Extractions**: High-fidelity handlers for **Wikipedia (V1/V2 summary, V3 full), Reddit (JSON post + comments), HackerNews (Algolia item + comments), GitHub (Issue + comments), YouTube (MetaData + Transcripts), StackOverflow (Question + Answers), TikTok (oEmbed creator meta).**
- **Generic V3**: High-fidelity full Markdown fetching via **Jina.ai Reader** proxy.
- **Generic V2**: Capped extraction (~2000 chars) for broad research.
- **Probe Sequence**: Native `.md` and `llms.txt` lookup.

---

## Next Steps

1. **Amazon & Commerce**: Add dedicated product spec and review extraction for the `amazon.com` family.
2. **Quality Metrics**: Automate structural fidelity checks vs. frozen corpus.
3. **Clipboard Integration**: Stabilize `-c/--copy` across Linux/macOS/Windows.
4. **Enhanced YouTube**: Extract more video metadata (category, tags, publish date).

---

## Complete (v0.6)

- [x] Unify Output Density under `verbosity` parameter.
- [x] Direct SERP output for Search Engines.
- [x] GitHub Issues & Comments retrieval.
- [x] Jina Reader integration for V3 Generic.
- [x] Update documentation and source headers.

---

## Known Weak Spots

- **Crawl Efficiency**: Parallel fetching is conservative; need more aggressive pipelining.
- **HTML Heuristics**: Link-density scoring still misidentifies some documentation sidebars as boilerplate.
- **Rate Limits**: Non-API platforms (Reddit/Wikipedia) are prone to blocks; need smarter proxy rotation or MASQ updates.
