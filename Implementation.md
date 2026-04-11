# ripweb — Implementation Guide

This document covers the concrete Rust implementation of:
1. Deep research CLI flags (`--track`, `--find`, `--batch`, `--wikidata`, `--as-of`, `--site`, `--tables`)
2. Specialised source extractors (Semantic Scholar, OpenAlex, FBref, etc.)
3. Eval binary redesign (`cache` / `recall` / `tune` / `domains`)
4. Layered test directory reorganisation

All code runs synchronously or via tokio async. No ML runtime, no JVM, no external services beyond the web itself.

---

## 1. CLI Flag Additions

### 1.1 `src/cli.rs` changes

Add to the existing `clap` derive struct. All new flags are optional and composable with existing ones.

```rust
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct Cli {
    // ... existing fields ...

    /// JSONL session log. Appended after each invocation.
    /// Tracks visited URLs, found keywords, exit codes.
    #[arg(long, value_name = "FILE")]
    pub track: Option<PathBuf>,

    /// Comma-separated terms. Returns only content blocks
    /// containing ALL terms (case-insensitive). Falls back
    /// to any-match ranked by count if no block matches all.
    #[arg(long, value_name = "TERMS")]
    pub find: Option<String>,

    /// Read newline-delimited URLs from stdin and fetch
    /// all concurrently. Incompatible with -u and -q.
    #[arg(long, conflicts_with_all = ["url", "query"])]
    pub batch: bool,

    /// Execute a SPARQL query against Wikidata and emit
    /// the result as a Markdown table. Incompatible with
    /// -u, -q, and --batch.
    #[arg(
        long,
        value_name = "SPARQL",
        conflicts_with_all = ["url", "query", "batch"]
    )]
    pub wikidata: Option<String>,

    /// Fetch the Wayback Machine snapshot closest to this
    /// date (YYYY-MM-DD). Requires -u. Incompatible with -q.
    #[arg(
        long,
        value_name = "DATE",
        requires = "url",
        conflicts_with = "query"
    )]
    pub as_of: Option<String>,

    /// Scope search query to a single domain (site: operator).
    /// Requires -q. Has no effect on direct URL fetches.
    #[arg(long, value_name = "DOMAIN", requires = "query")]
    pub site: Option<String>,

    /// Promote and preserve HTML tables in output.
    /// Suppresses link-saturation pruning on table elements.
    #[arg(long)]
    pub tables: bool,
}
```

Clap validates conflicts at parse time — no runtime checks needed for incompatible flag combinations. Parse `--as-of` into a `chrono::NaiveDate` immediately after CLI parsing to fail fast on malformed dates.

---

## 2. `--track`: Session Log

### Architecture

`--track` is purely an output side-effect. It does not change how ripweb fetches or extracts — it only appends a JSONL record after the main work is done. No state is read from the file during execution; the LLM reads it between ripweb invocations.

### Data structures

```rust
// src/research/track.rs

use serde::Serialize;
use std::path::Path;
use chrono::Utc;

#[derive(Serialize)]
pub struct SessionEntry {
    /// The URL fetched, or the search query string if -q was used
    pub url: String,
    /// The raw query string if -q mode, otherwise None
    pub query: Option<String>,
    /// All terms from --find that were actually found in the output
    pub keywords_found: Vec<String>,
    /// Platform type: "generic", "github", "reddit", "wikipedia", etc.
    pub source_type: String,
    /// ISO 8601 timestamp
    pub fetched_at: String,
    /// ripweb exit code for this invocation
    pub exit_code: u8,
    /// Whether the response was served from the on-disk cache
    pub cache_hit: bool,
    /// Character count of extracted output (proxy for token cost)
    pub output_chars: usize,
}

pub fn append_entry(path: &Path, entry: &SessionEntry) -> anyhow::Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    let line = serde_json::to_string(entry)?;
    writeln!(file, "{}", line)?;
    Ok(())
}
```

### Where to call it

In `src/run.rs`, after the main fetch/extract pipeline resolves and before process exit:

```rust
if let Some(track_path) = &cli.track {
    let entry = SessionEntry {
        url: resolved_url.to_string(),
        query: cli.query.clone(),
        keywords_found: find_result.matched_terms.clone(),
        source_type: source_type.to_string(),
        fetched_at: Utc::now().to_rfc3339(),
        exit_code: final_exit_code,
        cache_hit: fetch_meta.was_cached,
        output_chars: output.len(),
    };
    // Non-fatal: log to stderr if write fails, don't exit non-zero
    if let Err(e) = track::append_entry(track_path, &entry) {
        eprintln!("warning: --track write failed: {e}");
    }
}
```

Write errors on `--track` are non-fatal — the main output has already been written. Never let a session log write block or fail the fetch.

