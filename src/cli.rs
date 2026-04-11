//! Command Line Interface Definitions
//!
//! Defines the `Cli` struct and its associated arguments using `clap`.
//! Maps CLI flags to internal configuration and handles validation
//! (e.g., conflicting search/URL flags).

use clap::Parser;
use std::path::PathBuf;

use crate::verbosity::{OutputFormat, Verbosity};

#[derive(Parser, Debug, Clone)]
#[command(
    name = "ripweb",
    about = "Fast, local, privacy-respecting CLI for generating LLM context from the web",
    long_about = None,
)]
pub struct Cli {
    /// URL or search query (required unless --clean-cache is used)
    #[arg(required_unless_present_any = ["clean_cache", "batch", "wikidata"])]
    pub query_or_url: Option<String>,

    /// Force URL mode — treat input as a URL even without an http:// scheme
    #[arg(short = 'u', long = "url", conflicts_with = "force_query")]
    pub force_url: bool,

    /// Force query mode — send input to search engine regardless of content
    #[arg(short = 'q', long = "query", conflicts_with = "force_url")]
    pub force_query: bool,

    /// Output density (compact | standard | full)
    #[arg(long, value_enum, default_value_t = Verbosity::Standard)]
    pub verbosity: Verbosity,

    /// Output format (md | plain | structured)
    #[arg(long, value_enum, default_value_t = OutputFormat::Md)]
    pub format: OutputFormat,

    /// JSONL session log path. Appends one structured record per invocation.
    #[arg(long, value_name = "FILE")]
    pub track: Option<PathBuf>,

    /// Comma-separated search terms to filter output blocks.
    #[arg(long, value_name = "TERMS")]
    pub find: Option<String>,

    /// Read newline-delimited URLs from stdin and fetch concurrently.
    #[arg(long, conflicts_with_all = ["force_url", "force_query", "wikidata"])]
    pub batch: bool,

    /// Execute a SPARQL query against Wikidata.
    #[arg(
        long,
        value_name = "SPARQL",
        conflicts_with_all = ["force_url", "force_query", "batch"]
    )]
    pub wikidata: Option<String>,

    /// Fetch nearest Wayback snapshot to this date (YYYY-MM-DD).
    #[arg(long, value_name = "DATE", conflicts_with = "force_query")]
    pub as_of: Option<String>,

    /// Scope search query to a single domain via site: operator.
    #[arg(long, value_name = "DOMAIN", requires = "force_query")]
    pub site: Option<String>,

    /// Prioritize and preserve HTML tables in extraction.
    #[arg(long)]
    pub tables: bool,

    /// SearXNG instance base URL
    #[arg(long, default_value = "http://localhost:8080")]
    pub searxng_url: String,

    /// Maximum crawl depth from the seed URL
    #[arg(long, default_value_t = 1)]
    pub max_depth: u32,

    /// Maximum total pages to fetch
    #[arg(long, default_value_t = 10)]
    pub max_pages: usize,

    /// Allow pushing data through a cloud extraction parser (like Jina)
    #[arg(long, default_value_t = false)]
    pub allow_cloud: bool,

    /// Dry run: count tokens and print a size summary instead of outputting text
    #[arg(long)]
    pub stat: bool,

    /// Copy output to the system clipboard instead of writing to stdout
    #[arg(short = 'c', long)]
    pub copy: bool,

    /// Cache Time-To-Live in seconds (default: 604800 / 7 days)
    #[arg(long, value_name = "SECONDS")]
    pub cache_ttl: Option<u64>,

    /// Purge the XDG cache directory and exit
    #[arg(long)]
    pub clean_cache: bool,

    /// Increase log verbosity (-v = info, -vv = debug, -vvv = trace)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}
