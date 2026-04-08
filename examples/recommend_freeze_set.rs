use std::collections::{HashMap, HashSet};
use std::fs;

const INPUT_PATH: &str = "corpus/seeds/search_results_urls.normalized.csv";
const OUTPUT_PATH: &str = "corpus/seeds/freeze_candidates.csv";
const MAX_PER_CATEGORY: usize = 10;
const MAX_PER_DOMAIN: usize = 2;

#[derive(Debug, Clone)]
struct SeedRow {
    category_slug: String,
    category_label: String,
    source_row: usize,
    url: String,
    note: String,
    raw: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CandidateKind {
    Reference,
    Generic,
    Platform,
    QueryLike,
}

impl CandidateKind {
    fn as_str(self) -> &'static str {
        match self {
            CandidateKind::Reference => "reference",
            CandidateKind::Generic => "generic",
            CandidateKind::Platform => "platform",
            CandidateKind::QueryLike => "query_like",
        }
    }
}

#[derive(Debug, Clone)]
struct CandidateRow {
    category_slug: String,
    category_label: String,
    priority: usize,
    candidate_kind: CandidateKind,
    review_status: &'static str,
    domain: String,
    url: String,
    note: String,
    rationale: String,
    source_row: usize,
}

#[derive(Debug, Clone)]
struct ScoredRow {
    seed: SeedRow,
    domain: String,
    kind: CandidateKind,
    score: i32,
    rationale: String,
}

fn main() {
    let rows = parse_seed_rows(INPUT_PATH);
    let mut by_category: HashMap<String, Vec<ScoredRow>> = HashMap::new();

    for row in rows {
        if let Some(scored) = score_row(row) {
            by_category
                .entry(scored.seed.category_slug.clone())
                .or_default()
                .push(scored);
        }
    }

    let mut selected: Vec<CandidateRow> = Vec::new();

    for rows in by_category.values_mut() {
        rows.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.domain.cmp(&b.domain))
                .then_with(|| a.seed.url.cmp(&b.seed.url))
        });

        let mut picked_urls: HashSet<String> = HashSet::new();
        let mut per_domain: HashMap<String, usize> = HashMap::new();
        let mut category_selection: Vec<ScoredRow> = Vec::new();

        for target_kind in [
            CandidateKind::Reference,
            CandidateKind::Generic,
            CandidateKind::Platform,
            CandidateKind::QueryLike,
        ] {
            for row in rows.iter().filter(|row| row.kind == target_kind) {
                if category_selection.len() >= MAX_PER_CATEGORY {
                    break;
                }
                if picked_urls.contains(&row.seed.url) {
                    continue;
                }
                if per_domain.get(&row.domain).copied().unwrap_or(0) >= MAX_PER_DOMAIN {
                    continue;
                }
                picked_urls.insert(row.seed.url.clone());
                *per_domain.entry(row.domain.clone()).or_default() += 1;
                category_selection.push(row.clone());
            }
        }

        for row in rows.iter() {
            if category_selection.len() >= MAX_PER_CATEGORY {
                break;
            }
            if picked_urls.contains(&row.seed.url) {
                continue;
            }
            if per_domain.get(&row.domain).copied().unwrap_or(0) >= MAX_PER_DOMAIN {
                continue;
            }
            picked_urls.insert(row.seed.url.clone());
            *per_domain.entry(row.domain.clone()).or_default() += 1;
            category_selection.push(row.clone());
        }

        category_selection.sort_by(|a, b| b.score.cmp(&a.score));

        for (idx, row) in category_selection.into_iter().enumerate() {
            selected.push(CandidateRow {
                category_slug: row.seed.category_slug,
                category_label: row.seed.category_label,
                priority: idx + 1,
                candidate_kind: row.kind,
                review_status: "needs_review",
                domain: row.domain,
                url: row.seed.url,
                note: row.seed.note,
                rationale: row.rationale,
                source_row: row.seed.source_row,
            });
        }
    }

    selected.sort_by(|a, b| {
        a.category_slug
            .cmp(&b.category_slug)
            .then_with(|| a.priority.cmp(&b.priority))
    });

    write_candidate_csv(OUTPUT_PATH, &selected);

    println!("Wrote {}", OUTPUT_PATH);
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
        let count = selected
            .iter()
            .filter(|row| row.category_slug == category)
            .count();
        println!("{category}: {count} candidates");
    }
}

