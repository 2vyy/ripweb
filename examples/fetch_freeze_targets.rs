use ripweb::fetch::{
    client::build_client,
    preflight::PreflightCheck,
    FetchError, RetryConfig,
};
use std::fs;
use std::path::Path;

const TARGETS_CSV: &str = "corpus/frozen/fetch_targets.csv";
const RESULTS_MD: &str = "corpus/frozen/fetch_results.md";

#[derive(Debug, Clone)]
struct TargetRow {
    url: String,
    fixture_name: String,
    corpus_bucket: String,
    local_state: String,
    file_path: String,
}

#[derive(Debug, Clone)]
struct FetchResultRow {
    fixture_name: String,
    corpus_bucket: String,
    url: String,
    action: String,
    status: String,
    detail: String,
    file_path: String,
}

#[tokio::main]
async fn main() {
    let opts = parse_args(std::env::args().skip(1).collect());
    let rows = parse_targets(TARGETS_CSV);

    let selected: Vec<_> = rows
        .into_iter()
        .filter(|row| opts.refresh || row.local_state != "frozen")
        .filter(|row| {
            opts.fixture_filter
                .as_ref()
                .is_none_or(|name| &row.fixture_name == name)
        })
        .take(opts.limit.unwrap_or(usize::MAX))
        .collect();

    if selected.is_empty() {
        println!("No matching freeze targets to fetch.");
        return;
    }

    if opts.dry_run {
        write_results_md(
            RESULTS_MD,
            &selected
                .iter()
                .map(|row| FetchResultRow {
                    fixture_name: row.fixture_name.clone(),
                    corpus_bucket: row.corpus_bucket.clone(),
                    url: row.url.clone(),
                    action: "dry_run".to_owned(),
                    status: "planned".to_owned(),
                    detail: format!("would write {}", row.file_path),
                    file_path: row.file_path.clone(),
                })
                .collect::<Vec<_>>(),
        );
        println!("Dry run only. Wrote {}", RESULTS_MD);
        return;
    }

    let client = build_client().expect("build http client");
    let retry = RetryConfig::default();
    let mut results = Vec::new();

    for row in selected {
        println!("Fetching {} -> {}", row.url, row.file_path);
        let result = fetch_one(&client, &retry, &row).await;
        println!("  {}: {}", result.status, result.detail);
        results.push(result);
    }

    write_results_md(RESULTS_MD, &results);
    println!("Wrote {}", RESULTS_MD);
    println!("Rerun `cargo run --example prepare_freeze_targets` after updating fetch_status values.");
}

async fn fetch_one(
    client: &rquest::Client,
    retry: &RetryConfig,
    row: &TargetRow,
) -> FetchResultRow {
    match fetch_bytes(client, retry, &row.url).await {
        Ok(bytes) => {
            let path = Path::new(&row.file_path);
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            match fs::write(path, &bytes) {
                Ok(_) => FetchResultRow {
                    fixture_name: row.fixture_name.clone(),
                    corpus_bucket: row.corpus_bucket.clone(),
                    url: row.url.clone(),
                    action: if row.local_state == "frozen" {
                        "refresh".to_owned()
                    } else {
                        "fetch".to_owned()
                    },
                    status: "ok".to_owned(),
                    detail: format!("saved {} bytes", bytes.len()),
                    file_path: row.file_path.clone(),
                },
                Err(err) => FetchResultRow {
                    fixture_name: row.fixture_name.clone(),
                    corpus_bucket: row.corpus_bucket.clone(),
                    url: row.url.clone(),
                    action: "fetch".to_owned(),
                    status: "write_failed".to_owned(),
                    detail: err.to_string(),
                    file_path: row.file_path.clone(),
                },
            }
        }
        Err(err) => FetchResultRow {
            fixture_name: row.fixture_name.clone(),
            corpus_bucket: row.corpus_bucket.clone(),
            url: row.url.clone(),
            action: "fetch".to_owned(),
            status: "fetch_failed".to_owned(),
            detail: err,
            file_path: row.file_path.clone(),
        },
    }
}

async fn fetch_bytes(
    client: &rquest::Client,
    retry: &RetryConfig,
    url: &str,
) -> Result<Vec<u8>, String> {
    let resp = ripweb::fetch::client::fetch_with_retry(client, url, retry)
        .await
        .map_err(describe_fetch_error)?;

    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned());
    let content_length = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());

    PreflightCheck::validate(content_type.as_deref(), content_length)
        .map_err(|err| err.to_string())?;

    resp.bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|err| err.to_string())
}

fn write_results_md(path: &str, rows: &[FetchResultRow]) {
    let ok = rows.iter().filter(|row| row.status == "ok").count();
    let failed = rows.len().saturating_sub(ok);

    let mut out = String::new();
    out.push_str("# Freeze Fetch Results\n\n");
    out.push_str("- this file is generated by `cargo run --example fetch_freeze_targets`\n");
    out.push_str("- rerun `cargo run --example prepare_freeze_targets` after fetches complete\n");
    out.push_str("- then update `corpus/seeds/freeze_review.csv` rows to `fetch_status=frozen` for successful snapshots\n\n");
    out.push_str(&format!(
        "- total targets: {}\n- ok: {}\n- failed: {}\n\n",
        rows.len(),
        ok,
        failed
    ));
    out.push_str("| fixture | bucket | status | action | detail | path | url |\n");
    out.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");
    for row in rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            row.fixture_name, row.corpus_bucket, row.status, row.action, row.detail, row.file_path, row.url
        ));
    }
    fs::write(path, out).expect("write fetch results");
}

fn parse_targets(path: &str) -> Vec<TargetRow> {
    let text = fs::read_to_string(path).expect("read fetch_targets.csv");
    let mut lines = text.lines();
    let header = parse_csv_line(lines.next().expect("targets header"));
    let expected = [
        "category_slug",
        "candidate_kind",
        "domain",
        "url",
        "fixture_name",
        "corpus_bucket",
        "review_fetch_status",
        "local_state",
        "file_path",
    ];
    assert_eq!(header, expected, "unexpected fetch targets csv header");

    lines
        .filter(|line| !line.trim().is_empty())
        .map(parse_csv_line)
        .map(|fields| TargetRow {
            url: fields[3].clone(),
            fixture_name: fields[4].clone(),
            corpus_bucket: fields[5].clone(),
            local_state: fields[7].clone(),
            file_path: fields[8].clone(),
        })
        .collect()
}

#[derive(Default)]
struct Args {
    dry_run: bool,
    refresh: bool,
    limit: Option<usize>,
    fixture_filter: Option<String>,
}

fn parse_args(args: Vec<String>) -> Args {
    let mut out = Args::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dry-run" => out.dry_run = true,
            "--refresh" => out.refresh = true,
            "--limit" => {
                if let Some(value) = args.get(i + 1) {
                    out.limit = value.parse::<usize>().ok();
                    i += 1;
                }
            }
            "--fixture" => {
                if let Some(value) = args.get(i + 1) {
                    out.fixture_filter = Some(value.clone());
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    out
}

fn describe_fetch_error(err: FetchError) -> String {
    match err {
        FetchError::Network(e) => format!("network error: {e}"),
        FetchError::ServerError(code) => format!("server error: {code}"),
        FetchError::RateLimited => "rate limited".to_owned(),
    }
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(current);
                current = String::new();
            }
            _ => current.push(ch),
        }
    }

    fields.push(current);
    fields
}
