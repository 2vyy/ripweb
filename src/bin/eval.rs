//! Offline retrieval evaluation tooling for ripweb.
//!
//! Subcommands:
//! - `cache`: query engines and persist per-question result caches
//! - `recall`: evaluate recall/MRR over cached files (answer-string matching)
//! - `url-recall`: check whether the correct source URL appears in raw engine results
//! - `tune`: coordinate-ascent tuning of scorer weights
//! - `domains`: report corpus source domain frequencies

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use ripweb::{
    config::{BlocklistConfig, TrustConfig, get_config},
    search::{
        SearchResult, duckduckgo, eval_types::SearchResultRecord, fusion::rrf_fuse_with_k,
        marginalia, pipeline::score_results_with_weights, scoring::ScoringWeights, searxng,
    },
};

const DEFAULT_SEARXNG_URL: &str = "http://localhost:8080";
const ENGINE_LIMIT: usize = 20;

#[derive(Parser, Debug)]
#[command(
    name = "ripweb-eval",
    about = "Corpus-based search evaluation utilities"
)]
struct EvalCli {
    #[command(subcommand)]
    cmd: EvalCommand,
}

#[derive(Subcommand, Debug)]
enum EvalCommand {
    Cache(CacheArgs),
    Recall(RecallArgs),
    UrlRecall(UrlRecallArgs),
    Tune(TuneArgs),
    Domains(DomainsArgs),
}

#[derive(Args, Debug)]
struct CacheArgs {
    /// Path to the _ref split JSONL.
    #[arg(long)]
    input: PathBuf,

    /// Directory to write cache files.
    #[arg(long)]
    out: PathBuf,

    /// Maximum questions to cache (optional).
    #[arg(long)]
    pub limit: Option<usize>,

    /// Cache TTL in seconds. Eval caches are permanent by default.
    #[arg(long, default_value = "18446744073709551615")]
    pub ttl: u64,
}

#[derive(Args, Debug)]
struct RecallArgs {
    /// Directory containing cached query JSON files.
    #[arg(long)]
    cache: PathBuf,

    /// Recall@k threshold.
    #[arg(long, default_value = "10")]
    at_k: usize,
}

#[derive(Args, Debug)]
struct TuneArgs {
    /// Directory containing cached query JSON files.
    #[arg(long)]
    cache: PathBuf,

    /// Coordinate-ascent step size.
    #[arg(long, default_value = "0.1")]
    delta: f64,

    /// Number of non-improving passes before stopping.
    #[arg(long, default_value = "3")]
    patience: usize,
}

#[derive(Args, Debug)]
struct UrlRecallArgs {
    /// Directory containing cached query JSON files (from the `cache` subcommand).
    #[arg(long)]
    cache: PathBuf,

    /// Report url-recall@k for this k (also reports @1, @5, @∞ always).
    #[arg(long, default_value = "10")]
    at_k: usize,
}

#[derive(Args, Debug)]
struct DomainsArgs {
    /// One or more _ref split JSONL files.
    #[arg(long, num_args = 1..)]
    inputs: Vec<PathBuf>,

    /// Number of top domains to report.
    #[arg(long, default_value = "50")]
    top: usize,

