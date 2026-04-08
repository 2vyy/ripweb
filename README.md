# ripweb

`ripweb` is a high-efficiency, privacy-respecting Unix pipeline for harvesting web content into structure-preserving Markdown for LLM context.

It prioritizes native Markdown and keyless structured APIs (Wikipedia, StackOverflow, ArXiv) over raw HTML scraping to ensure the highest possible signal-to-noise ratio.

## Core Flow

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

## Project Structure

```text
ripweb/
├── config/             # Domain-family hints and extraction rules
├── src/                # Core implementation
│   ├── router.rs       # Input classification & routing
│   ├── search/         # Structured Platform APIs (Wiki, SO, etc.)
│   ├── fetch/          # Network, Probes, and Crawler
│   ├── extract/        # Scrutiny-based HTML extraction
│   └── minify/         # Token-killer & URL stripping
├── tests/              # Integration tests & fixtures
└── docs/               # Detailed Developer Wiki
```

### Core Components

- **Router**: Classifies inputs into queries or specific platforms.
- **Search Modules**: Native API wrappers for high-signal targets (Wikipedia, ArXiv, etc.).
- **Probe Layer**: Non-invasive discovery of `.md` and `llms.txt` files.
- **Extraction Engine**: Heuristic scoring and rendering of generic HTML into Markdown.
- **Post-Processor**: Re-ranks forum results and cleans document sidebars.

---

## Key Features

- **Keyless Platform APIs**: High-fidelity extraction for Wikipedia, StackOverflow, ArXiv, Reddit, HackerNews, and GitHub without requiring API keys.
- **Smart Probing**: Automatically detects `.md` suffixes and `llms.txt` indexes to avoid expensive HTML scraping.
- **Markdown-First**: Heuristic-based generic extraction that preserves semantic structure (tables, code blocks, lists).
- **Universal Fallback**: Integrated `r.jina.ai` proxy for JS-heavy or complex pages.
- **Aggressive Minification**: Optional "Token Killer" mode and URL tracking parameter stripping.
- **Privacy & Speed**: MASQ browser impersonation, local caching, and strict domain politeness.

## Documentation

For deep dives into the architecture and development, see the [Developer Wiki](docs/README.md):

- [Output Contract](docs/OUTPUT_CONTRACT.md) — The canonical output guarantees.
- [Extraction Pipeline](docs/EXTRACTION.md) — How the DOM parser and candidate scorer work.
- [Product Spec](docs/PRODUCT_SPEC.md) — CLI flags, routing logic, and architecture.
- [Testing & Evaluation](docs/TESTING.md) — Frozen corpus and bulk extraction reporting.
- [Current Priorities](docs/CURRENT_PRIORITIES.md) — Roadmap and next steps.
