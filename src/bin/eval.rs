//! LLM Tool-Use Evaluation Harness
//!
//! Benchmarks local models (llama.cpp) using ripweb as a tool.
//! Optimized for Research: tracks accurate tokens via /tokenize and hardware
//! performance via /metrics (Prometheus).

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;

use ripweb::{
    cli::Cli,
    fetch::{RetryConfig, cache::Cache, client::build_client, politeness::DomainSemaphores},
    mode::Mode,
    run::dispatch,
};

#[derive(Parser, Debug)]
#[command(
    name = "ripweb-eval",
    about = "Benchmarking harness for ripweb tool-use"
)]
struct Args {
    /// Path to benchmark cases (jsonl)
    #[arg(short, long, default_value = "eval/benchmarks.jsonl")]
    input: PathBuf,

    /// Output directory for results
    #[arg(short, long, default_value = "eval/results")]
    output: PathBuf,

    /// llama.cpp server base URL
    #[arg(short, long, default_value = "http://localhost:8080")]
    api_url: String,

    /// Model name string for reporting
    #[arg(short, long, default_value = "local-model")]
    model: String,

    /// Max steps in ReAct loop
    #[arg(long, default_value_t = 3)]
    max_steps: usize,

    /// Output density tier (1-3)
    #[arg(long, default_value_t = 2)]
    verbosity: u8,
}

#[derive(Debug, Deserialize, Serialize)]
struct BenchmarkCase {
    id: String,
    category: String,
    question: String,
    ground_truth: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExperimentResult {
    case_id: String,
    category: String,
    model: String,
    verbosity: u8,
    total_time_ms: u64,
    prompt_eval_ms: f64,
    tokens_per_sec: f64,
    prompt_tokens: usize,
    completion_tokens: usize,
    steps: usize,
    success: bool,
    full_transcript: String,
}

struct LlamaClient {
    base_url: String,
    client: Arc<rquest::Client>,
}

impl LlamaClient {
    fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Arc::new(build_client().unwrap()),
        }
    }

    /// Accurate token count using the llama.cpp native endpoint
    async fn tokenize(&self, text: &str) -> Result<usize> {
        let resp = self
            .client
            .post(format!("{}/tokenize", self.base_url))
            .json(&json!({ "content": text }))
            .send()
            .await?;

        let val: serde_json::Value = resp.json().await?;
        Ok(val["tokens"].as_array().map(|a| a.len()).unwrap_or(0))
    }

    /// Scrape hardware performance from the Prometheus metrics endpoint
    async fn get_metrics(&self) -> Result<LlamaMetrics> {
        let resp = self
            .client
            .get(format!("{}/metrics", self.base_url))
            .send()
            .await?;
        let body = resp.text().await?;

        let tps_re = Regex::new(r"llama_tokens_seconds_total\s+([0-9.]+)")?;
        let eval_re = Regex::new(r"llama_prompt_tokens_seconds_total\s+([0-9.]+)")?;

        let tps = tps_re
            .captures(&body)
            .and_then(|c: regex::Captures| {
                c.get(1)
                    .and_then(|m: regex::Match| m.as_str().parse::<f64>().ok())
            })
            .unwrap_or(0.0);

        let eval_ms = eval_re
            .captures(&body)
            .and_then(|c: regex::Captures| {
                c.get(1)
                    .and_then(|m: regex::Match| m.as_str().parse::<f64>().ok())
            })
            .unwrap_or(0.0)
            * 1000.0;

        Ok(LlamaMetrics { tps, eval_ms })
    }

    async fn completion(&self, messages: Vec<serde_json::Value>) -> Result<serde_json::Value> {
        let resp = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&json!({
                "messages": messages,
                "temperature": 0.1,
                "stop": ["Observation:", "###"]
            }))
            .send()
            .await?;

        Ok(resp.json().await?)
    }
}

struct LlamaMetrics {
    tps: f64,
    eval_ms: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let client = LlamaClient::new(args.api_url.clone());

    std::fs::create_dir_all(&args.output)?;
    let jsonl_path = args.output.join("results.jsonl");
    let mut jsonl_file = File::create(&jsonl_path)?;

    let reader = BufReader::new(File::open(&args.input).context("Failed to open benchmarks file")?);

    println!(
        "🚀 Starting Evaluation: {} (Verbosity V{})",
        args.model, args.verbosity
    );
    println!("--------------------------------------------------");

    for line in reader.lines() {
        let case: BenchmarkCase = serde_json::from_str(&line?)?;
        let result = run_experiment(&args, &case, &client).await?;

        writeln!(jsonl_file, "{}", serde_json::to_string(&result)?)?;
        log_summary(&result);
    }

    generate_markdown_report(&args.output, &args.model).await?;

    Ok(())
}