fn parse_seed_rows(path: &str) -> Vec<SeedRow> {
    let text = fs::read_to_string(path).expect("read normalized seed csv");
    let mut lines = text.lines();
    let header = lines.next().expect("normalized csv header");
    let header_fields = parse_csv_line(header);
    let expected = [
        "category_slug",
        "category_label",
        "source_row",
        "url",
        "note",
        "raw",
    ];
    assert_eq!(header_fields, expected, "unexpected normalized csv header");

    lines
        .filter(|line| !line.trim().is_empty())
        .map(parse_csv_line)
        .map(|fields| SeedRow {
            category_slug: fields[0].clone(),
            category_label: fields[1].clone(),
            source_row: fields[2].parse().expect("source_row usize"),
            url: fields[3].clone(),
            note: fields[4].clone(),
            raw: fields[5].clone(),
        })
        .collect()
}

fn write_candidate_csv(path: &str, rows: &[CandidateRow]) {
    let mut out = String::new();
    out.push_str("category_slug,category_label,priority,candidate_kind,review_status,domain,url,note,rationale,source_row\n");
    for row in rows {
        let fields = [
            row.category_slug.as_str(),
            row.category_label.as_str(),
            &row.priority.to_string(),
            row.candidate_kind.as_str(),
            row.review_status,
            row.domain.as_str(),
            row.url.as_str(),
            row.note.as_str(),
            row.rationale.as_str(),
            &row.source_row.to_string(),
        ];
        out.push_str(&fields.into_iter().map(escape_csv_field).collect::<Vec<_>>().join(","));
        out.push('\n');
    }
    fs::write(path, out).expect("write freeze candidate csv");
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

fn score_row(seed: SeedRow) -> Option<ScoredRow> {
    let domain = normalize_domain(&seed.url)?;
    let path = path_from_url(&seed.url);
    if path.ends_with(".pdf") {
        return None;
    }

    let kind = classify_kind(&domain, &seed.url);
    let mut score = 100i32;
    let mut rationale = Vec::new();

    match kind {
        CandidateKind::Reference => {
            score += 25;
            rationale.push("reference-source");
        }
        CandidateKind::Generic => {
            score += 10;
            rationale.push("generic-html");
        }
        CandidateKind::Platform => {
            score += 4;
            rationale.push("important-platform");
        }
        CandidateKind::QueryLike => {
            score -= 10;
            rationale.push("query-like-url");
        }
    }

    if seed.url.contains('?') {
        score -= 8;
        rationale.push("has-query");
    } else {
        score += 4;
        rationale.push("clean-url");
    }

    let depth = path.split('/').filter(|part| !part.is_empty()).count();
    if depth >= 2 {
        score += 8;
        rationale.push("deeper-path");
    }

    if !seed.note.is_empty() {
        rationale.push("search-timestamp-note");
    }

    if domain.ends_with(".gov") || domain.ends_with(".edu") {
        score += 12;
        rationale.push("public-institution");
    }

    if domain.contains("reddit.com")
        || domain.contains("youtube.com")
        || domain.contains("amazon.com")
        || domain.contains("facebook.com")
        || domain.contains("quora.com")
        || domain.contains("x.com")
    {
        rationale.push("hostile-or-ugc");
    }

    if seed.raw.contains("srsltid=") {
        score -= 8;
        rationale.push("tracking-heavy");
    }

    Some(ScoredRow {
        seed,
        domain,
        kind,
        score,
        rationale: rationale.join("|"),
    })
}

fn classify_kind(domain: &str, url: &str) -> CandidateKind {
    if is_reference_domain(domain) {
        CandidateKind::Reference
    } else if is_platform_domain(domain) {
        CandidateKind::Platform
    } else if url.contains('?')
        || url.contains("/search")
        || url.contains("/s?")
        || url.contains("/watch?")
    {
        CandidateKind::QueryLike
    } else {
        CandidateKind::Generic
    }
}

fn is_reference_domain(domain: &str) -> bool {
    domain.ends_with(".gov")
        || domain.ends_with(".edu")
        || matches!(
            domain,
            "wikipedia.org"
                | "en.wikipedia.org"
                | "britannica.com"
                | "pmc.ncbi.nlm.nih.gov"
                | "ncbi.nlm.nih.gov"
                | "nih.gov"
                | "science.nasa.gov"
                | "nature.com"
                | "sciencedirect.com"
                | "link.springer.com"
        )
}

fn is_platform_domain(domain: &str) -> bool {
    matches!(
        domain,
        "reddit.com"
            | "youtube.com"
            | "amazon.com"
            | "facebook.com"
            | "quora.com"
            | "x.com"
            | "github.com"
            | "stackoverflow.com"
    )
}

fn normalize_domain(url: &str) -> Option<String> {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let host = without_scheme.split('/').next()?.split('?').next()?.to_lowercase();
    Some(host.strip_prefix("www.").unwrap_or(&host).to_owned())
}

fn path_from_url(url: &str) -> &str {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    match without_scheme.find('/') {
        Some(idx) => &without_scheme[idx..],
        None => "/",
    }
}
