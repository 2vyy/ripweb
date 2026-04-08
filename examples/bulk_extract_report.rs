use ripweb::{
    corpus::WEB_FIXTURES,
    extract::web::WebExtractor,
    minify::collapse,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const REVIEW_PATH: &str = "corpus/seeds/freeze_review.csv";
const REPORT_DIR: &str = "corpus/reports";
const REPORT_CSV: &str = "corpus/reports/bulk_extract_report.csv";
const REPORT_MD: &str = "corpus/reports/bulk_extract_report.md";

#[derive(Debug, Clone)]
struct ReviewRow {
    url: String,
    fixture_name: String,
    corpus_bucket: String,
    fetch_status: String,
    decision: String,
}

#[derive(Debug, Clone)]
struct ReportRow {
    source: String,
    bucket: String,
    name: String,
    origin_url: String,
    status: String,
    flags: String,
    input_bytes: usize,
    markdown_bytes: usize,
    aggressive_bytes: usize,
    word_count: usize,
    heading_count: usize,
    paragraph_count: usize,
    code_fence_count: usize,
    link_count: usize,
}

fn main() {
    fs::create_dir_all(REPORT_DIR).expect("create report dir");

    let mut rows = Vec::new();

    for fixture in WEB_FIXTURES {
        let path = PathBuf::from(fixture.html_path);
        println!("Analyzing shared fixture {}", fixture.name);
        rows.push(analyze_fixture(
            "shared_corpus",
            "web",
            fixture.name,
            "",
            &path,
        ));
    }

    for review in parse_review_rows(REVIEW_PATH) {
        if review.decision != "accept" || review.fetch_status != "frozen" {
            continue;
        }
        if review.fixture_name.is_empty() || review.corpus_bucket.is_empty() {
            continue;
        }

        let path = Path::new("corpus")
            .join("frozen")
            .join(&review.corpus_bucket)
            .join(format!("{}.html", review.fixture_name));

        println!(
            "Analyzing frozen fixture {}/{}",
            review.corpus_bucket, review.fixture_name
        );
        rows.push(analyze_fixture(
            "freeze_review",
            &review.corpus_bucket,
            &review.fixture_name,
            &review.url,
            &path,
        ));
    }

    write_report_csv(REPORT_CSV, &rows);
    write_report_md(REPORT_MD, &rows);

    println!("Wrote {}", REPORT_CSV);
    println!("Wrote {}", REPORT_MD);
}

fn analyze_fixture(
    source: &str,
    bucket: &str,
    name: &str,
    origin_url: &str,
    path: &Path,
) -> ReportRow {
    let started = Instant::now();
    let Ok(bytes) = fs::read(path) else {
        return ReportRow {
            source: source.to_owned(),
            bucket: bucket.to_owned(),
            name: name.to_owned(),
            origin_url: origin_url.to_owned(),
            status: "missing_fixture".to_owned(),
            flags: "missing_fixture".to_owned(),
            input_bytes: 0,
            markdown_bytes: 0,
            aggressive_bytes: 0,
            word_count: 0,
            heading_count: 0,
            paragraph_count: 0,
            code_fence_count: 0,
            link_count: 0,
        };
    };

    let input_bytes = bytes.len();
    let markdown = WebExtractor::extract_with_url(
        &bytes,
        Some("text/html; charset=utf-8"),
        (!origin_url.is_empty()).then_some(origin_url),
    )
    .unwrap_or_default();
    let aggressive = collapse(&markdown);
    let stats = analyze_text(&markdown);

    let mut flags = Vec::new();
    if markdown.trim().is_empty() {
        flags.push("empty_output");
    }
    if stats.word_count < 40 {
        flags.push("too_short");
    }
    if stats.link_count > stats.word_count.saturating_div(8).max(8) {
        flags.push("link_heavy");
    }
    if stats.heading_count == 0 && stats.paragraph_count <= 1 && stats.word_count > 120 {
        flags.push("flat_structure");
    }
    if input_bytes > 0 && markdown.len() > input_bytes {
        flags.push("output_longer_than_input");
    }
    if aggressive.len() >= markdown.len() && !markdown.is_empty() {
        flags.push("aggressive_not_smaller");
    }

    let status = if flags.is_empty() { "ok" } else { "needs_review" };
    println!(
        "Finished {source}:{bucket}/{name} in {:?}",
        started.elapsed()
    );

    ReportRow {
        source: source.to_owned(),
        bucket: bucket.to_owned(),
        name: name.to_owned(),
        origin_url: origin_url.to_owned(),
        status: status.to_owned(),
        flags: flags.join("|"),
        input_bytes,
        markdown_bytes: markdown.len(),
        aggressive_bytes: aggressive.len(),
        word_count: stats.word_count,
        heading_count: stats.heading_count,
        paragraph_count: stats.paragraph_count,
        code_fence_count: stats.code_fence_count,
        link_count: stats.link_count,
    }
}

#[derive(Default)]
struct TextStats {
    word_count: usize,
    heading_count: usize,
    paragraph_count: usize,
    code_fence_count: usize,
    link_count: usize,
}

fn analyze_text(text: &str) -> TextStats {
    let mut stats = TextStats {
        word_count: text.split_whitespace().count(),
        paragraph_count: text
            .split("\n\n")
            .filter(|chunk| !chunk.trim().is_empty())
            .count(),
        ..TextStats::default()
    };

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            stats.heading_count += 1;
        }
        if trimmed.starts_with("```") {
            stats.code_fence_count += 1;
        }
        stats.link_count += trimmed.matches("](").count();
    }

    stats
}