---

## 3. `--find`: Keyword Intersection Filter

### Architecture

`--find` operates on the **extracted Markdown output**, not on raw HTML. It runs as a post-extraction pass over the text that would otherwise be written to stdout. This keeps it engine-agnostic — it works identically for generic web, Reddit, Wikipedia, etc.

### Algorithm

```rust
// src/research/find.rs

pub struct FindResult {
    pub matched_blocks: Vec<String>,
    pub match_mode: MatchMode,
    pub matched_terms: Vec<String>, // for --track
}

pub enum MatchMode {
    /// All terms found in at least one block
    AllTerms,
    /// Fallback: blocks ranked by how many terms they contain
    PartialMatch,
    /// Nothing matched anything
    NoMatch,
}

pub fn filter(text: &str, raw_terms: &str) -> FindResult {
    let terms: Vec<String> = raw_terms
        .split(',')
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();

    // Split the Markdown text into logical blocks.
    // A "block" is: a paragraph, a table row, or a list item.
    // Delimiters: blank lines (paragraphs), \n| (table rows), \n- or \n* (list items)
    let blocks = split_into_blocks(text);

    let mut all_match: Vec<String> = Vec::new();
    let mut partial: Vec<(usize, String)> = Vec::new(); // (match_count, block)

    for block in &blocks {
        let lower = block.to_lowercase();
        let count = terms.iter().filter(|t| lower.contains(t.as_str())).count();
        if count == terms.len() {
            all_match.push(block.clone());
        } else if count > 0 {
            partial.push((count, block.clone()));
        }
    }

    if !all_match.is_empty() {
        FindResult {
            matched_blocks: all_match,
            match_mode: MatchMode::AllTerms,
            matched_terms: terms,
        }
    } else if !partial.is_empty() {
        // Sort descending by match count
        partial.sort_by(|a, b| b.0.cmp(&a.0));
        FindResult {
            matched_blocks: partial.into_iter().map(|(_, b)| b).collect(),
            match_mode: MatchMode::PartialMatch,
            matched_terms: terms,
        }
    } else {
        FindResult {
            matched_blocks: vec![],
            match_mode: MatchMode::NoMatch,
            matched_terms: terms,
        }
    }
}

fn split_into_blocks(text: &str) -> Vec<String> {
    // A block ends at: a blank line, a table row boundary (line starting with |),
    // or a list item boundary (line starting with - or *).
    // Consecutive non-blank lines with no structural delimiter form one block.
    let mut blocks: Vec<String> = Vec::new();
    let mut current = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current.trim().is_empty() {
                blocks.push(current.trim().to_string());
                current = String::new();
            }
        } else if trimmed.starts_with('|') {
            // Each table row is its own block
            if !current.trim().is_empty() {
                blocks.push(current.trim().to_string());
                current = String::new();
            }
            blocks.push(line.to_string());
        } else {
            current.push('\n');
            current.push_str(line);
        }
    }
    if !current.trim().is_empty() {
        blocks.push(current.trim().to_string());
    }
    blocks
}
```

### Output formatting

When `--find` produces results, prepend a header so the LLM knows what filtering was applied:

```rust
pub fn format_find_output(result: &FindResult, terms: &str) -> String {
    match result.match_mode {
        MatchMode::AllTerms => {
            format!(
                "> filtered: {} blocks containing all of: {}\n\n{}",
                result.matched_blocks.len(),
                terms,
                result.matched_blocks.join("\n\n")
            )
        }
        MatchMode::PartialMatch => {
            format!(
                "> note: no block contained all terms [{}]; showing partial matches ranked by coverage\n\n{}",
                terms,
                result.matched_blocks.join("\n\n")
            )
        }
        MatchMode::NoMatch => String::new(), // caller sets exit code 4
    }
}
```

Exit code 4 when `MatchMode::NoMatch`. The stderr message should name the terms that failed to match so the LLM can adjust its query.

---

## 4. `--batch`: Concurrent Multi-URL Fetch

### Architecture

`--batch` reads URLs from stdin, one per line, and runs the full fetch+extract pipeline concurrently across all of them. It reuses the existing fetch stack entirely — politeness limits, preflight checks, caching, and the existing `--max-pages` budget all apply unchanged.

The key requirement from the PRD is that results emit **as they complete** (not in input order). This maximises time-to-first-result for the LLM.

### Implementation

