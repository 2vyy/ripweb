mod common;
use common::{
    eval_metrics::compute_metrics,
    search_eval::load_benchmark,
};
use ripweb::search::trace::QueryTrace;

fn baseline_traces(fixture_path: &str) -> (Vec<ripweb::search::eval_types::BenchmarkQuery>, Vec<QueryTrace>) {
    let queries = load_benchmark(fixture_path);
    let traces: Vec<QueryTrace> = queries
        .iter()
        .map(|q| QueryTrace::from_engine_results(&q.query, &q.baseline_results))
        .collect();
    (queries, traces)
}

#[test]
fn baseline_metrics_regression() {
    let (queries, traces) = baseline_traces("tests/fixtures/search/eval/regression.jsonl");
    let metrics = compute_metrics(&queries, &traces);
    insta::assert_json_snapshot!("baseline_regression_metrics", metrics);
}

#[test]
fn baseline_traces_regression() {
    let (queries, traces) = baseline_traces("tests/fixtures/search/eval/regression.jsonl");
    let summary: Vec<serde_json::Value> = queries
        .iter()
        .zip(traces.iter())
        .map(|(q, t)| {
            serde_json::json!({
                "query": q.query,
                "intent": q.intent,
                "top3": t.final_rank.iter().take(3).collect::<Vec<_>>(),
                "gold_urls": q.gold_urls,
            })
        })
        .collect();
    insta::assert_json_snapshot!("baseline_regression_traces", summary);
}

#[test]
fn baseline_metrics_techdocs() {
    let (queries, traces) = baseline_traces("tests/fixtures/search/eval/techdocs_bench.jsonl");
    let metrics = compute_metrics(&queries, &traces);
    insta::assert_json_snapshot!("baseline_techdocs_metrics", metrics);
}

#[test]
fn baseline_traces_techdocs() {
    let (queries, traces) = baseline_traces("tests/fixtures/search/eval/techdocs_bench.jsonl");
    let summary: Vec<serde_json::Value> = queries
        .iter()
        .zip(traces.iter())
        .map(|(q, t)| {
            serde_json::json!({
                "query": q.query,
                "intent": q.intent,
                "top3": t.final_rank.iter().take(3).collect::<Vec<_>>(),
                "gold_urls": q.gold_urls,
            })
        })
        .collect();
    insta::assert_json_snapshot!("baseline_techdocs_traces", summary);
}
