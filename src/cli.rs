//! Command Line Interface Definitions
//!
//! Defines the `Cli` struct and its associated arguments using `clap`.
//! Maps CLI flags to internal configuration and handles validation
//! (e.g., conflicting search/URL flags).

use clap::Parser;

/// Which search backend to use when the input is a query (not a URL).
#[derive(clap::ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchEngine {
    /// DuckDuckGo HTML SERP (default, no setup required)
    #[default]
    Ddg,
    /// SearXNG metasearch instance (requires --searxng-url)
    Searxng,
    /// Marginalia indie/non-SEO web search (public demo key, no config)
    Marginalia,
}

#[derive(Parser, Debug)]
#[command(
    name = "ripweb",
    about = "Fast, local, privacy-respecting CLI for generating LLM context from the web",
    long_about = None,
)]
pub struct Cli {
    /// URL or search query (required unless --clean-cache is used)
    #[arg(required_unless_present = "clean_cache")]
    pub query_or_url: Option<String>,

    /// Force URL mode — treat input as a URL even without an http:// scheme
    #[arg(short = 'u', long = "url", conflicts_with = "force_query")]
    pub force_url: bool,

    /// Force query mode — send input to search engine regardless of content
    #[arg(short = 'q', long = "query", conflicts_with = "force_url")]
    pub force_query: bool,

    /// Search engine backend to use for queries
    ///
    /// ddg (default): DuckDuckGo HTML scrape, no setup required
    /// searxng: SearXNG metasearch — requires --searxng-url
    /// marginalia: indie/non-SEO web — public demo key, no config
    #[arg(long, value_enum, default_value_t = SearchEngine::Ddg)]
    pub engine: SearchEngine,

    /// SearXNG instance base URL (required when --engine=searxng)
    ///
    /// Example public instances: https://searx.be, https://search.disroot.org
    /// Self-host with Docker for best reliability and no rate limits.
    #[arg(long, default_value = "")]
    pub searxng_url: String,

    /// Maximum crawl depth from the seed URL
    #[arg(long, default_value_t = 1)]
    pub max_depth: u32,

    /// Maximum total pages to fetch
    #[arg(long, default_value_t = 10)]
    pub max_pages: usize,

    /// Output verbosity level (1=Nucleus, 2=Signal, 3=Full Context)
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(1..=3))]
    pub verbosity: u8,

    /// Dry run: count tokens and print a size summary instead of outputting text
    #[arg(long)]
    pub stat: bool,

    /// Copy output to the system clipboard instead of writing to stdout
    #[arg(short = 'c', long)]
    pub copy: bool,

    /// Purge the XDG cache directory and exit
    #[arg(long)]
    pub clean_cache: bool,

    /// Increase log verbosity (-v = info, -vv = debug, -vvv = trace)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}