```rust
// src/research/batch.rs

use tokio::sync::mpsc;
use futures::stream::{FuturesUnordered, StreamExt};

pub async fn run_batch(
    urls: Vec<String>,
    cli: &Cli,
    writer: &mut impl Write,
) -> anyhow::Result<u8> {
    let max_pages = cli.max_pages.unwrap_or(10);
    let urls: Vec<String> = urls.into_iter().take(max_pages).collect();

    if urls.len() < /* original count */ 0 {
        eprintln!("warning: --batch input truncated to --max-pages limit ({max_pages})");
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<BatchResult>();

    // Spawn all fetches concurrently. Domain politeness is enforced
    // inside fetch::fetch_url via the existing semaphore — no additional
    // concurrency control needed here.
    let mut futs = FuturesUnordered::new();

    for url in urls {
        let tx = tx.clone();
        let cli = cli.clone(); // Cli must derive Clone
        futs.push(tokio::spawn(async move {
            let result = fetch_and_extract(&url, &cli).await;
            let _ = tx.send(result);
        }));
    }
    drop(tx);

    // Drive all futures to completion, collecting via channel
    tokio::spawn(async move {
        while futs.next().await.is_some() {}
    });

    let mut worst_exit = 0u8;

    while let Some(result) = rx.recv().await {
        // Emit source delimiter immediately as result arrives
        writeln!(writer, "\n\n# --- [Source: {}] ---\n", result.url)?;
        match result.output {
            Ok(text) => write!(writer, "{}", text)?,
            Err(e) => {
                eprintln!("error fetching {}: {e}", result.url);
                worst_exit = worst_exit.max(result.exit_code);
            }
        }
    }

    Ok(worst_exit)
}

struct BatchResult {
    url: String,
    output: Result<String, anyhow::Error>,
    exit_code: u8,
}
```

The exit code for `--batch` is the worst exit code seen across all URLs — if any URL returned exit code 3 (blocked), the batch returns 3. If all succeeded, returns 0. This lets the caller detect partial failures.

Reading URLs from stdin:

```rust
// in src/run.rs, when cli.batch is true
use std::io::{self, BufRead};

let stdin = io::stdin();
let urls: Vec<String> = stdin
    .lock()
    .lines()
    .filter_map(|l| l.ok())
    .map(|l| l.trim().to_string())
    .filter(|l| !l.is_empty() && (l.starts_with("http://") || l.starts_with("https://")))
    .collect();

if urls.is_empty() {
    eprintln!("error: --batch requires URLs on stdin, one per line");
    std::process::exit(1);
}
```

---

## 5. `--wikidata`: SPARQL Execution

### Architecture

Pure fetch + format. ripweb sends the SPARQL query to the Wikidata public endpoint, parses the SPARQL JSON response format (standard W3C), and emits a Markdown table. No query construction, no validation beyond what the endpoint returns.

### SPARQL JSON response format

Wikidata returns:
```json
{
  "head": { "vars": ["match", "date", "referee"] },
  "results": {
    "bindings": [
      {
        "match": { "type": "uri", "value": "http://www.wikidata.org/entity/Q12345" },
        "date":  { "type": "literal", "value": "1990-06-25" },
        "referee": { "type": "literal", "value": "José Ramiz Leal" }
      }
    ]
  }
}
```

### Implementation

```rust
// src/research/wikidata.rs

use serde::Deserialize;

#[derive(Deserialize)]
struct SparqlResponse {
    head: Head,
    results: Results,
}

#[derive(Deserialize)]
struct Head {
    vars: Vec<String>,
}

#[derive(Deserialize)]
struct Results {
    bindings: Vec<HashMap<String, Binding>>,
}

#[derive(Deserialize)]
struct Binding {
    #[serde(rename = "type")]
    kind: String, // "uri", "literal", "bnode"
    value: String,
}

pub async fn execute(sparql: &str, client: &Client) -> anyhow::Result<String> {
    let url = "https://query.wikidata.org/sparql";

    let response = client
        .get(url)
        .query(&[("query", sparql), ("format", "json")])
        .header("Accept", "application/sparql-results+json")
        .header("User-Agent", "ripweb/0.1 (https://github.com/2vyy/ripweb)")
        .timeout(Duration::from_secs(30)) // SPARQL can be slow
        .send()
        .await?;

    // Wikidata returns 400 for malformed SPARQL with an error body
    if response.status() == 400 {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("SPARQL error: {body}");
    }

    let parsed: SparqlResponse = response.json().await?;
    Ok(format_as_markdown_table(&parsed))
}

fn format_as_markdown_table(resp: &SparqlResponse) -> String {
    if resp.results.bindings.is_empty() {
        return "> No results returned by Wikidata for this query.\n".to_string();
    }

    let vars = &resp.head.vars;
    let mut out = String::new();

    // Header row
    out.push('|');
    for v in vars { out.push_str(&format!(" {} |", v)); }
    out.push('\n');

    // Separator
    out.push('|');
    for _ in vars { out.push_str("---|"); }
    out.push('\n');

    // Data rows
    for binding in &resp.results.bindings {
        out.push('|');
        for var in vars {
            let cell = match binding.get(var) {
                Some(b) if b.kind == "uri" => {
                    // Render Wikidata entity URIs as Markdown links
                    if b.value.starts_with("http://www.wikidata.org/entity/") {
                        let qid = b.value.rsplit('/').next().unwrap_or(&b.value);
                        format!("[{}]({})", qid, b.value)
                    } else {
                        format!("[link]({})", b.value)
                    }
                }
                Some(b) => b.value.clone(),
                None => String::new(),
            };
            out.push_str(&format!(" {} |", cell));
        }
        out.push('\n');
    }

    out
}
```