    /// Optional output file (defaults to stdout).
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct RefQuestion {
    question: String,
    source_url: String,
    answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheFile {
    question: String,
    source_url: String,
    answer: String,
    engine_results: BTreeMap<String, Vec<SearchResultRecord>>,
    cached_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = EvalCli::parse();
    match cli.cmd {
        EvalCommand::Cache(args) => run_cache(args).await,
        EvalCommand::Recall(args) => run_recall(args),
        EvalCommand::UrlRecall(args) => run_url_recall(args),
        EvalCommand::Tune(args) => run_tune(args),
        EvalCommand::Domains(args) => run_domains(args),
    }
}

async fn run_cache(args: CacheArgs) -> Result<()> {
    let mut rows = load_ref_split(&args.input)?;
    if let Some(limit) = args.limit {
        rows.truncate(limit);
    }
    if rows.is_empty() {
        anyhow::bail!("input split contains no rows");
    }

    fs::create_dir_all(&args.out).with_context(|| {
        format!(
            "failed to create cache output directory {}",
            args.out.display()
        )
    })?;

    let searxng_url =
        std::env::var("RIPWEB_SEARXNG_URL").unwrap_or_else(|_| DEFAULT_SEARXNG_URL.to_owned());
    let client = rquest::Client::builder()
        .build()
        .context("failed to construct HTTP client")?;

    for (idx, row) in rows.iter().enumerate() {
        let output_path = args.out.join(format!("{}.json", sha256_hex(&row.question)));

        // Benchmarking caches are intentionaly permanent by default to ensure
        // consistent offline evaluation. We only re-fetch if the TTL has expired.
        if let Some(age) = fs::metadata(&output_path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|m| std::time::SystemTime::now().duration_since(m).ok())
            .filter(|&age| age.as_secs() < args.ttl)
        {
            eprintln!(
                "[{}/{}] skipping cached ({} < {}) {}",
                idx + 1,
                rows.len(),
                age.as_secs(),
                args.ttl,
                row.question
            );
            continue;
        }

        let (searxng_res, ddg_res, marginalia_res) = tokio::join!(
            searxng::search(&client, &searxng_url, &row.question, ENGINE_LIMIT),
            duckduckgo::search(&client, &row.question, ENGINE_LIMIT),
            marginalia::search(&client, &row.question, ENGINE_LIMIT),
        );

        let mut failures = 0usize;
        let mut engine_results = BTreeMap::new();
        engine_results.insert(
            "searxng".to_owned(),
            match searxng_res {
                Ok(results) => to_records(results),
                Err(err) => {
                    failures += 1;
                    eprintln!("cache warning [{}] searxng failed: {err}", row.question);
                    Vec::new()
                }
            },
        );
        engine_results.insert(
            "ddg_lite".to_owned(),
            match ddg_res {
                Ok(results) => to_records(results),
                Err(err) => {
                    failures += 1;
                    eprintln!("cache warning [{}] ddg_lite failed: {err}", row.question);
                    Vec::new()
                }
            },
        );
        engine_results.insert(
            "marginalia".to_owned(),
            match marginalia_res {
                Ok(results) => to_records(results),
                Err(err) => {
                    failures += 1;
                    eprintln!("cache warning [{}] marginalia failed: {err}", row.question);
                    Vec::new()
                }
            },
        );

        let all_empty = engine_results.values().all(Vec::is_empty);
        let exit_code = if all_empty {
            Some(if failures == engine_results.len() {
                3
            } else {
                4
            })
        } else {
            Some(0)
        };

        let cache = CacheFile {
            question: row.question.clone(),
            source_url: row.source_url.clone(),
            answer: row.answer.clone(),
            engine_results,
            cached_at: unix_timestamp_seconds(),
            exit_code,
        };

        let output_path = args.out.join(format!("{}.json", sha256_hex(&row.question)));
        let payload = serde_json::to_vec_pretty(&cache)
            .with_context(|| format!("failed to serialize cache JSON for {}", row.question))?;
        fs::write(&output_path, payload)
            .with_context(|| format!("failed to write {}", output_path.display()))?;

        eprintln!("[{}/{}] cached {}", idx + 1, rows.len(), row.question);
    }

    Ok(())
}

fn run_recall(args: RecallArgs) -> Result<()> {
    let cache_files = load_cache_files(&args.cache)?;
    if cache_files.is_empty() {
        anyhow::bail!("cache directory contains no JSON files");
    }
    let at_k = args.at_k.max(1);

    let cfg = get_config();
    let weights = cfg.search.scoring.clone();
    let trust = &cfg.search.trust;
    let blocklist = &cfg.search.blocklist;

    let mut pass_at_k = 0usize;
    let mut pass_at_1 = 0usize;
    let mut fail = 0usize;
    let mut miss = 0usize;
    let mut blocked = 0usize;
    let mut no_content = 0usize;
    let mut covered = 0usize;
    let mut mrr_sum = 0.0_f64;

    for cache in &cache_files {
        if let Some(3) = cache.exit_code {
            blocked += 1;
            continue;
        }
        if let Some(4) = cache.exit_code {
            no_content += 1;
            continue;
        }

        let ranked = rank_urls(cache, trust, blocklist, &weights);
        if ranked.is_empty() {
            fail += 1;
            continue;
        }

        match find_rank(&ranked, &cache.source_url) {
            Some(rank) => {
                covered += 1;
                mrr_sum += 1.0 / rank as f64;
                if rank == 1 {
                    pass_at_1 += 1;
                }
                if rank <= at_k {
                    pass_at_k += 1;
                } else {
                    miss += 1;
                }
            }
            None => {
                fail += 1;
            }
        }
    }

    let total = cache_files.len();
    println!(
        "split:       {} ({} questions)",
        split_name(&args.cache),
        total
    );
    println!(
        "recall@{}:   {:.1}%  ({} / {})",
        at_k,
        percent(pass_at_k, total),
        pass_at_k,
        total
    );
    println!(
        "recall@1:    {:.1}%  ({} / {})",
        percent(pass_at_1, total),
        pass_at_1,
        total
    );
    println!("MRR:         {:.3}", safe_div(mrr_sum, total as f64));
    println!(
        "coverage:    {:.1}%  ({} / {})",
        percent(covered, total),
        covered,
        total
    );
    println!(
        "blocked:     {:.1}%  ({} / {})",
        percent(blocked, total),
        blocked,
        total
    );
    println!(
        "no-content:  {:.1}%  ({} / {})",
        percent(no_content, total),
        no_content,
        total
    );
    eprintln!("details: pass={pass_at_k} miss={miss} fail={fail}");

    Ok(())
}

/// url-recall measures **coverage**: does any engine return the correct source URL at all,
/// and at what position in the raw (pre-scoring) merged result list?
///
/// This is the lower bound on what any scoring strategy can achieve — if the URL never
/// appears in any engine's raw results, no amount of reranking can surface it.
///
/// Output rows:
///   url-recall@1   — correct URL was the #1 result in the fused raw list
///   url-recall@5   — correct URL in top 5
///   url-recall@<k> — correct URL in top k (configurable via --at-k)
///   url-recall@∞   — correct URL appeared anywhere (coverage)
///   coverage-gap   — correct URL was never returned by any engine
fn run_url_recall(args: UrlRecallArgs) -> Result<()> {
    let cache_files = load_cache_files(&args.cache)?;
    if cache_files.is_empty() {
        anyhow::bail!("cache directory contains no JSON files");
    }
    let at_k = args.at_k.max(1);

    // Collect all k thresholds we want to report (always include 1, 5, 10, at_k, ∞).
    // We store results per cutoff without re-scanning.
    let report_ks: Vec<usize> = {
        let mut ks = vec![1usize, 5, 10, at_k];
        ks.dedup();
        ks.sort_unstable();
        ks.dedup();
        ks
    };

    let mut pass_at: HashMap<usize, usize> = report_ks.iter().map(|&k| (k, 0)).collect();
    let mut pass_inf = 0usize; // covered by any engine at any rank
    let mut skipped_engine_fail = 0usize;

    // Per-question raw rank across all engines (RRF-fused by appearance order, no scoring).
    // We collect all unique URLs in the order they appear across engines, deduplicated.
    for cache in &cache_files {
        // Skip questions where all engines hard-failed.
        let all_empty = cache.engine_results.values().all(Vec::is_empty);
        if all_empty {
            skipped_engine_fail += 1;
            continue;
        }

        // Build a raw fused list: interleave engine results round-robin, dedup by
        // normalized URL. This is a simple appearance-order merge (no score weighting)
        // so we measure raw engine coverage, not scoring quality.
        let raw_urls = raw_union_urls(cache);

        // Check each cutoff.
        let rank = raw_urls
            .iter()
            .position(|u| normalize_url_for_eval(u) == normalize_url_for_eval(&cache.source_url))
            .map(|idx| idx + 1); // 1-indexed

        if let Some(r) = rank {
            pass_inf += 1;
            for &k in &report_ks {
                if r <= k {
                    *pass_at.entry(k).or_insert(0) += 1;
                }
            }
        }
    }

    let total = cache_files.len();

    println!(
        "split:        {} ({} questions)",
        split_name(&args.cache),
        total
    );
    if skipped_engine_fail > 0 {
        eprintln!(
            "note: {} questions had no results from any engine",
            skipped_engine_fail
        );
    }
    println!();

    for &k in &report_ks {
        let hits = pass_at[&k];
        println!(
            "url-recall@{: <2}  {:.1}%   ({} / {})",
            k,
            percent(hits, total),
            hits,
            total
        );
    }
    println!(
        "url-recall@∞   {:.1}%   ({} / {})   ← correct URL appeared anywhere in results",
        percent(pass_inf, total),
        pass_inf,
        total
    );
    println!(
        "coverage gap:   {:.1}%   ({} / {})   ← correct URL not surfaced by any engine",
        percent(total.saturating_sub(pass_inf), total),
        total.saturating_sub(pass_inf),
        total
    );

    Ok(())
}

/// Build a deduplicated, interleaved union of all engine result URLs for a cached question.
///
/// This is a round-robin merge that preserves relative engine order without any score weighting.
/// The result represents raw engine coverage — what URLs were returned at all, in what order
/// they first appeared across engines.
fn raw_union_urls(cache: &CacheFile) -> Vec<String> {
    // Collect per-engine URL lists in deterministic order (BTreeMap gives this).
    let engine_url_lists: Vec<Vec<String>> = cache
        .engine_results
        .values()
        .map(|records| records.iter().map(|r| r.url.clone()).collect())
        .collect();

    let max_len = engine_url_lists.iter().map(Vec::len).max().unwrap_or(0);
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut merged: Vec<String> = Vec::new();

    // Round-robin interleave: position 0 of each engine, then position 1, etc.
    for pos in 0..max_len {
        for list in &engine_url_lists {
            if let Some(url) = list.get(pos) {
                let norm = normalize_url_for_eval(url);
                if seen.insert(norm) {
                    merged.push(url.clone());
                }
            }
        }
    }

    merged
}

fn run_tune(args: TuneArgs) -> Result<()> {
    let cache_files = load_cache_files(&args.cache)?;
    if cache_files.is_empty() {
        anyhow::bail!("cache directory contains no JSON files");
    }

    let cfg = get_config();
    let tuned = coordinate_ascent(
        &cache_files,
        cfg.search.scoring.clone(),
        args.delta.max(0.0001),
        args.patience.max(1),
        &cfg.search.trust,
        &cfg.search.blocklist,
    );
    let best_mrr = compute_mrr(
        &cache_files,
        &cfg.search.trust,
        &cfg.search.blocklist,
        &tuned,
    );

    println!(
        "# Tuned weights — MRR: {:.3} on {} ({} questions)",
        best_mrr,
        split_name(&args.cache),
        cache_files.len()
    );
    println!("# Generated: {}", unix_timestamp_seconds());
    println!("[search.scoring]");
    println!("domain_trust = {:.6}", tuned.domain_trust);
    println!("domain_diversity = {:.6}", tuned.domain_diversity);
    println!("snippet_relevance = {:.6}", tuned.snippet_relevance);
    println!("url_pattern = {:.6}", tuned.url_pattern);
    println!("blocklist_penalty = {:.6}", tuned.blocklist_penalty);
    println!("project_match = {:.6}", tuned.project_match);
    println!("rrf_k = {:.6}", tuned.rrf_k);

    Ok(())
}

fn run_domains(args: DomainsArgs) -> Result<()> {
    if args.inputs.is_empty() {
        anyhow::bail!("at least one --inputs path is required");
    }

    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut split_summaries: Vec<String> = Vec::new();

    for input in &args.inputs {
        let rows = load_ref_split(input)?;
        split_summaries.push(format!("{} ({})", split_name(input), rows.len()));
        for row in rows {
            if let Some(host) = parse_host(&row.source_url) {
                *counts.entry(host).or_insert(0) += 1;
            }
        }
    }

    let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.truncate(args.top);

    let mut markdown = String::new();
    let _ = writeln!(markdown, "# Corpus Domain Analysis");
    let _ = writeln!(
        markdown,
        "Generated: {} | Sources: {}",
        unix_timestamp_seconds(),
        split_summaries.join(" + ")
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "| Rank | Domain | Questions | Fail rate |");
    let _ = writeln!(markdown, "|---|---|---|---|");
    for (idx, (domain, count)) in ranked.iter().enumerate() {
        let _ = writeln!(markdown, "| {} | {} | {} | n/a |", idx + 1, domain, count);
    }

    if let Some(out_path) = &args.out {
        if let Some(parent) = out_path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed creating parent directory for output {}",
                    out_path.display()
                )
            })?;
        }
        fs::write(out_path, markdown)
            .with_context(|| format!("failed writing {}", out_path.display()))?;
    } else {
        print!("{markdown}");
    }

    Ok(())
}

