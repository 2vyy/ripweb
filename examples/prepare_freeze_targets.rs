use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const REVIEW_PATH: &str = "corpus/seeds/freeze_review.csv";
const FROZEN_DIR: &str = "corpus/frozen";
const TARGETS_CSV: &str = "corpus/frozen/fetch_targets.csv";
const STATUS_MD: &str = "corpus/frozen/status.md";

#[derive(Debug, Clone)]
struct ReviewRow {
    category_slug: String,
    candidate_kind: String,
    domain: String,
    url: String,
    decision: String,
    fixture_name: String,
    corpus_bucket: String,
    fetch_status: String,
}

#[derive(Debug, Clone)]
struct ReadyTarget {
    category_slug: String,
    candidate_kind: String,
    domain: String,
    url: String,
    fixture_name: String,
    corpus_bucket: String,
    fetch_status: String,
    file_path: String,
    local_state: String,
}

fn main() {
    fs::create_dir_all(FROZEN_DIR).expect("create corpus/frozen");

    let rows = parse_review_rows(REVIEW_PATH);
    let accepted: Vec<_> = rows.into_iter().filter(|row| row.decision == "accept").collect();

    let mut ready = Vec::new();
    let mut missing_metadata = Vec::new();

    for row in accepted {
        if row.fixture_name.is_empty() || row.corpus_bucket.is_empty() {
            missing_metadata.push(row);
            continue;
        }

        let bucket_dir = Path::new(FROZEN_DIR).join(&row.corpus_bucket);
        fs::create_dir_all(&bucket_dir).expect("create frozen bucket dir");
        let file_path = bucket_dir.join(format!("{}.html", row.fixture_name));
        let local_state = if file_path.exists() {
            "frozen"
        } else {
            "missing_local_html"
        };

        ready.push(ReadyTarget {
            category_slug: row.category_slug,
            candidate_kind: row.candidate_kind,
            domain: row.domain,
            url: row.url,
            fixture_name: row.fixture_name,
            corpus_bucket: row.corpus_bucket,
            fetch_status: row.fetch_status,
            file_path: path_to_repo_string(&file_path),
            local_state: local_state.to_owned(),
        });
    }

    ready.sort_by(|a, b| {
        a.corpus_bucket
            .cmp(&b.corpus_bucket)
            .then_with(|| a.fixture_name.cmp(&b.fixture_name))
    });

    write_targets_csv(TARGETS_CSV, &ready);
    write_status_md(STATUS_MD, &ready, &missing_metadata);

    println!("Wrote {}", TARGETS_CSV);
    println!("Wrote {}", STATUS_MD);
    println!("accepted rows: {}", ready.len() + missing_metadata.len());
    println!("ready targets: {}", ready.len());
    println!("missing metadata: {}", missing_metadata.len());
}

fn write_targets_csv(path: &str, rows: &[ReadyTarget]) {
    let mut out = String::new();
    out.push_str("category_slug,candidate_kind,domain,url,fixture_name,corpus_bucket,review_fetch_status,local_state,file_path\n");
    for row in rows {
        let fields = [
            row.category_slug.as_str(),
            row.candidate_kind.as_str(),
            row.domain.as_str(),
            row.url.as_str(),
            row.fixture_name.as_str(),
            row.corpus_bucket.as_str(),
            row.fetch_status.as_str(),
            row.local_state.as_str(),
            row.file_path.as_str(),
        ];
        out.push_str(
            &fields
                .into_iter()
                .map(escape_csv_field)
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');
    }
    fs::write(path, out).expect("write fetch_targets.csv");
}

fn write_status_md(path: &str, ready: &[ReadyTarget], missing_metadata: &[ReviewRow]) {
    let frozen = ready.iter().filter(|row| row.local_state == "frozen").count();
    let missing_local_html = ready.len().saturating_sub(frozen);

    let mut by_bucket: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for row in ready {
        let entry = by_bucket
            .entry(row.corpus_bucket.as_str())
            .or_insert((0usize, 0usize));
        entry.0 += 1;
        if row.local_state == "frozen" {
            entry.1 += 1;
        }
    }

    let mut out = String::new();
    out.push_str("# Frozen Fixture Status\n\n");
    out.push_str("This directory stores local HTML snapshots promoted from reviewed seed URLs.\n\n");
    out.push_str(
        "Use `cargo run --example prepare_freeze_targets` after editing `corpus/seeds/freeze_review.csv`.\n\n",
    );
    out.push_str(&format!(
        "- accepted rows: {}\n- ready targets: {}\n- already frozen locally: {}\n- missing local html: {}\n- missing fixture metadata: {}\n\n",
        ready.len() + missing_metadata.len(),
        ready.len(),
        frozen,
        missing_local_html,
        missing_metadata.len()
    ));

    out.push_str("## Bucket Summary\n\n");
    out.push_str("| bucket | accepted targets | frozen locally |\n");
    out.push_str("| --- | ---: | ---: |\n");
    for (bucket, (accepted, frozen_count)) in by_bucket {
        out.push_str(&format!("| {bucket} | {accepted} | {frozen_count} |\n"));
    }

    out.push_str("\n## Ready Targets\n\n");
    out.push_str("| fixture | bucket | local_state | review_fetch_status | url |\n");
    out.push_str("| --- | --- | --- | --- | --- |\n");
    for row in ready {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            row.fixture_name, row.corpus_bucket, row.local_state, row.fetch_status, row.url
        ));
    }

    if !missing_metadata.is_empty() {
        out.push_str("\n## Missing Metadata\n\n");
        out.push_str("| category | url | reason |\n");
        out.push_str("| --- | --- | --- |\n");
        for row in missing_metadata {
            let reason = if row.fixture_name.is_empty() && row.corpus_bucket.is_empty() {
                "missing fixture_name and corpus_bucket"
            } else if row.fixture_name.is_empty() {
                "missing fixture_name"
            } else {
                "missing corpus_bucket"
            };
            out.push_str(&format!("| {} | {} | {} |\n", row.category_slug, row.url, reason));
        }
    }

    fs::write(path, out).expect("write frozen status markdown");
}

fn parse_review_rows(path: &str) -> Vec<ReviewRow> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut lines = text.lines();
    let Some(header_line) = lines.next() else {
        return Vec::new();
    };
    let header = parse_csv_line(header_line);
    let expected = [
        "category_slug",
        "category_label",
        "priority",
        "candidate_kind",
        "domain",
        "url",
        "note",
        "rationale",
        "source_row",
        "decision",
        "decision_reason",
        "fixture_name",
        "corpus_bucket",
        "fetch_status",
    ];
    assert_eq!(header, expected, "unexpected review csv header");

    lines
        .filter(|line| !line.trim().is_empty())
        .map(parse_csv_line)
        .map(|fields| ReviewRow {
            category_slug: fields[0].clone(),
            candidate_kind: fields[3].clone(),
            domain: fields[4].clone(),
            url: fields[5].clone(),
            decision: fields[9].clone(),
            fixture_name: fields[11].clone(),
            corpus_bucket: fields[12].clone(),
            fetch_status: fields[13].clone(),
        })
        .collect()
}

fn path_to_repo_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
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