The 30-second timeout is intentional — Wikidata SPARQL can be slow on complex queries involving large result sets. Exit code 2 on timeout, exit code 1 on 400 SPARQL error (the query itself is malformed, which is a configuration error by the caller).

---

## 6. `--as-of`: Wayback Machine Integration

### Architecture

Two sequential HTTP calls: CDX API lookup (fast, ~50ms), then snapshot fetch (slow, treated as a normal page fetch through the existing pipeline).

### CDX API

```rust
// src/research/wayback.rs

use serde::Deserialize;

#[derive(Deserialize)]
struct WaybackAvailable {
    archived_snapshots: ArchivedSnapshots,
}

#[derive(Deserialize)]
struct ArchivedSnapshots {
    closest: Option<Snapshot>,
}

#[derive(Deserialize)]
struct Snapshot {
    available: bool,
    url: String,        // The full Wayback Machine URL to fetch
    timestamp: String,  // YYYYMMDDHHMMSS
    status: String,     // HTTP status of the original archive
}

pub async fn resolve_snapshot(
    original_url: &str,
    date: &NaiveDate,
    client: &Client,
) -> anyhow::Result<ResolvedSnapshot> {
    let timestamp = date.format("%Y%m%d").to_string();
    let cdx_url = format!(
        "http://archive.org/wayback/available?url={}&timestamp={}",
        urlencoding::encode(original_url),
        timestamp
    );

    let resp: WaybackAvailable = client
        .get(&cdx_url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?
        .json()
        .await?;

    match resp.archived_snapshots.closest {
        Some(snap) if snap.available => {
            // Parse actual snapshot date from timestamp string (YYYYMMDDHHMMSS)
            let actual_date = NaiveDate::parse_from_str(&snap.timestamp[..8], "%Y%m%d")
                .unwrap_or(*date);
            Ok(ResolvedSnapshot {
                snapshot_url: snap.url,
                requested_date: *date,
                actual_date,
            })
        }
        _ => anyhow::bail!("no Wayback Machine snapshot found for {original_url} near {date}"),
    }
}

pub struct ResolvedSnapshot {
    pub snapshot_url: String,
    pub requested_date: NaiveDate,
    pub actual_date: NaiveDate,
}
```

After resolving the snapshot URL, pass it through the **existing fetch pipeline unchanged** — cache, preflight, extraction, minification all apply normally. The only special handling is prepending the metadata header to the output:

```rust
let header = format!(
    "> **Archived snapshot** — requested: {} · actual: {} · [source]({})\n\n",
    snapshot.requested_date,
    snapshot.actual_date,
    snapshot.snapshot_url,
);
let full_output = header + &extracted_text;
```

Wayback Machine URLs include a timestamp in the path (`/web/19950601000000/https://example.com`). The existing URL normaliser should strip Wayback prefixes before caching so the cache key is the original URL + date, not the full Wayback URL.

---

## 7. `--site`: Domain-Scoped Search

### Architecture

This is the simplest feature — pure query string manipulation before the search backends are called.

```rust
// in src/run.rs, when building the search query

let effective_query = match &cli.site {
    Some(domain) => format!("{} site:{}", query, domain),
    None => query.to_string(),
};
```

SearXNG, DDG Lite, and Marginalia all respect `site:` operators in the query string. No special handling per engine is needed.

One subtlety: after results return, assert that all URLs are from the specified domain and filter out any that aren't (some engines ignore `site:` for certain result types). Emit a stderr count of how many results were filtered.

```rust
if let Some(domain) = &cli.site {
    let before = results.len();
    results.retain(|r| {
        url::Url::parse(&r.url)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.ends_with(domain.as_str())))
            .unwrap_or(false)
    });
    let dropped = before - results.len();
    if dropped > 0 {
        eprintln!("note: --site filtered {dropped} results not from {domain}");
    }
    if results.is_empty() {
        eprintln!("warning: --site '{domain}' produced no results");
        // fall back to unscoped search with a warning
    }
}
```

---

## 8. `--tables`: Table-Priority Extraction