fn load_ref_split(path: &Path) -> Result<Vec<RefQuestion>> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match ext.as_str() {
        "jsonl" => load_ref_jsonl(path),
        "parquet" => {
            anyhow::bail!("parquet input is not currently supported; convert split to JSONL")
        }
        _ => anyhow::bail!("unsupported input type '{}'; expected .jsonl", ext),
    }
}

fn load_ref_jsonl(path: &Path) -> Result<Vec<RefQuestion>> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut rows = Vec::new();
    for (line_no, line) in BufReader::new(file).lines().enumerate() {
        let line = line.with_context(|| {
            format!("failed reading line {} in {}", line_no + 1, path.display())
        })?;
        if line.trim().is_empty() {
            continue;
        }
        rows.push(parse_ref_line(&line, line_no + 1, path)?);
    }
    Ok(rows)
}

fn parse_ref_line(line: &str, line_no: usize, path: &Path) -> Result<RefQuestion> {
    let value: Value = serde_json::from_str(line).with_context(|| {
        format!(
            "{} line {} is not valid JSON: {}",
            path.display(),
            line_no,
            line
        )
    })?;

    let question = pick_string(&value, &["question", "query", "prompt"])
        .with_context(|| format!("{} line {} missing question/query", path.display(), line_no))?;
    let source_url = pick_string(&value, &["source_url", "sourceUrl", "url", "source"])
        .with_context(|| format!("{} line {} missing source_url/url", path.display(), line_no))?;
    let answer =
        pick_string(&value, &["answer", "reference_answer", "gold_answer"]).unwrap_or_default();

    Ok(RefQuestion {
        question,
        source_url,
        answer,
    })
}