async fn run_experiment(
    args: &Args,
    case: &BenchmarkCase,
    llama: &LlamaClient,
) -> Result<ExperimentResult> {
    let mut transcript = String::new();
    let start_time = Instant::now();
    let mut messages = vec![json!({
        "role": "system",
        "content": format!(
            "You are a research assistant with access to the 'ripweb' web search tool.\n\
            Use the following loop: Thought -> Action -> Observation.\n\n\
            Tool Usage Example:\n\
            Thought: I need to find the latest version of Axum.\n\
            Action: ripweb -q \"axum rust latest stable release\"\n\
            Observation: [Tool output will appear here]\n\n\
            Final Answer: [Your conclusion]\n\n\
            Question: {}", case.question
        )
    })];

    let mut steps = 0;
    let mut prompt_tokens = 0;
    let mut completion_tokens = 0;

    while steps < args.max_steps {
        steps += 1;
        let response = llama.completion(messages.clone()).await?;
        let choice = &response["choices"][0];
        let content = choice["message"]["content"].as_str().unwrap_or("");

        prompt_tokens += response["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize;
        completion_tokens += response["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize;

        transcript.push_str(&format!("\n--- Step {} ---\n{}", steps, content));

        if content.contains("Final Answer:") {
            break;
        }

        if let Some(query) = extract_action(content) {
            println!("  🔍 Tool Action: {}", query);
            let observation = execute_ripweb_tool(query, args.verbosity).await?;

            // Record exact token size of tool output
            let obs_tokens = llama.tokenize(&observation).await?;
            println!("  📄 Observation Density: {} tokens", obs_tokens);

            messages.push(json!({ "role": "assistant", "content": content }));
            messages.push(
                json!({ "role": "user", "content": format!("Observation: {}", observation) }),
            );
            transcript.push_str(&format!(
                "\nObservation: ({} tokens)\n{}",
                obs_tokens, observation
            ));
        } else {
            break;
        }
    }

    let total_time = start_time.elapsed();
    let metrics = llama.get_metrics().await.unwrap_or(LlamaMetrics {
        tps: 0.0,
        eval_ms: 0.0,
    });

    Ok(ExperimentResult {
        case_id: case.id.clone(),
        category: case.category.clone(),
        model: args.model.clone(),
        verbosity: args.verbosity,
        total_time_ms: total_time.as_millis() as u64,
        prompt_eval_ms: metrics.eval_ms,
        tokens_per_sec: metrics.tps,
        prompt_tokens,
        completion_tokens,
        steps,
        success: transcript.contains("Final Answer:"),
        full_transcript: transcript,
    })
}

fn extract_action(content: &str) -> Option<&str> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r#"Action:\s*ripweb\s+-q\s+['"]?(.+?)['"]?($|\n)"#).ok());

    re.as_ref()
        .and_then(|r| r.captures(content))
        .and_then(|c: regex::Captures| c.get(1))
        .map(|m: regex::Match| m.as_str())
}

async fn execute_ripweb_tool(query: &str, verbosity: u8) -> Result<String> {
    let mode = match verbosity {
        1 => Mode::Compact,
        2 => Mode::Balanced,
        3 => Mode::Verbose,
        _ => Mode::Balanced,
    };

    let cli = Cli {
        query_or_url: Some(query.to_string()),
        force_url: false,
        force_query: true,
        engine: Default::default(),
        searxng_url: String::new(),
        max_depth: 1,
        max_pages: 5,
        allow_cloud: false,
        mode,
        stat: false,
        copy: false,
        clean_cache: false,
        verbose: 0,
    };

    let client = Arc::new(build_client()?);
    let (text, _) = dispatch(
        &cli,
        query,
        &client,
        RetryConfig::default(),
        DomainSemaphores::new(3),
        Cache::xdg().map(Arc::new),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Ripweb error: {}", e))?;

    Ok(text)
}

fn log_summary(res: &ExperimentResult) {
    let status = if res.success { "✅" } else { "❌" };
    println!(
        "{} [{}] Case: {} | Tokens: P{} C{} | Time: {}ms",
        status,
        res.category,
        res.case_id,
        res.prompt_tokens,
        res.completion_tokens,
        res.total_time_ms
    );
}

async fn generate_markdown_report(dir: &Path, model: &str) -> Result<()> {
    let jsonl_path = dir.join("results.jsonl");
    let report_path = dir.join("report.md");
    let mut report = File::create(&report_path)?;

    writeln!(report, "# LLM Tool-Use Research Report")?;
    writeln!(report, "\n- **Model**: {}\n- **Date**: 2026-04-10\n", model)?;
    writeln!(
        report,
        "| Case ID | Category | Steps | Prompt Tokens | Completion Tokens | Time (ms) | Success |"
    )?;
    writeln!(report, "|---|---|---|---|---|---|---|")?;

    let file = File::open(jsonl_path)?;
    for line in BufReader::new(file).lines() {
        let res: ExperimentResult = serde_json::from_str(&line?)?;
        writeln!(
            report,
            "| {} | {} | {} | {} | {} | {} | {} |",
            res.case_id,
            res.category,
            res.steps,
            res.prompt_tokens,
            res.completion_tokens,
            res.total_time_ms,
            if res.success { "YES" } else { "NO" }
        )?;
    }

    Ok(())
}