### Architecture

`--tables` is a flag passed into the extraction pipeline that changes two behaviours in `src/extract/`:

1. **Candidate scoring** (`src/extract/candidate.rs`): increase the score multiplier for `<table>` elements. Normally tables get `+12`; with `--tables` raise this to `+80` (same weight as `<article>`).

2. **Link saturation pruning** (`src/extract/render.rs`): the existing pruner drops blocks where `link_chars / total_chars > 0.4`. Tables are exempted from this check when `--tables` is active, so navigation columns don't cause data tables to be dropped.

```rust
// src/extract/mod.rs

pub struct ExtractOptions {
    pub verbosity: Verbosity,
    pub format: OutputFormat,
    pub tables_priority: bool, // set from cli.tables
    // ... existing fields ...
}
```

```rust
// src/extract/candidate.rs

fn score_element(el: &Element, opts: &ExtractOptions) -> i32 {
    let mut score = base_score(el);
    match el.tag_name() {
        "table" => {
            score += if opts.tables_priority { 80 } else { 12 };
        }
        // ... existing cases ...
    }
    score
}
```

```rust
// src/extract/render.rs

fn should_prune_block(block: &Block, opts: &ExtractOptions) -> bool {
    // Never prune tables when --tables is active
    if opts.tables_priority && block.is_table() {
        return false;
    }
    let link_ratio = block.link_chars as f32 / block.total_chars.max(1) as f32;
    link_ratio > 0.4
}
```

The table renderer itself should already emit clean pipe-delimited Markdown from the existing `render.rs`. If it currently flattens tables to prose, that needs fixing regardless of `--tables` — the flag just prioritises finding them, not rendering them.

---

## 9. Specialised Source Extractors

Each follows the same pattern: a new file in `src/search/`, a struct implementing the same trait as existing platform extractors, registered in `src/search/mod.rs` and `src/router.rs`.

### 9.1 Semantic Scholar

```rust
// src/search/semantic_scholar.rs

const API_BASE: &str = "https://api.semanticscholar.org/graph/v1";

pub async fn fetch_paper(paper_id: &str, client: &Client) -> anyhow::Result<String> {
    // paper_id can be: S2 paper ID, DOI, ArXiv ID (arXiv:XXXX.XXXXX), etc.
    let fields = "title,authors,year,abstract,venue,citationCount,externalIds";
    let url = format!("{API_BASE}/paper/{paper_id}?fields={fields}");

    let resp: PaperResponse = client.get(&url).send().await?.json().await?;
    Ok(format_paper(&resp))
}

pub async fn search_papers(query: &str, client: &Client) -> anyhow::Result<String> {
    let url = format!("{API_BASE}/paper/search");
    let resp: SearchResponse = client
        .get(&url)
        .query(&[("query", query), ("fields", "title,authors,year,venue,abstract"), ("limit", "10")])
        .send()
        .await?
        .json()
        .await?;
    Ok(format_search_results(&resp))
}
```

Route trigger: URLs matching `semanticscholar.org/paper/` go here. Query strings matching patterns like "paper about X by author at institution Y" can also be routed here as a supplementary search alongside SearXNG.

### 9.2 OpenAlex

OpenAlex is the highest-priority academic extractor because it's the only one that reliably provides full institutional affiliations — which is what makes "find paper where first author was at institution X" queries answerable.

```rust
// src/search/openalex.rs

const API_BASE: &str = "https://api.openalex.org";

pub async fn search_works(query: &str, client: &Client) -> anyhow::Result<String> {
    // OpenAlex requires a polite pool header — include a contact email
    // This is not authentication, just rate-limit pool identification
    let resp: WorksResponse = client
        .get(format!("{API_BASE}/works"))
        .query(&[
            ("search", query),
            ("per-page", "10"),
            ("select", "id,title,authorships,publication_year,primary_location,abstract_inverted_index"),
        ])
        .header("User-Agent", "ripweb/0.1 (mailto:contact@example.com)")
        .send()
        .await?
        .json()
        .await?;

    Ok(format_works(&resp))
}
```

OpenAlex's `abstract_inverted_index` is an unusual format — it maps each word to a list of positions. Reconstruct the abstract:

```rust
fn reconstruct_abstract(inverted: &HashMap<String, Vec<usize>>) -> String {
    let mut words: Vec<(usize, &str)> = inverted
        .iter()
        .flat_map(|(word, positions)| positions.iter().map(|&pos| (pos, word.as_str())))
        .collect();
    words.sort_by_key(|(pos, _)| *pos);
    words.into_iter().map(|(_, w)| w).collect::<Vec<_>>().join(" ")
}
```

### 9.3 FBref (Sports Statistics)

