# ripweb Developer Wiki

`ripweb` is a local, single-binary Rust CLI that fetches web content and converts it to structured Markdown optimized for LLM context windows.

This wiki is the authoritative reference for all development decisions. When two sources conflict, the page higher in the authority hierarchy wins:

```
OUTPUT_CONTRACT.md  >  EXTRACTION.md  >  everything else
```

.
├── benches
│   ├── minify.rs
│   └── README.md
├── Cargo.lock
├── Cargo.toml
├── config
│   └── ripweb.toml
├── docs
│   ├── CONFIGURATION.md
│   ├── CURRENT_PRIORITIES.md
│   ├── EXTRACTION.md
│   ├── NETWORK.md
│   ├── OUTPUT_CONTRACT.md
│   ├── PRODUCT_SPEC.md
│   ├── README.md
│   └── TESTING.md
├── src
│   ├── cli.rs
│   ├── config.rs
│   ├── error.rs
│   ├── extract
│   │   ├── boilerplate.rs
│   │   ├── candidate.rs
│   │   ├── family.rs
│   │   ├── links.rs
│   │   ├── mod.rs
│   │   ├── render.rs
│   │   └── web.rs
│   ├── fetch
│   │   ├── cache.rs
│   │   ├── client.rs
│   │   ├── crawler.rs
│   │   ├── error.rs
│   │   ├── llms_txt.rs
│   │   ├── mod.rs
│   │   ├── normalize.rs
│   │   ├── politeness.rs
│   │   └── preflight.rs
│   ├── lib.rs
│   ├── main.rs
│   ├── minify
│   │   ├── mod.rs
│   │   ├── state_machine.rs
│   │   └── urls.rs
│   ├── router.rs
│   ├── run.rs
│   └── search
│       ├── duckduckgo.rs
│       ├── github.rs
│       ├── hackernews.rs
│       ├── mod.rs
│       └── reddit.rs
└── tests
    ├── cli.rs
    ├── crawler.rs
    ├── extract_web.rs
    ├── fetch_cache.rs
    ├── fetch_client.rs
    ├── fetch_llms_txt.rs
    ├── fetch_network.rs
    ├── fixtures
    │   ├── extract
    │   ├── search
    │   └── torture
    │       ├── density
    │       ├── dom
    │       ├── encoding
    │       └── spa
    ├── output_contract.rs
    ├── README.md
    ├── router.rs
    ├── search_duckduckgo.rs
    ├── search_github.rs
    ├── search_hackernews.rs
    ├── search_reddit.rs
    └── snapshots
        ├── extract_web__snapshot_article_clean_page.snap
        ├── extract_web__snapshot_bloated_generic_page.snap
        └── extract_web__snapshot_spa_next_data_page.snap


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
