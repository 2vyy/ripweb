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

`ripweb` performs a **Probe Sequence** for generic documentation URLs to hunt for native Markdown before scraping HTML:
1. `<url>.md` or `<url>/index.html.md`
2. `/llms.txt` or `/.well-known/llms.txt`

### Source-Specific Routing

| Source | Strategy |
|---|---|
| **GitHub** | REST API for Issues/Comments; `raw.githubusercontent.com` for READMEs/Files. Uses `GITHUB_TOKEN` from ENV if available. |
| **Reddit** | Append `.json` to get the structured thread. |
| **StackOverflow** | SE API v2.3 to fetch title + answers, ranked by `Accepted` status and `Score`. |
| **Wikipedia** | REST v1 summary API for clean article extracts. |
| **ArXiv** | Atom query API for metadata and abstracts. |
| **HackerNews** | Algolia HN API. |
| **Generic Web** | DOM parser + nuke list + scoring. Fallback to **Jina.ai** if content is thin. |

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
├── docs/                    # Architecture and Design documentation
├── benches/                 # Performance regression benchmarks
├── tests/                   # Integration tests, fixtures, snapshots
└── src/
    ├── main.rs              # CLI entry point
    ├── cli.rs               # Argument definitions
    ├── router.rs            # Input routing and platform classification
    ├── run.rs               # Orchestration and dispatch loop
    ├── search/              # Specialized platform APIs
    │   ├── arxiv.rs, stackoverflow.rs, wikipedia.rs
    │   ├── hackernews.rs, reddit.rs, github.rs
    │   └── duckduckgo.rs
    ├── fetch/               # Network and probe layer
    │   ├── client.rs, crawler.rs, cache.rs, politeness.rs
    │   ├── preflight.rs, normalize.rs, probe.rs, llms_txt.rs
    │   └── error.rs
    ├── extract/             # HTML Parsers and re-ranking
    │   ├── web.rs, candidate.rs, postprocess.rs, render.rs
    │   ├── boilerplate.rs, family.rs, links.rs, jina.rs
    └── minify/              # Zero-allocation token killer
```

---

## 8. Near-Term Roadmap

See [CURRENT_PRIORITIES.md](CURRENT_PRIORITIES.md).

### Platform Expansion Order

When adding new site-specific extractors, follow this sequence:

1. `youtube.com` (Transcripts/Metadata)
2. `amazon.com` (Product API/Specs)
3. `x.com` / `mastodon.social`

This order favors high-signal, stable targets before brittle or fast-changing platforms.

### Definition of Done for New Extractors

Every new extractor ships with:

- clear routing behavior documented
- written output contract for the target
- local fixtures
- snapshot or golden coverage
- no live-internet dependence in tests
