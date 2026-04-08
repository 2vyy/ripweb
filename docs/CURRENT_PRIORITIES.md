# Current Priorities

This page tracks what is done, what is next, and the known weak spots to target. It is intentionally lighter than a full project tracker.

---

## Current State

- output contract is formalized in [OUTPUT_CONTRACT.md](OUTPUT_CONTRACT.md)
- generic web extraction is Markdown-first with basic candidate scoring
- curated vs generated evaluation is separated
- seed URL import, freeze review, frozen fixture workflow, and bulk parser reports are in place
- first frozen real-world batch and tokenizer audit are complete
- the main product gap is extraction quality across different page families, not infrastructure

---

## Immediate Next Steps

1. Improve generic content selection heuristics (better density and link scoring)
2. Add explicit page-family detection on top of the selected candidate
3. Add `Docs` family rendering rules (stronger sidebar stripping)
4. Add `Listing` and `Search` family detection and rendering rules
5. Add `Product` family detection and rendering rules
6. Add `Forum` / `Discussion` family detection and rendering rules
7. Revisit aggressive mode only after the Markdown path is stable

---

## v0.4 Foundation

- [x] Formalize output contract
- [x] Markdown-first extraction as the primary mode
- [x] Frozen corpus and bulk evaluation workflow
- [ ] Better generic content selection heuristics
- [ ] Page-family aware extraction for all planned families
- [ ] Less manual golden workflow (scripted generation)
- [ ] End-to-end benchmarks (fetch → extract → render on real corpora)
- [ ] Metrics for content selection quality, signal retention, structural fidelity

---

## v0.5 Platform Expansion

Add site-specific extractors in this order:

1. `wikipedia.org`
2. `arxiv.org`
3. `youtube.com`
4. `reddit.com` improvements
5. `github.com` improvements
6. `x.com` / `amazon.com`

Each new extractor ships with: routing, output contract, fixtures, snapshot/golden coverage, no live-network tests.

---

## Known Weak Spots

- generic extraction heuristics are still the biggest quality risk
- the parser does not yet reason explicitly about all page families
- golden and benchmark workflows are too manual
- platform support is thin relative to the product ambition
- evaluation currently measures "contains some words" rather than "good Markdown extraction for LLM use"

---

## Research Track

- analyze comparable tools (Readability, jusText, Trafilatura, goose) and compare output contracts
- research smarter parsing approaches before adding more heuristics
- capture findings in [EXTRACTION.md](EXTRACTION.md)
- revisit token optimization experiments only after the Markdown path is stable