FBref uses standard HTML tables but has a consistent structure. No JSON API — pure HTML scrape, but the tables are clean enough that `--tables` + generic extraction handles most cases. A dedicated extractor adds:

- Filtering to only the stats table (not the nav/sidebar tables)
- Recognising FBref's specific table IDs (`#matchlogs_for`, `#referee_stats`, etc.)
- Stripping FBref's "via Sports Reference" attribution rows

```rust
// src/search/fbref.rs

pub fn extract(html: &str, url: &Url) -> anyhow::Result<String> {
    let dom = tl::parse(html, tl::ParserOptions::default())?;
    let parser = dom.parser();

    // FBref wraps data in divs with id like "div_matchlogs_for"
    // Find all divs whose id starts with "div_" and contains a table
    let tables = dom
        .query_selector("div[id^='div_'] table")
        .unwrap()
        .filter_map(|h| h.get(parser))
        .collect::<Vec<_>>();

    let mut out = String::new();
    for table in tables {
        out.push_str(&render_table(table, parser));
        out.push('\n');
    }

    if out.trim().is_empty() {
        // Fall back to generic extraction
        return crate::extract::web::extract(html, url);
    }

    Ok(out)
}
```

---

## 10. Eval Binary Redesign

### 10.1 Overall structure

```
src/bin/eval.rs
  └── main() — parse subcommand, dispatch

Subcommands:
  cache    — fetch and store raw search results for _ref corpus
  recall   — measure extraction recall against stored responses
  tune     — coordinate ascent over scorer weights to maximise MRR
  domains  — domain frequency analysis over _ref splits
```

Use `clap` with subcommands:

```rust
#[derive(Parser)]
struct EvalCli {
    #[command(subcommand)]
    cmd: EvalCommand,
}

#[derive(Subcommand)]
enum EvalCommand {
    Cache(CacheArgs),
    Recall(RecallArgs),
    Tune(TuneArgs),
    Domains(DomainsArgs),
}
```

### 10.2 `cache` subcommand

Reads a `_ref` split from a local Parquet or JSONL file (downloaded from HuggingFace separately — the eval binary does not download datasets). For each entry, issues the question text as a search query to all configured engines and stores the raw response JSON to disk.

```rust
#[derive(Args)]
struct CacheArgs {
    /// Path to the _ref split JSONL (pre-downloaded from HuggingFace)
    #[arg(long)]
    input: PathBuf,

    /// Directory to write cached search results
    #[arg(long)]
    out: PathBuf,

    /// Maximum number of questions to process (for quick test runs)
    #[arg(long)]
    limit: Option<usize>,
}
```

Cache file naming: `<out>/<sha256_of_question>.json`. SHA256 of the question text ensures stable filenames across re-runs.

Each cache file contains:
```json
{
  "question": "...",
  "source_url": "https://...",
  "answer": "...",
  "engine_results": {
    "searxng": [ { "url": "...", "title": "...", "snippet": "..." } ],
    "ddg_lite": [ ... ],
    "marginalia": [ ... ]
  },
  "cached_at": "2025-04-10T14:00:00Z"
}
```

This format means `recall` and `tune` never touch the network — they operate entirely on the cached JSON files.

### 10.3 `recall` subcommand

```rust
#[derive(Args)]
struct RecallArgs {
    /// Directory containing cache files from `cache` subcommand
    #[arg(long)]
    cache: PathBuf,

    /// k for recall@k — is the correct URL in the top k results?
    #[arg(long, default_value = "10")]
    at_k: usize,
}
```

Algorithm:
1. Load all cache files from `--cache`
2. For each file, apply current scoring weights from `config/ripweb.toml` to the `engine_results`
3. Merge via RRF into a single ranked list
4. Check if `source_url` appears in the top `--at-k` URLs (exact URL match after normalisation)
5. Accumulate: PASS (URL in top-k), FAIL (URL not in results at all), MISS (URL in results but below rank k), SKIP (exit code 3/4 during original fetch)

Output:
```
split:       seal_ref (111 questions)
recall@10:   73.0%  (81 / 111)
recall@1:    31.5%  (35 / 111)
MRR:         0.412
coverage:    89.2%  (correct URL appeared in results at any rank)
blocked:     4.5%   (exit code 3 during cache phase)
no-content:  6.3%   (exit code 4 during cache phase)
```

MRR is the primary metric for `tune`. recall@10 is the primary metric for user-facing quality reporting.

### 10.4 `tune` subcommand

```rust
#[derive(Args)]
struct TuneArgs {
    #[arg(long)]
    cache: PathBuf,

    /// Step size for coordinate ascent
    #[arg(long, default_value = "0.1")]
    delta: f64,

    /// Number of passes without improvement before stopping
    #[arg(long, default_value = "3")]
    patience: usize,
}
```

