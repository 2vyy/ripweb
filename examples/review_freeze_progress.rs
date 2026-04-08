use std::collections::HashMap;
use std::fs;

const REVIEW_PATH: &str = "corpus/seeds/freeze_review.csv";

fn main() {
    let rows = parse_review_rows(REVIEW_PATH);

    let mut by_category: HashMap<String, (usize, usize, usize)> = HashMap::new();
    for row in &rows {
        let entry = by_category
            .entry(row.category_slug.clone())
            .or_insert((0, 0, 0));
        match row.decision.as_str() {
            "accept" => entry.0 += 1,
            "reject" => entry.1 += 1,
            _ => entry.2 += 1,
        }
    }

    println!("\nFreeze review progress\n");
    println!("{:<14} {:>7} {:>7} {:>7}", "category", "accept", "reject", "pending");
    println!("{:<14} {:>7} {:>7} {:>7}", "--------------", "-------", "-------", "-------");
    for category in [
        "programming",
        "news",
        "shopping",
        "science",
        "cooking",
        "finance",
        "health",
        "sports",
        "travel",
        "legal",
    ] {
        let (accept, reject, pending) = by_category.get(category).copied().unwrap_or((0, 0, 0));
        println!("{:<14} {:>7} {:>7} {:>7}", category, accept, reject, pending);
    }

    println!("\nAccepted examples:");
    for row in rows.iter().filter(|row| row.decision == "accept").take(10) {
        println!(
            "- [{}] {} ({}) fixture={} bucket={} fetch={}",
            row.category_slug, row.url, row.candidate_kind, row.fixture_name, row.corpus_bucket, row.fetch_status
        );
    }
}

#[derive(Debug, Clone)]
struct ReviewRow {
    category_slug: String,
    decision: String,
    candidate_kind: String,
    url: String,
    fixture_name: String,
    corpus_bucket: String,
    fetch_status: String,
}

fn parse_review_rows(path: &str) -> Vec<ReviewRow> {
    let text = fs::read_to_string(path).expect("read freeze_review.csv");
    let mut lines = text.lines();
    let _header = lines.next().expect("review header");
    lines
        .filter(|line| !line.trim().is_empty())
        .map(parse_csv_line)
        .map(|fields| ReviewRow {
            category_slug: fields[0].clone(),
            decision: fields[9].clone(),
            candidate_kind: fields[3].clone(),
            url: fields[5].clone(),
            fixture_name: fields[11].clone(),
            corpus_bucket: fields[12].clone(),
            fetch_status: fields[13].clone(),
        })
        .collect()
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
