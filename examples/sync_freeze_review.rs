use std::collections::HashMap;
use std::fs;

const CANDIDATES_PATH: &str = "corpus/seeds/freeze_candidates.csv";
const REVIEW_PATH: &str = "corpus/seeds/freeze_review.csv";

#[derive(Debug, Clone)]
struct CandidateRow {
    category_slug: String,
    category_label: String,
    priority: String,
    candidate_kind: String,
    domain: String,
    url: String,
    note: String,
    rationale: String,
    source_row: String,
}

#[derive(Debug, Clone)]
struct ReviewRow {
    category_slug: String,
    category_label: String,
    priority: String,
    candidate_kind: String,
    domain: String,
    url: String,
    note: String,
    rationale: String,
    source_row: String,
    decision: String,
    decision_reason: String,
    fixture_name: String,
    corpus_bucket: String,
    fetch_status: String,
}

fn main() {
    let candidates = parse_candidates(CANDIDATES_PATH);
    let existing = parse_existing_reviews(REVIEW_PATH);

    let mut merged = Vec::new();
    for candidate in candidates {
        let key = review_key(&candidate.category_slug, &candidate.url);
        let prior = existing.get(&key);
        merged.push(ReviewRow {
            category_slug: candidate.category_slug,
            category_label: candidate.category_label,
            priority: candidate.priority,
            candidate_kind: candidate.candidate_kind,
            domain: candidate.domain,
            url: candidate.url,
            note: candidate.note,
            rationale: candidate.rationale,
            source_row: candidate.source_row,
            decision: prior
                .map(|row| row.decision.clone())
                .unwrap_or_else(|| "pending".to_owned()),
            decision_reason: prior
                .map(|row| row.decision_reason.clone())
                .unwrap_or_default(),
            fixture_name: prior
                .map(|row| row.fixture_name.clone())
                .unwrap_or_default(),
            corpus_bucket: prior
                .map(|row| row.corpus_bucket.clone())
                .unwrap_or_default(),
            fetch_status: prior
                .map(|row| row.fetch_status.clone())
                .unwrap_or_default(),
        });
    }

    write_review_csv(REVIEW_PATH, &merged);

    let pending = merged.iter().filter(|row| row.decision == "pending").count();
    let accepted = merged.iter().filter(|row| row.decision == "accept").count();
    let rejected = merged.iter().filter(|row| row.decision == "reject").count();

    println!("Wrote {}", REVIEW_PATH);
    println!("pending: {}", pending);
    println!("accept: {}", accepted);
    println!("reject: {}", rejected);
}

fn parse_candidates(path: &str) -> Vec<CandidateRow> {
    let text = fs::read_to_string(path).expect("read freeze_candidates.csv");
    let mut lines = text.lines();
    let header = parse_csv_line(lines.next().expect("candidate header"));
    let expected = [
        "category_slug",
        "category_label",
        "priority",
        "candidate_kind",
        "review_status",
        "domain",
        "url",
        "note",
        "rationale",
        "source_row",
    ];
    assert_eq!(header, expected, "unexpected candidate csv header");

    lines
        .filter(|line| !line.trim().is_empty())
        .map(parse_csv_line)
        .map(|fields| CandidateRow {
            category_slug: fields[0].clone(),
            category_label: fields[1].clone(),
            priority: fields[2].clone(),
            candidate_kind: fields[3].clone(),
            domain: fields[5].clone(),
            url: fields[6].clone(),
            note: fields[7].clone(),
            rationale: fields[8].clone(),
            source_row: fields[9].clone(),
        })
        .collect()
}

fn parse_existing_reviews(path: &str) -> HashMap<String, ReviewRow> {
    let Ok(text) = fs::read_to_string(path) else {
        return HashMap::new();
    };

    let mut lines = text.lines();
    let Some(first_line) = lines.next() else {
        return HashMap::new();
    };
    let header = parse_csv_line(first_line);
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
            category_label: fields[1].clone(),
            priority: fields[2].clone(),
            candidate_kind: fields[3].clone(),
            domain: fields[4].clone(),
            url: fields[5].clone(),
            note: fields[6].clone(),
            rationale: fields[7].clone(),
            source_row: fields[8].clone(),
            decision: fields[9].clone(),
            decision_reason: fields[10].clone(),
            fixture_name: fields[11].clone(),
            corpus_bucket: fields[12].clone(),
            fetch_status: fields[13].clone(),
        })
        .map(|row| (review_key(&row.category_slug, &row.url), row))
        .collect()
}

fn write_review_csv(path: &str, rows: &[ReviewRow]) {
    let mut out = String::new();
    out.push_str("category_slug,category_label,priority,candidate_kind,domain,url,note,rationale,source_row,decision,decision_reason,fixture_name,corpus_bucket,fetch_status\n");
    for row in rows {
        let fields = [
            row.category_slug.as_str(),
            row.category_label.as_str(),
            row.priority.as_str(),
            row.candidate_kind.as_str(),
            row.domain.as_str(),
            row.url.as_str(),
            row.note.as_str(),
            row.rationale.as_str(),
            row.source_row.as_str(),
            row.decision.as_str(),
            row.decision_reason.as_str(),
            row.fixture_name.as_str(),
            row.corpus_bucket.as_str(),
            row.fetch_status.as_str(),
        ];
        out.push_str(&fields.into_iter().map(escape_csv_field).collect::<Vec<_>>().join(","));
        out.push('\n');
    }
    fs::write(path, out).expect("write freeze_review.csv");
}

fn review_key(category: &str, url: &str) -> String {
    format!("{category}::{url}")
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