The scoring pipeline in `src/search/scoring/` uses weights that are currently hardcoded or read from config. For `tune` to work, weights must be injectable at runtime — refactor each scorer to accept a `ScoringWeights` struct:

```rust
// src/search/scoring/mod.rs

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ScoringWeights {
    pub domain_trust: f64,
    pub domain_diversity: f64,
    pub snippet_relevance: f64,
    pub url_pattern: f64,
    pub blocklist_penalty: f64,
    pub project_match: f64,
    pub rrf_k: f64, // the k constant in RRF
}

impl Default for ScoringWeights {
    fn default() -> Self {
        // These are the initial hand-tuned values — will be replaced by tune output
        Self {
            domain_trust: 1.0,
            domain_diversity: 0.5,
            snippet_relevance: 1.0,
            url_pattern: 0.8,
            blocklist_penalty: 1.0,
            project_match: 0.7,
            rrf_k: 60.0,
        }
    }
}
```

Coordinate ascent loop:

```rust
fn coordinate_ascent(
    cache_files: &[CacheFile],
    initial: ScoringWeights,
    delta: f64,
    patience: usize,
) -> ScoringWeights {
    let mut weights = initial;
    let mut best_mrr = compute_mrr(cache_files, &weights);
    let mut stall_count = 0;

    // The fields to tune, as (getter, setter) pairs
    // In practice, use an array of indices into a Vec<f64> representation
    let n_weights = 7; // number of tunable fields

    loop {
        let mut improved = false;

        for i in 0..n_weights {
            // Try +delta
            let mut candidate = weights.clone();
            candidate.set(i, (candidate.get(i) + delta).max(0.0));
            let mrr_plus = compute_mrr(cache_files, &candidate);

            // Try -delta
            let mut candidate_minus = weights.clone();
            candidate_minus.set(i, (candidate_minus.get(i) - delta).max(0.0));
            let mrr_minus = compute_mrr(cache_files, &candidate_minus);

            if mrr_plus > best_mrr {
                weights = candidate;
                best_mrr = mrr_plus;
                improved = true;
            } else if mrr_minus > best_mrr {
                weights = candidate_minus;
                best_mrr = mrr_minus;
                improved = true;
            }
        }

        if !improved {
            stall_count += 1;
            if stall_count >= patience {
                break;
            }
        } else {
            stall_count = 0;
        }
    }

    eprintln!("converged: MRR = {:.4}", best_mrr);
    weights
}
```

Output to stdout as TOML, ready to paste:
```toml
# Tuned weights — MRR: 0.487 on seal_ref (111 questions)
# Generated: 2025-04-10
[search.scoring]
domain_trust = 1.2
domain_diversity = 0.4
snippet_relevance = 1.1
url_pattern = 0.9
blocklist_penalty = 1.3
project_match = 0.6
rrf_k = 60.0
```

### 10.5 `domains` subcommand

```rust
#[derive(Args)]
struct DomainsArgs {
    /// One or more _ref split JSONL files
    #[arg(long, num_args = 1..)]
    inputs: Vec<PathBuf>,

    /// Number of top domains to report
    #[arg(long, default_value = "50")]
    top: usize,

    /// Write output to this file (default: stdout)
    #[arg(long)]
    out: Option<PathBuf>,
}
```

Algorithm: parse all `source_url` fields, extract `host` via `url::Url::parse`, count occurrences, sort descending, emit as Markdown table. Also compute per-domain fail rate if cache files are available.

Output format (written to `corpus/CORPUS_DOMAINS.md`):
```markdown
# Corpus Domain Analysis
Generated: 2025-04-10 | Sources: seal_ref (111) + webwalkerqa_ref (680)

| Rank | Domain | Questions | Fail rate |
|---|---|---|---|
| 1 | en.wikipedia.org | 89 | 2.2% |
| 2 | rsssf.com | 34 | 18.8% |
...
```

---

## 11. Test Directory Reorganisation

### 11.1 Target layout

```
tests/
  extraction/              ← Layer 2: HTML/JSON-in, Markdown-out
    apostles/              ← platform-specific fixtures
      github_issue.html
      github_issue.meta
      reddit_thread.json
      reddit_thread.meta
      wikipedia_rust.json
      wikipedia_rust.meta
      ... (one pair per platform)
    generic/               ← generic extractor fixtures
      article_clean.html
      article_clean.meta
      bloated_generic.html
      docs_sidebar.html
      forum_thread.html
      listing_results.html
      product_detail.html
      spa_next_data.html
    torture/               ← adversarial robustness
      density/
      dom/
      encoding/
      spa/
  search/                  ← Layer 3: mocked network
    adapters/              ← one fixture per engine parser
      searxng_response.json
      ddg_lite_response.html
      marginalia_response.json
    scoring/               ← unit tests for each scorer
    fusion/                ← RRF merge tests
    pipeline/              ← full scoring+fusion integration
    eval/                  ← JSONL quality benchmarks (existing)
      regression.jsonl
      techdocs_bench.jsonl
      regression_fanout.jsonl
      techdocs_fanout.jsonl
  contract/                ← Layer 4: binary-level CLI
  snapshots/               ← all insta golden outputs (flat)
  common/                  ← shared helpers (keep as-is)
```

