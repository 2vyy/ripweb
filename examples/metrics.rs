use ripweb::{
    corpus::WEB_FIXTURES,
    extract::{web::WebExtractor, Extractor},
    minify::collapse,
};
use std::collections::HashSet;
use std::fs;

fn tokenize(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|c: char| c.is_ascii_punctuation() || c == '\'' || c == '"')
                .to_string()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

fn compute_metrics(golden: &str, output: &str) -> (f64, f64) {
    let golden_words = tokenize(golden);
    let output_words = tokenize(output);

    let golden_count = golden_words.len();
    let output_count = output_words.len();

    if golden_count == 0 || output_count == 0 {
        return (0.0, 0.0);
    }

    let intersection: HashSet<_> = golden_words.intersection(&output_words).collect();
    let intersection_count = intersection.len();

    let recall = (intersection_count as f64 / golden_count as f64) * 100.0;
    let precision = (intersection_count as f64 / output_count as f64) * 100.0;

    (recall, precision)
}

fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════════════════════════════╗");
    println!(
        "║                         LLM EVALUATION HARNESS                                      ║"
    );
    println!(
        "╠══════════════════════════════════════════════════════════════════════════════════════╣"
    );
    println!(
        "║  Fixture                 │ Kill %   │ Recall % │ Precision %                       ║"
    );
    println!(
        "╠══════════════════════════╪══════════╪══════════╪════════════════════════════════════╣"
    );

    let mut total_kill = 0.0;
    let mut total_recall = 0.0;
    let mut total_precision = 0.0;
    let mut count = 0;
    let mut metrics_count = 0;

    for fixture in WEB_FIXTURES {
        if !fixture.include_in_metrics {
            continue;
        }

        let html_bytes = match fs::read(fixture.html_path) {
            Ok(b) => b,
            Err(e) => {
                println!("  ✗ {} - Failed to read: {}", fixture.name, e);
                continue;
            }
        };

        let raw_len = html_bytes.len();
        let extracted = WebExtractor::extract(&html_bytes, Some("text/html; charset=utf-8"))
            .unwrap_or_default();
        let output = collapse(&extracted);
        let out_len = output.len();

        let kill_ratio = if raw_len > 0 {
            (1.0 - (out_len as f64 / raw_len as f64)) * 100.0
        } else {
            0.0
        };

        total_kill += kill_ratio;
        count += 1;

        // Only compute recall/precision if we have a golden file
        let (recall, precision) = if let Some(golden_path) = fixture.curated_reference_path {
            match fs::read_to_string(golden_path) {
                Ok(golden) => {
                    let metrics = compute_metrics(&golden, &output);
                    total_recall += metrics.0;
                    total_precision += metrics.1;
                    metrics_count += 1;
                    metrics
                }
                Err(_) => (0.0, 0.0),
            }
        } else {
            (0.0, 0.0)
        };

        println!(
            "║  {:<22} │ {:>7.1}% │ {:>7.1}% │ {:>7.1}%                           ║",
            fixture.name, kill_ratio, recall, precision
        );
    }

    let avg_kill = total_kill / count as f64;
    let avg_recall = if metrics_count > 0 {
        total_recall / metrics_count as f64
    } else {
        0.0
    };
    let avg_precision = if metrics_count > 0 {
        total_precision / metrics_count as f64
    } else {
        0.0
    };

    println!(
        "╠══════════════════════════╧══════════╧══════════╧════════════════════════════════════╣"
    );
    println!(
        "║  AVERAGE                 │ {:>7.1}% │ {:>7.1}% │ {:>7.1}%                           ║",
        avg_kill, avg_recall, avg_precision
    );
    println!(
        "║  (metrics: {} fixtures)   │          │          │                                   ║",
        metrics_count
    );
    println!(
        "╚══════════════════════════════════════════════════════════════════════════════════════╝"
    );

    println!("\nTargets: Kill >85%, Recall >95%, Precision >60%");
}