fn pick_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn load_cache_files(cache_dir: &Path) -> Result<Vec<CacheFile>> {
    let mut paths: Vec<PathBuf> = fs::read_dir(cache_dir)
        .with_context(|| format!("failed to read cache directory {}", cache_dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    paths.sort();

    let mut out = Vec::new();
    for path in paths {
        let bytes = fs::read(&path)
            .with_context(|| format!("failed to read cache file {}", path.display()))?;
        let file: CacheFile = serde_json::from_slice(&bytes)
            .with_context(|| format!("failed to parse cache JSON {}", path.display()))?;
        out.push(file);
    }
    Ok(out)
}

fn to_records(results: Vec<SearchResult>) -> Vec<SearchResultRecord> {
    results
        .into_iter()
        .map(|r| SearchResultRecord {
            url: r.url,
            title: r.title,
            snippet: r.snippet,
        })
        .collect()
}

fn to_search_results(records: &[SearchResultRecord]) -> Vec<SearchResult> {
    records
        .iter()
        .map(|r| SearchResult {
            url: r.url.clone(),
            title: r.title.clone(),
            snippet: r.snippet.clone(),
        })
        .collect()
}

fn rank_urls(
    cache: &CacheFile,
    trust: &TrustConfig,
    blocklist: &BlocklistConfig,
    weights: &ScoringWeights,
) -> Vec<String> {
    let mut engine_lists: Vec<(&str, Vec<SearchResult>)> = Vec::new();
    for (engine, records) in &cache.engine_results {
        engine_lists.push((engine.as_str(), to_search_results(records)));
    }
    if engine_lists.is_empty() {
        return Vec::new();
    }

    let fused = rrf_fuse_with_k(&engine_lists, weights.rrf_k);
    let scored = score_results_with_weights(fused, &cache.question, trust, blocklist, weights);
    scored.into_iter().map(|s| s.result.url).collect()
}

fn find_rank(ranked: &[String], source_url: &str) -> Option<usize> {
    let target = normalize_url_for_eval(source_url);
    ranked
        .iter()
        .position(|candidate| normalize_url_for_eval(candidate) == target)
        .map(|idx| idx + 1)
}

fn normalize_url_for_eval(url: &str) -> String {
    if let Ok(mut parsed) = url::Url::parse(url) {
        parsed.set_fragment(None);
        let mut path = parsed.path().trim_end_matches('/').to_owned();
        if path.is_empty() {
            path.push('/');
        }
        parsed.set_path(&path);
        return parsed.to_string();
    }
    url.trim().trim_end_matches('/').to_ascii_lowercase()
}

fn compute_mrr(
    cache_files: &[CacheFile],
    trust: &TrustConfig,
    blocklist: &BlocklistConfig,
    weights: &ScoringWeights,
) -> f64 {
    let mut evaluated = 0usize;
    let mut sum = 0.0_f64;
    for cache in cache_files {
        if matches!(cache.exit_code, Some(3 | 4)) {
            continue;
        }
        evaluated += 1;
        let ranked = rank_urls(cache, trust, blocklist, weights);
        if let Some(rank) = find_rank(&ranked, &cache.source_url) {
            sum += 1.0 / rank as f64;
        }
    }
    safe_div(sum, evaluated as f64)
}

fn coordinate_ascent(
    cache_files: &[CacheFile],
    initial: ScoringWeights,
    delta: f64,
    patience: usize,
    trust: &TrustConfig,
    blocklist: &BlocklistConfig,
) -> ScoringWeights {
    let mut weights = initial;
    let mut best_mrr = compute_mrr(cache_files, trust, blocklist, &weights);
    let mut stall_count = 0usize;

    loop {
        let mut improved = false;

        for i in 0..ScoringWeights::TUNABLE_FIELDS {
            let mut plus = weights.clone();
            plus.set(i, plus.get(i) + delta);
            let plus_mrr = compute_mrr(cache_files, trust, blocklist, &plus);

            let mut minus = weights.clone();
            minus.set(i, minus.get(i) - delta);
            let minus_mrr = compute_mrr(cache_files, trust, blocklist, &minus);

            if plus_mrr > best_mrr {
                weights = plus;
                best_mrr = plus_mrr;
                improved = true;
            } else if minus_mrr > best_mrr {
                weights = minus;
                best_mrr = minus_mrr;
                improved = true;
            }
        }

        if improved {
            stall_count = 0;
        } else {
            stall_count += 1;
            if stall_count >= patience {
                break;
            }
        }
    }

    eprintln!("converged: MRR = {:.4}", best_mrr);
    weights
}

fn parse_host(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_owned))
}

