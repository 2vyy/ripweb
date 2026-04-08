# ripweb Developer Wiki

`ripweb` is a local, single-binary Rust CLI that fetches web content and converts it to structured Markdown optimized for LLM context windows.

This wiki is the authoritative reference for all development decisions. When two sources conflict, the page higher in the authority hierarchy wins:

```
OUTPUT_CONTRACT.md  >  EXTRACTION.md  >  everything else
```

---

## Pages

| Page | Purpose | Go here when you want to... |
|---|---|---|
| [OUTPUT_CONTRACT.md](OUTPUT_CONTRACT.md) | The canonical output guarantee | Know exactly what ripweb must emit in any mode |
| [PRODUCT_SPEC.md](PRODUCT_SPEC.md) | What ripweb is and how it behaves | Understand input routing, CLI flags, exit codes, architecture |
| [EXTRACTION.md](EXTRACTION.md) | Pipeline internals and page-family logic | Work on the extractor, add a page family, or debug parser output |
| [NETWORK.md](NETWORK.md) | Fetch layer, caching, politeness, retry | Change how ripweb fetches, caches, or handles errors |
| [TESTING.md](TESTING.md) | All four test layers and evaluation workflow | Write a test, add a fixture, run benchmarks, or evaluate extraction quality |
| [CONFIGURATION.md](CONFIGURATION.md) | Project config file reference | Add a domain-family hint or understand config shape |
| [CURRENT_PRIORITIES.md](CURRENT_PRIORITIES.md) | Current state and next steps | Orient yourself on what matters right now |
