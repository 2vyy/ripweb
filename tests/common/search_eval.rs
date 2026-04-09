#![allow(dead_code)]

use ripweb::search::eval_types::BenchmarkQuery;
use std::path::Path;

/// Load a JSONL benchmark fixture file.
///
/// Each non-empty line must be a valid JSON-serialised `BenchmarkQuery`.
/// Panics with a clear message on I/O or parse failure (test helper).
pub fn load_benchmark(path: &str) -> Vec<BenchmarkQuery> {
    let full_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let content = std::fs::read_to_string(&full_path)
        .unwrap_or_else(|e| panic!("cannot read fixture {path}: {e}"));
    content
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(i, line)| {
            serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("line {} of {path} is not valid BenchmarkQuery: {e}\nContent: {line}", i + 1))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_regression_fixture() {
        let queries = load_benchmark("tests/fixtures/search/eval/regression.jsonl");
        assert_eq!(queries.len(), 5, "regression fixture must have 5 entries");
        assert!(!queries[0].query.is_empty());
        assert!(!queries[0].gold_urls.is_empty());
        assert!(!queries[0].baseline_results.is_empty());
    }

    #[test]
    fn loads_techdocs_fixture() {
        let queries = load_benchmark("tests/fixtures/search/eval/techdocs_bench.jsonl");
        assert_eq!(queries.len(), 5, "techdocs fixture must have 5 entries");
        for q in &queries {
            assert!(!q.gold_priority.is_empty(), "every entry needs at least one gold_priority URL");
        }
    }
}
