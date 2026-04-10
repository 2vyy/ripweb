mod common;
use common::{eval_metrics::compute_metrics, search_eval::load_benchmark};
use ripweb::search::trace::QueryTrace;

fn baseline_traces(
    fixture_path: &str,
) -> (
    Vec<ripweb::search::eval_types::BenchmarkQuery>,
    Vec<QueryTrace>,
) {
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

// ── Phase 1: scored eval ──────────────────────────────────────────────────────

use ripweb::search::{SearchResult, eval_types::SearchResultRecord, pipeline::score_results};

fn to_search_result(r: &SearchResultRecord) -> SearchResult {
    SearchResult {
        url: r.url.clone(),
        title: r.title.clone(),
        snippet: r.snippet.clone(),
    }
}

fn scored_traces(
    fixture_path: &str,
) -> (
    Vec<ripweb::search::eval_types::BenchmarkQuery>,
    Vec<QueryTrace>,
) {
    let queries = load_benchmark(fixture_path);
    let traces: Vec<QueryTrace> = queries
        .iter()
        .map(|q| {
            // Convert baseline_results to SearchResult, run through pipeline.
            let raw: Vec<SearchResult> = q.baseline_results.iter().map(to_search_result).collect();
            let scored = score_results(raw, &q.query);
            // Build a trace from the scored, ranked output.
            let records: Vec<SearchResultRecord> = scored
                .iter()
                .map(|s| SearchResultRecord {
                    url: s.result.url.clone(),
                    title: s.result.title.clone(),
                    snippet: s.result.snippet.clone(),
                })
                .collect();
            QueryTrace::from_engine_results(&q.query, &records)
        })
        .collect();
    (queries, traces)
}

#[test]
fn phase1_metrics_regression() {
    let (queries, traces) = scored_traces("tests/fixtures/search/eval/regression.jsonl");
    let metrics = compute_metrics(&queries, &traces);
    insta::assert_json_snapshot!("phase1_regression_metrics", metrics);
}

#[test]
fn phase1_metrics_techdocs() {
    let (queries, traces) = scored_traces("tests/fixtures/search/eval/techdocs_bench.jsonl");
    let metrics = compute_metrics(&queries, &traces);
    insta::assert_json_snapshot!("phase1_techdocs_metrics", metrics);
}

#[test]
fn phase1_success_at_3_not_worse_than_baseline_regression() {
    let (bq, bt) = baseline_traces("tests/fixtures/search/eval/regression.jsonl");
    let (sq, st) = scored_traces("tests/fixtures/search/eval/regression.jsonl");
    let baseline = compute_metrics(&bq, &bt);
    let scored = compute_metrics(&sq, &st);
    assert!(
        scored.success_at_3 >= baseline.success_at_3,
        "Phase 1 scoring must not regress Success@3: baseline={}, scored={}",
        baseline.success_at_3,
        scored.success_at_3
    );
}

#[test]
fn phase1_success_at_3_not_worse_than_baseline_techdocs() {
    let (bq, bt) = baseline_traces("tests/fixtures/search/eval/techdocs_bench.jsonl");
    let (sq, st) = scored_traces("tests/fixtures/search/eval/techdocs_bench.jsonl");
    let baseline = compute_metrics(&bq, &bt);
    let scored = compute_metrics(&sq, &st);
    assert!(
        scored.success_at_3 >= baseline.success_at_3,
        "Phase 1 scoring must not regress Success@3: baseline={}, scored={}",
        baseline.success_at_3,
        scored.success_at_3
    );
}

// ── Phase 2: fan-out eval ─────────────────────────────────────────────────────

use ripweb::search::fusion::rrf_fuse;

fn fan_out_traces(
    fixture_path: &str,
) -> (
    Vec<ripweb::search::eval_types::BenchmarkQuery>,
    Vec<QueryTrace>,
) {
    let queries = load_benchmark(fixture_path);
    let traces: Vec<QueryTrace> = queries
        .iter()
        .map(|q| {
            // Build DDG results from baseline_results.
            let ddg_results: Vec<SearchResult> =
                q.baseline_results.iter().map(to_search_result).collect();

            // Build Marginalia results from supplemental_results["marginalia"].
            let marginalia_results: Vec<SearchResult> = q
                .supplemental_results
                .get("marginalia")
                .map(|recs| recs.iter().map(to_search_result).collect())
                .unwrap_or_default();

            // Fuse with RRF.
            let fused = rrf_fuse(&[("ddg", ddg_results), ("marginalia", marginalia_results)]);

            // Run through Phase 1 scoring.
            let scored = score_results(fused, &q.query);
            let records: Vec<SearchResultRecord> = scored
                .iter()
                .map(|s| SearchResultRecord {
                    url: s.result.url.clone(),
                    title: s.result.title.clone(),
                    snippet: s.result.snippet.clone(),
                })
                .collect();
            QueryTrace::from_engine_results(&q.query, &records)
        })
        .collect();
    (queries, traces)
}

#[test]
fn phase2_metrics_regression() {
    let (queries, traces) = fan_out_traces("tests/fixtures/search/eval/regression_fanout.jsonl");
    let metrics = compute_metrics(&queries, &traces);
    insta::assert_json_snapshot!("phase2_regression_metrics", metrics);
}

#[test]
fn phase2_metrics_techdocs() {
    let (queries, traces) = fan_out_traces("tests/fixtures/search/eval/techdocs_fanout.jsonl");
    let metrics = compute_metrics(&queries, &traces);
    insta::assert_json_snapshot!("phase2_techdocs_metrics", metrics);
}

#[test]
fn phase2_success_at_3_not_worse_than_phase1_regression() {
    let (bq, bt) = scored_traces("tests/fixtures/search/eval/regression.jsonl");
    let (fq, ft) = fan_out_traces("tests/fixtures/search/eval/regression_fanout.jsonl");
    let phase1 = compute_metrics(&bq, &bt);
    let phase2 = compute_metrics(&fq, &ft);
    assert!(
        phase2.success_at_3 >= phase1.success_at_3,
        "Phase 2 fan-out must not regress Success@3 vs Phase 1: phase1={}, phase2={}",
        phase1.success_at_3,
        phase2.success_at_3
    );
}

#[test]
fn phase2_success_at_3_not_worse_than_phase1_techdocs() {
    let (bq, bt) = scored_traces("tests/fixtures/search/eval/techdocs_bench.jsonl");
    let (fq, ft) = fan_out_traces("tests/fixtures/search/eval/techdocs_fanout.jsonl");
    let phase1 = compute_metrics(&bq, &bt);
    let phase2 = compute_metrics(&fq, &ft);
    assert!(
        phase2.success_at_3 >= phase1.success_at_3,
        "Phase 2 fan-out must not regress Success@3 vs Phase 1: phase1={}, phase2={}",
        phase1.success_at_3,
        phase2.success_at_3
    );
}