fn split_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

fn percent(part: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    (part as f64 / total as f64) * 100.0
}

fn safe_div(numerator: f64, denominator: f64) -> f64 {
    if denominator <= 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

fn unix_timestamp_seconds() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_is_stable() {
        assert_eq!(
            sha256_hex("hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn normalize_url_for_eval_ignores_trailing_slash() {
        let a = normalize_url_for_eval("https://example.com/docs/");
        let b = normalize_url_for_eval("https://example.com/docs");
        assert_eq!(a, b);
    }

    #[test]
    fn parse_ref_line_accepts_query_and_url_aliases() {
        let line = r#"{"query":"tokio async","url":"https://tokio.rs","answer":"x"}"#;
        let row = parse_ref_line(line, 1, Path::new("fixture.jsonl")).expect("parsed row");
        assert_eq!(row.question, "tokio async");
        assert_eq!(row.source_url, "https://tokio.rs");
        assert_eq!(row.answer, "x");
    }

    #[test]
    fn find_rank_matches_after_normalization() {
        let ranked = vec![
            "https://a.com/".to_owned(),
            "https://b.com/path".to_owned(),
            "https://c.com".to_owned(),
        ];
        assert_eq!(find_rank(&ranked, "https://b.com/path/"), Some(2));
    }

    #[test]
    fn split_name_prefers_file_stem() {
        let path = Path::new("/tmp/regression_ref.jsonl");
        assert_eq!(split_name(path), "regression_ref");
    }

    /// raw_union_urls should interleave engine results round-robin and deduplicate.
    #[test]
    fn raw_union_urls_interleaves_and_deduplicates() {
        use ripweb::search::eval_types::SearchResultRecord;
        let mut engine_results = BTreeMap::new();

        // Engine A returns [a, b, c]
        engine_results.insert(
            "engine_a".to_owned(),
            vec![
                SearchResultRecord {
                    url: "https://a.com".to_owned(),
                    title: String::new(),
                    snippet: None,
                },
                SearchResultRecord {
                    url: "https://b.com".to_owned(),
                    title: String::new(),
                    snippet: None,
                },
                SearchResultRecord {
                    url: "https://c.com".to_owned(),
                    title: String::new(),
                    snippet: None,
                },
            ],
        );

        // Engine B returns [a, d] — a is a duplicate, d is new
        engine_results.insert(
            "engine_b".to_owned(),
            vec![
                SearchResultRecord {
                    url: "https://a.com".to_owned(),
                    title: String::new(),
                    snippet: None,
                },
                SearchResultRecord {
                    url: "https://d.com".to_owned(),
                    title: String::new(),
                    snippet: None,
                },
            ],
        );

        let cache = CacheFile {
            question: "test".to_owned(),
            source_url: "https://d.com".to_owned(),
            answer: String::new(),
            engine_results,
            cached_at: "0".to_owned(),
            exit_code: Some(0),
        };

        let union = raw_union_urls(&cache);
        // BTreeMap orders engine_a before engine_b.
        // Round-robin: pos0 → a.com, a.com (dup skip) → a.com first, then b.com (pos1 engine_a),
        // d.com (pos1 engine_b), then c.com (pos2 engine_a).
        // Expected order: a, b, d, c
        assert_eq!(
            union,
            vec![
                "https://a.com",
                "https://b.com",
                "https://d.com",
                "https://c.com",
            ]
        );

        // d.com appears at position 3 (1-indexed), within @3 but not @2
        let rank = union
            .iter()
            .position(|u| normalize_url_for_eval(u) == normalize_url_for_eval("https://d.com"))
            .map(|i| i + 1);
        assert_eq!(rank, Some(3));
    }

    /// A question where no engine returns the source URL should contribute to coverage-gap.
    #[test]
    fn raw_union_urls_returns_empty_on_no_results() {
        let cache = CacheFile {
            question: "obscure question".to_owned(),
            source_url: "https://obscure.example.com/page".to_owned(),
            answer: String::new(),
            engine_results: BTreeMap::new(),
            cached_at: "0".to_owned(),
            exit_code: Some(4),
        };
        let union = raw_union_urls(&cache);
        assert!(union.is_empty());
    }
}
