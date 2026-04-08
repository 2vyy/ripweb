# ripweb Developer Wiki

`ripweb` is a local, single-binary Rust CLI that fetches web content and converts it to structured Markdown optimized for LLM context windows.

This wiki is the authoritative reference for all development decisions. When two sources conflict, the page higher in the authority hierarchy wins:

```
OUTPUT_CONTRACT.md  >  EXTRACTION.md  >  everything else
```

## Program Flow

```text
  [ CLI Input ]  
        │
        ▼
  [ Router ] ──────────┐
        │              ▼
  [ Search ]    [ URL Classification ]
 (DuckDuckGo)          │
        │              ├─► Platform API (Wikipedia, SO, GitHub, etc.)
        ▼              ├─► Smart Probes (.md, llms.txt)
  [ Fetch Loop ] ◄─────┴─► Generic HTML Scraper
        │
        ▼
  [ Extraction ] ──────► [ Post-Process ]
 (DOM parsing)          (Re-ranking, Cleaning)
        │                      │
        └──────────┬───────────┘
                   │
                   ▼
           [ Output Mode ]
        (Markdown | Aggressive)
                   │
                   ▼
                [ Stdout ]
```

---

## File Tree

## Project Structure

```text
ripweb/
├── config/                  # Domain-family hints and extraction rules
├── docs/                    # Developer Wiki & architecture guides
├── benches/                 # Performance regression benchmarks
├── tests/                   # Integration tests and HTML fixtures
└── src/                     # Source Code
    ├── main.rs              # CLI entry point & stream orchestration
    ├── cli.rs               # Command-line argument definitions
    ├── router.rs            # URL classification & platform routing
    ├── run.rs               # The main dispatch & coordination loop
    ├── search/              # Structured Platform APIs
    │   ├── wikipedia.rs     # REST v1 Summary API
    │   ├── stackoverflow.rs # SE API v2.3 with answer ranking
    │   ├── arxiv.rs         # Metadata and Abstract harvesting
    │   ├── github.rs        # Issue/Comment/README extraction
    │   ├── reddit.rs        # JSON-native thread parsing
    │   ├── hackernews.rs    # Algolia API integration
    │   ├── youtube.rs       # oEmbed + timedtext transcripts
    │   ├── twitter.rs       # publish.twitter.com oEmbed
    │   └── tiktok.rs        # Public oEmbed metadata
    ├── fetch/               # The Network & Probe Layer
    │   ├── probe.rs         # .md and llms.txt auto-discovery
    │   ├── crawler.rs       # Recursive HTML scraper
    │   ├── client.rs        # MASQ browser impersonator
    │   ├── cache.rs         # XDG filesystem caching
    │   ├── politeness.rs    # Domain-keyed concurrency limits
    │   └── preflight.rs     # Content-Type & size validation
    ├── extract/             # Content Extraction Engine
    │   ├── web.rs           # Generic HTML Pipeline
    │   ├── candidate.rs     # Content-root scoring heuristics
    │   ├── boilerplate.rs   # Noise-reduction & nuke lists
    │   ├── family.rs        # Page type classification (Docs, Forum, etc.)
    │   ├── postprocess.rs   # Re-ranking & sidebar stripping
    │   ├── render.rs        # DOM to Markdown conversion
    │   └── jina.rs          # Universal high-fidelity fallback
    └── minify/              # Post-Extraction Compression
        ├── state_machine.rs # Zero-allocation token killer
        └── urls.rs          # Tracking parameter stripper
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