### 11.2 Migration steps

These are file moves, not rewrites. Do them in this order to avoid breaking CI mid-migration:

**Step 1 — create new directories**
```bash
mkdir -p tests/extraction/apostles tests/extraction/generic tests/extraction/torture
mkdir -p tests/search/adapters tests/search/scoring tests/search/fusion
mkdir -p tests/search/pipeline tests/search/eval tests/contract
```

**Step 2 — move fixtures** (the HTML/JSON/meta files, not the Rust test files)
```bash
mv tests/fixtures/apostles/* tests/extraction/apostles/
mv tests/fixtures/extract/*  tests/extraction/generic/
mv tests/fixtures/torture/*  tests/extraction/torture/
mv tests/fixtures/search/eval/* tests/search/eval/
mv tests/fixtures/search/ddg_results.html tests/search/adapters/
```

**Step 3 — update fixture paths in Rust test files**

Each test file that calls something like:
```rust
let html = include_str!("fixtures/apostles/github_issue.html");
```
becomes:
```rust
let html = include_str!("extraction/apostles/github_issue.html");
```

This is a mechanical find-and-replace. Run `cargo test` after each test file update to catch any missed paths.

**Step 4 — move and rename Rust test files**
```bash
# Layer 2
# tests/apostle_extraction.rs → stays as tests/extraction.rs (or keep in place, just rename)

# Layer 3
# tests/search.rs, tests/search_fusion.rs, tests/search_pipeline.rs,
# tests/search_scoring.rs, tests/search_eval.rs
# → consolidate into tests/search.rs with submodules, or keep separate files
#   and move to match the new layout mentally (Rust doesn't enforce directory
#   structure for test files — they just need to be in tests/)

# Layer 4
# tests/cli.rs, tests/cli_e2e.rs, tests/output_contract.rs
# → rename to tests/contract_cli.rs, tests/contract_e2e.rs, tests/contract_output.rs
```

**Step 5 — rename snapshots**

Snapshot names are derived from test function names via insta. To rename a snapshot, rename the test function and run `cargo insta review` to approve the new snapshot. Do this incrementally — one platform at a time as you touch the relevant test code.

Current → target:
```
apostle_extraction__apostle_snapshot_github_issue     → extraction__github_issue
apostle_extraction__apostle_snapshot_reddit_thread_v2 → extraction__reddit_thread
extract_web__snapshot_article_clean_page              → extraction__article_clean
extract_web__snapshot_docs_sidebar_page               → extraction__docs_sidebar
```

**Step 6 — delete removed files**
```bash
rm src/search/twitter.rs
rm src/search/tiktok.rs
rm src/search/ddg_instant.rs
# Remove references from src/search/mod.rs and src/router.rs
```

**Step 7 — delete now-empty directories**
```bash
rm -rf tests/fixtures/
```

### 11.3 Adding a new DDG Lite fixture to Layer 3

The existing `tests/fixtures/search/ddg_results.html` targets the HTML endpoint. Replace with a DDG Lite fixture:

```bash
# Capture a real DDG Lite response once, freeze it as the fixture
curl 'https://lite.duckduckgo.com/lite/?q=tokio+async+rust' \
  -H 'User-Agent: Mozilla/5.0...' \
  > tests/search/adapters/ddg_lite_response.html
```

Test:
```rust
#[test]
fn ddg_lite_parser_extracts_results() {
    let html = include_str!("search/adapters/ddg_lite_response.html");
    let results = crate::search::duckduckgo::parse_lite(html).unwrap();
    assert!(results.len() >= 3);
    assert!(results.iter().all(|r| r.url.starts_with("https://")));
    // Assert no /l/?uddg= redirect URLs leaked through
    assert!(results.iter().all(|r| !r.url.contains("duckduckgo.com/l/")));
}
```

### 11.4 Snapshot stability guarantee

Never run `cargo insta accept` in CI. The CI workflow should run:
```bash
cargo test
cargo insta test --check  # fails if any snapshots are pending review
```

`--check` mode fails the build if snapshots are pending rather than accepting them silently. This ensures every extraction change is a deliberate human decision.