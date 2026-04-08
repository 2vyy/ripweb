# Product Spec

This document defines what `ripweb` is, what it accepts as input, how it routes requests, and what it exposes as a CLI. For the exact shape of emitted output, see [OUTPUT_CONTRACT.md](OUTPUT_CONTRACT.md).

---

## 1. Core Philosophy

`ripweb` is a fast, local, privacy-respecting, single-binary Unix pipe for generating LLM context. Three tenets:

- **Network-Bound Reality.** CPU parsing time is negligible; performance focuses on concurrency, caching, and connection pooling.
- **Bot-Bypass.** MASQ browser impersonation avoids blocks.
- **Verbosity Modulated.** Information is scaled from "Nucleus" (V1) to "Full Context" (V3) to manage token windows.

---

## 2. Input Routing

Input is classified as a Search Query or a URL. 

### Source-Specific Routing

| Source | Strategy |
|---|---|
| **GitHub** | REST API for Issues/Comments; `raw.githubusercontent.com` for READMEs. |
| **Reddit** | Append `.json` for structured thread JSON. |
| **StackOverflow** | SE API v2.3 to fetch title + answers, ranked by score. |
| **Wikipedia** | REST v1 summary (V1/V2); Full Page (V3). |
| **YouTube** | oEmbed metadata + full transcripts (V3). |
| **TikTok** | Public oEmbed creator metadata. |
| **Generic Web** | DOM parser (V2); **Jina Reader** rehydration (V3). |

---

## 3. Output Verbosity (1-3)

`ripweb` uses a unified `verbosity` scale instead of fixed output modes. See [OUTPUT_CONTRACT.md](OUTPUT_CONTRACT.md) for full density requirements.

1. **V1 (Nucleus)** — Titles and links only. No prose.
2. **V2 (Signal)** — Headlines, summaries, and snippets (capped prose).
3. **V3 (Full Context)** — Exhaustive details, all comments, and full transcripts.

---

## 4. CLI Flags

| Flag | Description |
|---|---|
| `-u <url>` | Treat input explicitly as a URL |
| `-q <query>` | Treat input explicitly as a search query |
| `--verbosity <n>` | Output density (1-3, default: 2) |
| `--limit <n>` | Number of search results (default: 10) |
| `--max-depth <n>` | Recursive crawl depth (default: 1) |
| `--max-pages <n>` | Global page budget (default: 10) |
| `--stat` | Dry-run: count tokens and payload size |
| `-c` / `--copy` | Copy output to clipboard |
| `--clean-cache` | Delete local cache directory |
| `-v` / `-vv` / `-vvv` | Logging verbosity (mapped to `stderr`) |

---

## 5. Unix Pipeline Contract

- `stdout` is for data.
- `stderr` is for progress, stats, and logs.
- `SIGPIPE` exists gracefully.
- Source delimiters `\n\n# --- [Source: URL] ---\n\n` separate multiple data sources.

---

## 6. Exit Codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | Configuration or CLI Argument Error |
| `2` | Network Failure |
| `3` | Blocked (429/403) |
| `4` | No Content |
