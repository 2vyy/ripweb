use clap::Parser;

#[derive(clap::ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputMode {
    #[default]
    #[value(alias = "md")]
    Markdown,
    #[value(alias = "tk", alias = "token-killer")]
    Aggressive,
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

    /// Force query mode — send input to DuckDuckGo regardless of content
    #[arg(short = 'q', long = "query", conflicts_with = "force_url")]
    pub force_query: bool,

    /// Maximum crawl depth from the seed URL
    #[arg(long, default_value_t = 1)]
    pub max_depth: u32,

    /// Maximum total pages to fetch
    #[arg(long, default_value_t = 10)]
    pub max_pages: usize,

    /// Output mode: markdown keeps structure; aggressive maximizes token compression
    #[arg(short = 'm', long = "mode", value_enum, default_value_t = OutputMode::Markdown)]
    pub mode: OutputMode,

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
