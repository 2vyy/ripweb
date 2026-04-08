use ripweb::{
    corpus::WEB_FIXTURES,
    extract::{web::WebExtractor, Extractor},
    minify::collapse,
};
use std::fs;
use tiktoken_rs::cl100k_base;

const REPORT_CSV: &str = "corpus/reports/tokenizer_audit.csv";
const REPORT_MD: &str = "corpus/reports/tokenizer_audit.md";
const REVIEW_PATH: &str = "corpus/seeds/freeze_review.csv";

#[derive(Clone)]
struct Strategy {
    name: &'static str,
    description: &'static str,
    apply: fn(&str) -> String,
}

#[derive(Debug)]
struct Row {
    source: String,
    name: String,
    strategy: String,
    tokens: usize,
    delta_vs_markdown: isize,
}

#[derive(Debug, Clone)]
struct ReviewRow {
    url: String,
    fixture_name: String,
    corpus_bucket: String,
    fetch_status: String,
    decision: String,
}

fn main() {
    let strategies = vec![
        Strategy {
            name: "markdown",
            description: "Identity baseline.",
            apply: |text| text.trim().to_owned(),
        },
        Strategy {
            name: "aggressive_current",
            description: "Current aggressive mode.",
            apply: collapse,
        },
        Strategy {
            name: "drop_ui_lines",
            description: "Remove low-value UI-only lines such as copy affordances.",
            apply: drop_low_value_ui_lines,
        },
        Strategy {
            name: "strip_heading_anchors",
            description: "Remove decorative heading anchor links.",
            apply: strip_heading_anchors_only,
        },
        Strategy {
            name: "label_only_internal_links",
            description: "Replace low-value internal relative links with labels only.",
            apply: simplify_low_value_links_only,
        },
        Strategy {
            name: "footnote_links",
            description: "Rewrite inline Markdown links to footnotes.",
            apply: rewrite_links_to_footnotes,
        },
    ];

    let bpe = cl100k_base().expect("load cl100k tokenizer");
    let mut rows = Vec::new();

    for fixture in WEB_FIXTURES {
        let bytes = match fs::read(fixture.html_path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let markdown = WebExtractor::extract(&bytes, Some("text/html; charset=utf-8")).unwrap_or_default();
        analyze_document("shared_corpus", fixture.name, &markdown, &strategies, &bpe, &mut rows);
    }

    for review in parse_review_rows(REVIEW_PATH) {
        if review.decision != "accept" || review.fetch_status != "frozen" {
            continue;
        }
        if review.fixture_name.is_empty() || review.corpus_bucket.is_empty() {
            continue;
        }
        let path = format!(
            "corpus/frozen/{}/{}.html",
            review.corpus_bucket, review.fixture_name
        );
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let markdown = WebExtractor::extract(&bytes, Some("text/html; charset=utf-8")).unwrap_or_default();
        let source_name = format!("freeze_review:{}", review.fixture_name);
        analyze_document(&source_name, &review.url, &markdown, &strategies, &bpe, &mut rows);
    }

    write_csv(REPORT_CSV, &rows);
    write_md(REPORT_MD, &rows, &strategies);

    println!("Wrote {}", REPORT_CSV);
    println!("Wrote {}", REPORT_MD);
}

fn analyze_document(
    source: &str,
    name: &str,
    markdown: &str,
    strategies: &[Strategy],
    bpe: &tiktoken_rs::CoreBPE,
    rows: &mut Vec<Row>,
) {
    let baseline = bpe.encode_with_special_tokens(markdown.trim()).len();
    for strategy in strategies {
        let output = (strategy.apply)(markdown);
        let tokens = bpe.encode_with_special_tokens(output.trim()).len();
        rows.push(Row {
            source: source.to_owned(),
            name: name.to_owned(),
            strategy: strategy.name.to_owned(),
            tokens,
            delta_vs_markdown: tokens as isize - baseline as isize,
        });
    }
}

fn write_csv(path: &str, rows: &[Row]) {
    let mut out =
        String::from("source,name,strategy,tokens,delta_vs_markdown\n");
    for row in rows {
        let fields = [
            row.source.as_str(),
            row.name.as_str(),
            row.strategy.as_str(),
            &row.tokens.to_string(),
            &row.delta_vs_markdown.to_string(),
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
    fs::write(path, out).expect("write tokenizer audit csv");
}

fn write_md(path: &str, rows: &[Row], strategies: &[Strategy]) {
    let mut out = String::new();
    out.push_str("# Tokenizer Audit\n\n");
    out.push_str("This report compares candidate aggressive-mode transforms against the OpenAI `cl100k` tokenizer.\n\n");

    out.push_str("## Strategies\n\n");
    for strategy in strategies {
        out.push_str(&format!("- `{}`: {}\n", strategy.name, strategy.description));
    }

    out.push_str("\n## Summary\n\n");
    out.push_str("| strategy | avg delta vs markdown | improved docs | worse docs |\n");
    out.push_str("| --- | ---: | ---: | ---: |\n");
    for strategy in strategies {
        let strategy_rows: Vec<_> = rows.iter().filter(|row| row.strategy == strategy.name).collect();
        if strategy_rows.is_empty() {
            continue;
        }
        let total_delta: isize = strategy_rows.iter().map(|row| row.delta_vs_markdown).sum();
        let improved = strategy_rows.iter().filter(|row| row.delta_vs_markdown < 0).count();
        let worse = strategy_rows.iter().filter(|row| row.delta_vs_markdown > 0).count();
        let avg_delta = total_delta as f64 / strategy_rows.len() as f64;
        out.push_str(&format!(
            "| {} | {avg_delta:.1} | {} | {} |\n",
            strategy.name, improved, worse
        ));
    }

    out.push_str("\n## Per Document\n\n");
    out.push_str("| document | strategy | tokens | delta |\n");
    out.push_str("| --- | --- | ---: | ---: |\n");
    for row in rows {
        out.push_str(&format!(
            "| {} / {} | {} | {} | {} |\n",
            row.source, row.name, row.strategy, row.tokens, row.delta_vs_markdown
        ));
    }

    fs::write(path, out).expect("write tokenizer audit md");
}

fn drop_low_value_ui_lines(text: &str) -> String {
    let mut out = String::new();
    let mut in_code_fence = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if !in_code_fence && matches!(trimmed, "Copy item path" | "Expand description") {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_owned()
}

fn strip_heading_anchors_only(text: &str) -> String {
    text.lines()
        .map(strip_decorative_heading_anchor)
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_decorative_heading_anchor(line: &str) -> String {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return line.to_owned();
    }
    line.replace("[§](#", "§](#")
        .split_once("§](#")
        .and_then(|(prefix, rest)| rest.split_once(')').map(|(_, suffix)| format!("{prefix}{suffix}")))
        .unwrap_or_else(|| line.to_owned())
}

fn simplify_low_value_links_only(text: &str) -> String {
    text.lines()
        .map(simplify_low_value_links_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn simplify_low_value_links_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        let Some(label_end_rel) = line[i + 1..].find(']') else {
            out.push('[');
            i += 1;
            continue;
        };
        let label_end = i + 1 + label_end_rel;
        if bytes.get(label_end + 1) != Some(&b'(') {
            out.push('[');
            i += 1;
            continue;
        }
        let Some(href_end_rel) = line[label_end + 2..].find(')') else {
            out.push('[');
            i += 1;
            continue;
        };
        let href_end = label_end + 2 + href_end_rel;
        let label = &line[i + 1..label_end];
        let href = &line[label_end + 2..href_end];
        if should_inline_link_label_only(label, href) {
            out.push_str(label);
        } else {
            out.push_str(&line[i..=href_end]);
        }
        i = href_end + 1;
    }
    out
}

fn should_inline_link_label_only(label: &str, href: &str) -> bool {
    if label.is_empty() || href.is_empty() {
        return false;
    }
    if href.starts_with('#') {
        return true;
    }
    if label == href {
        return true;
    }
    !href.contains("://") && !href.starts_with('/') && !href.starts_with("mailto:")
}

fn rewrite_links_to_footnotes(text: &str) -> String {
    let mut out = String::new();
    let mut refs: Vec<String> = Vec::new();
    let mut in_code_fence = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            out.push_str(line);
            out.push('\n');
            continue;
        }

        if in_code_fence {
            out.push_str(line);
            out.push('\n');
            continue;
        }

        out.push_str(&rewrite_links_to_footnotes_line(line, &mut refs));
        out.push('\n');
    }

    if !refs.is_empty() {
        out.push('\n');
        for (idx, href) in refs.iter().enumerate() {
            out.push_str(&format!("[{}]: {}\n", idx + 1, href));
        }
    }

    out.trim().to_owned()
}

fn rewrite_links_to_footnotes_line(line: &str, refs: &mut Vec<String>) -> String {
    let mut out = String::with_capacity(line.len());
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        let Some(label_end_rel) = line[i + 1..].find(']') else {
            out.push('[');
            i += 1;
            continue;
        };
        let label_end = i + 1 + label_end_rel;
        if bytes.get(label_end + 1) != Some(&b'(') {
            out.push('[');
            i += 1;
            continue;
        }
        let Some(href_end_rel) = line[label_end + 2..].find(')') else {
            out.push('[');
            i += 1;
            continue;
        };
        let href_end = label_end + 2 + href_end_rel;
        let label = &line[i + 1..label_end];
        let href = &line[label_end + 2..href_end];
        refs.push(href.to_owned());
        out.push_str(&format!("{label}[{}]", refs.len()));
        i = href_end + 1;
    }
    out
}

fn parse_review_rows(path: &str) -> Vec<ReviewRow> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut lines = text.lines();
    let _ = lines.next();
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