fn parse_review_rows(path: &str) -> Vec<ReviewRow> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut lines = text.lines();
    if lines.next().is_none() {
        return Vec::new();
    }

    lines
        .filter(|line| !line.trim().is_empty())
        .map(parse_csv_line)
        .map(|fields| ReviewRow {
            url: fields[5].clone(),
            fixture_name: fields[11].clone(),
            corpus_bucket: fields[12].clone(),
            fetch_status: fields[13].clone(),
            decision: fields[9].clone(),
        })
        .collect()
}

fn write_report_csv(path: &str, rows: &[ReportRow]) {
    let mut out = String::new();
    out.push_str("source,bucket,name,origin_url,status,flags,input_bytes,markdown_bytes,aggressive_bytes,word_count,heading_count,paragraph_count,code_fence_count,link_count\n");
    for row in rows {
        let fields = [
            row.source.as_str(),
            row.bucket.as_str(),
            row.name.as_str(),
            row.origin_url.as_str(),
            row.status.as_str(),
            row.flags.as_str(),
            &row.input_bytes.to_string(),
            &row.markdown_bytes.to_string(),
            &row.aggressive_bytes.to_string(),
            &row.word_count.to_string(),
            &row.heading_count.to_string(),
            &row.paragraph_count.to_string(),
            &row.code_fence_count.to_string(),
            &row.link_count.to_string(),
        ];
        out.push_str(&fields.into_iter().map(escape_csv_field).collect::<Vec<_>>().join(","));
        out.push('\n');
    }
    fs::write(path, out).expect("write bulk report csv");
}

fn write_report_md(path: &str, rows: &[ReportRow]) {
    let total = rows.len();
    let ok = rows.iter().filter(|row| row.status == "ok").count();
    let needs_review = rows.iter().filter(|row| row.status == "needs_review").count();
    let missing = rows.iter().filter(|row| row.status == "missing_fixture").count();

    let mut out = String::new();
    out.push_str("# Bulk Extract Report\n\n");
    out.push_str("This report is for bulk stress/evaluation, not exact golden validation.\n\n");
    out.push_str(&format!(
        "- total rows: {total}\n- ok: {ok}\n- needs_review: {needs_review}\n- missing_fixture: {missing}\n\n"
    ));

    out.push_str("## Flagged Rows\n\n");
    out.push_str("| name | bucket | status | flags | markdown_bytes | words |\n");
    out.push_str("| --- | --- | --- | --- | ---: | ---: |\n");
    for row in rows.iter().filter(|row| row.status != "ok") {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            row.name,
            row.bucket,
            row.status,
            if row.flags.is_empty() { "-" } else { row.flags.as_str() },
            row.markdown_bytes,
            row.word_count
        ));
    }

    fs::write(path, out).expect("write bulk report markdown");
}

fn escape_csv_field(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
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
