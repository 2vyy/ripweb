# Network

This document covers how `ripweb` fetches content: the HTTP client, preflight checks, politeness rules, retry logic, caching, and crawl constraints.

---

## 1. HTTP Client

`ripweb` uses `rquest` (the maintained fork of `reqwest-impersonate`) with browser TLS/JA3/JA4 fingerprint impersonation enabled. Standard HTTP clients are blocked by most targets; impersonation is non-optional.

Enabled features: `json`, `charset`, `gzip`, `brotli`.

The global client is initialized once in `src/fetch/client.rs` with:

- connection pool shared across all requests
- global concurrency limit across all in-flight requests
- TLS impersonation active by default

---

## 2. Preflight Checks

Before downloading a full response body, `ripweb` inspects response headers. Implemented in `src/fetch/preflight.rs`.

Rules:

- reject non-text MIME types (`application/pdf`, `video/mp4`, `application/zip`, etc.)
- enforce a hard `MAX_PAGE_SIZE` limit (5 MB) on `Content-Length` to prevent OOM panics from rogue binary links
- if `Content-Type` or `Content-Length` indicates rejection, fail that URL gracefully and continue with the queue

---

## 3. Domain Politeness

`ripweb` maintains per-domain concurrency limits using `tokio::sync::Semaphore` keyed by domain. Implemented in `src/fetch/politeness.rs`.

- maximum 3 concurrent requests per domain
- applies to all request types (search, crawl, direct fetch)

This prevents hammering individual hosts regardless of the global concurrency budget.

---

## 4. Timeouts

Each request enforces a hard timeout of 10 seconds. Requests that exceed this fail gracefully; the URL is skipped and the crawl continues.

---

## 5. Retry Logic

For recoverable errors, `ripweb` retries a maximum of 2 times with exponential backoff and jitter before failing the URL gracefully.

Retried status codes:

- `429` Too Many Requests
- `503` Service Unavailable
- `504` Gateway Timeout

Non-retried failures (fail immediately):

- `403` Forbidden / Cloudflare block → exit code `3`
- `4xx` other client errors → logged, URL skipped

After exhausting retries, the URL is skipped and the rest of the queue proceeds.

---

## 6. Caching

Responses are cached to disk using XDG standard paths. Implemented in `src/fetch/cache.rs`.

- cache directory: `~/.cache/ripweb/`
- cache TTL: 24 hours
- cache key: normalized URL (see URL normalization below)
- cache hit: stream directly from disk, bypassing the network entirely
- `--clean-cache` flag deletes the cache directory

Cache entries are per-URL. The cache is not invalidated by code changes; use `--clean-cache` when testing extraction changes against live content you've already fetched.

---

## 7. URL Normalization

URLs are normalized before caching and before deduplication in the visited set. Implemented in `src/fetch/normalize.rs`.

Normalization rules:

- strip URL fragments (`#anchor`)
- strip tracking parameters (UTM tags and other known noise)
- lowercase scheme and host
- remove trailing slashes where safe

---

## 8. Crawl Constraints

Multi-page crawl behavior is governed by three constraints. Implemented in `src/fetch/crawler.rs`.

| Constraint | Default | Scope |
|---|---|---|
| `--max-depth` | 1 | Maximum link-follow depth from seed URLs |
| `--max-pages` | 10 | Global page budget across the entire run, not per-seed |
| visited set | — | HashSet of normalized URLs, prevents revisiting |

Link-following rules:

- only follow links found inside content areas (`<main>`, `<article>`)
- only follow same-domain links
- do not follow links from nav, footer, sidebar, or other chrome

When `--max-pages` is exhausted, the crawl stops immediately regardless of depth or remaining queue.

---

## 9. Special Source Routing

Some sources bypass the generic fetch pipeline entirely:

| Source | Strategy |
|---|---|
| `llms.txt` / `/.well-known/llms.txt` | Fetched and parsed directly, bypasses DOM extractor |
| GitHub | Routes to GitHub API or raw.githubusercontent.com; uses `GITHUB_TOKEN` if present |
| HackerNews | Uses Algolia JSON API |
| Reddit | Appends `.json` to URL; uses browser impersonation |

See [PRODUCT_SPEC.md §2](PRODUCT_SPEC.md) for full source routing details.
