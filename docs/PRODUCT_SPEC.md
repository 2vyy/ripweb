# Product Spec

This document defines what `ripweb` is, what it accepts as input, how it routes requests, and what it exposes as a CLI. For the exact shape of emitted output, see [OUTPUT_CONTRACT.md](OUTPUT_CONTRACT.md).

---

## 1. Core Philosophy

`ripweb` is a fast, local, privacy-respecting, single-binary Unix pipe for generating LLM context. Four tenets:

- **Network-Bound Reality.** CPU parsing time is negligible compared to network latency. The architecture prioritizes caching, connection pooling, and concurrent fetching with strict domain politeness.
- **Bot-Bypass by Default.** Standard HTTP clients are blocked by most sites. `ripweb` masquerades as a standard browser.
- **Markdown First.** `ripweb` is primarily a structure-preserving Markdown extractor. Token-killer mode is secondary until the Markdown path is trustworthy.
- **Streaming over Buffering.** Data flows to `stdout` in real-time as it is parsed.

---

## 2. Input Routing

Do not attempt to outsmart the user with brittle URL-guessing. Treat input as a search query by default unless it explicitly starts with `http://` or `https://`. Explicit `-u` (URL) and `-q` (query) flags override auto-detection.

### Auto-Discovery

Before parsing any URL, instantly check for `/llms.txt`, `/.well-known/llms.txt`, or `/docs/search.json`. Bypass HTML parsing entirely if found.

### Source-Specific Routing

| Source | Strategy |
|---|---|
| **GitHub** | Detect `GITHUB_TOKEN` in environment. Route to API or raw user content. Target READMEs, file trees, raw code. Avoid HTML GitHub pages. |
| **Reddit** | Append `.json` to URL. Use browser impersonation to bypass Cloudflare. Prompt for optional API token if rate-limited. |
| **StackOverflow** | Extract question, accepted answer, upvoted answers, and related questions. Strip "Hot Network Questions" and negative-score replies. |
| **HackerNews** | Use the JSON-native Algolia HN API. |
| **Generic Web** | Use DOM parser, apply nuke list, score content roots, apply page-family rules. See [EXTRACTION.md](EXTRACTION.md). |

### Charset Handling

Before parsing, read the HTTP `Content-Type` charset header or `<meta charset="...">` tag. Use `encoding_rs` to transcode Shift-JIS, ISO-8859-1, and other legacy encodings into valid UTF-8 before passing bytes to the DOM parser.

---

## 3. Output Modes

`ripweb` exposes two output modes. Full details are in [OUTPUT_CONTRACT.md](OUTPUT_CONTRACT.md).

- **`markdown` (default):** Preserve semantic structure and link targets while stripping page chrome and tracking junk.
- **`aggressive`:** Run the token killer over the Markdown output. Secondary until Markdown extraction is stable.

---

## 4. CLI Flags

| Flag | Description |
|---|---|
| `-u <url>` | Treat input explicitly as a URL |
| `-q <query>` | Treat input explicitly as a search query |
| `--limit <n>` | Limit number of results or pages |
| `--max-depth <n>` | Recursive crawl depth (default: 1) |
| `--max-pages <n>` | Global page budget across the whole run (default: 10) |
| `--stat` | Dry-run: estimate tokens and payload size before output |
| `-c` / `--copy` | Pipe output directly to system clipboard |
| `--xml-wrap` | Wrap output in context-injection XML tags |
| `--clean-cache` | Delete local cache directory |
| `-v` / `-vv` / `-vvv` | Verbosity levels mapped to `stderr` log levels |

---

## 5. Unix Pipeline Contract

- `stdout` is strictly for data
- `stderr` is for metadata, progress spinners, and logs
- ANSI colors and interactive pagers are disabled when output is piped (detected via `is-terminal`)
- `SIGPIPE` (`BrokenPipe`) exits gracefully with code `0`
- `Ctrl-C` flushes buffers and closes open Markdown blocks before shutting down
- XDG standard paths are used for config (`~/.config/ripweb/config.toml`) and cache (`~/.cache/ripweb/`)

### Multi-Page Boundary Markers

When crawling multiple pages (`--max-depth > 1`), each new page is prepended with a source delimiter in `stdout`:

```
\n\n# --- [Source: https://...] ---\n\n
```

This is part of the stable public output contract. See [OUTPUT_CONTRACT.md](OUTPUT_CONTRACT.md).

---

## 6. Exit Codes

| Code | Meaning | Trigger |
|---|---|---|
| `0` | Success | Data found, parsed, and output successfully |
| `1` | General Error | CLI argument parsing failed or invalid configuration |
| `2` | Network Failure | DNS error, connection timeout, or unreachable host |
| `3` | Blocked | 429 rate limit or 403 Cloudflare/WAF block |
| `4` | No Content | Search succeeded but yielded 0 results or empty text after parsing |

---

## 7. File Structure

```
ripweb/
├── Cargo.toml
├── config/
│   └── ripweb.toml          # Domain-family hints and extraction config
├── corpus/                  # Frozen real-world fixtures and bulk reports
├── docs/                    # This wiki
├── examples/                # Developer tooling (not part of public CLI)
├── tests/                   # Integration tests, fixtures, snapshots, expected outputs
└── src/
    ├── main.rs              # CLI entry point, stdout streaming, signal handling
    ├── cli.rs               # Argument definitions
    ├── router.rs            # Input routing (query vs URL, llms.txt auto-discovery)
    ├── config.rs            # Config loading
    ├── corpus.rs            # Fixture manifest
    ├── error.rs             # Error types
    ├── lib.rs               # Library root
    ├── search/              # Search API wrappers (return URL lists)
    │   ├── duckduckgo.rs
    │   ├── hackernews.rs
    │   ├── reddit.rs
    │   └── github.rs
    ├── fetch/               # Network and crawl layer
    │   ├── client.rs        # HTTP client setup and global concurrency
    │   ├── crawler.rs       # Crawl loop and visited-set logic
    │   ├── cache.rs         # XDG filesystem cache
    │   ├── politeness.rs    # Domain-keyed semaphores
    │   ├── preflight.rs     # Content-Type and size checks
    │   ├── normalize.rs     # URL normalization
    │   ├── llms_txt.rs      # llms.txt auto-discovery
    │   └── error.rs
    ├── extract/             # Parsers and DOM manipulation
    │   ├── web.rs           # Generic extractor: nuke list, scoring, SPA fallback
    │   └── links.rs         # Link extraction helpers
    └── minify/              # Token killer
        ├── state_machine.rs # Zero-allocation whitespace collapser
        └── urls.rs          # Tracking parameter stripper
```

---

## 8. Near-Term Roadmap

See [CURRENT_PRIORITIES.md](CURRENT_PRIORITIES.md).

### Platform Expansion Order

When adding new site-specific extractors, follow this sequence:

1. `wikipedia.org`
2. `arxiv.org`
3. `youtube.com`
4. `reddit.com` improvements
5. `github.com` improvements
6. `x.com` / `amazon.com`

This order favors high-signal, stable targets before brittle or fast-changing platforms.

### Definition of Done for New Extractors

Every new extractor ships with:

- clear routing behavior documented
- written output contract for the target
- local fixtures
- snapshot or golden coverage
- no live-internet dependence in tests
