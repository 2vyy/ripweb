#![allow(dead_code)]

use ripweb::search::eval_types::BenchmarkQuery;
use ripweb::search::trace::QueryTrace;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EvalMetrics {
    pub query_count: usize,
    pub success_at_1: f64,
    pub success_at_3: f64,
    pub success_at_5: f64,
    pub mrr: f64,
    pub ndcg_at_10: f64,
}

pub fn success_at_k(gold_urls: &[String], ranked: &[String], k: usize) -> bool {
    ranked
        .iter()
        .take(k)
        .any(|url| gold_urls.iter().any(|g| url_matches(url, g)))
}

pub fn reciprocal_rank(gold_urls: &[String], ranked: &[String]) -> f64 {
    ranked
        .iter()
        .enumerate()
        .find_map(|(i, url)| {
            if gold_urls.iter().any(|g| url_matches(url, g)) {
                Some(1.0 / (i + 1) as f64)
            } else {
                None
            }
        })
        .unwrap_or(0.0)
}

pub fn ndcg_at_k(
    gold_urls: &[String],
    gold_priority: &[String],
    ranked: &[String],
    k: usize,
) -> f64 {
    let dcg: f64 = ranked
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, url)| {
            let rel = relevance(url, gold_priority, gold_urls);
            rel / (i as f64 + 2.0).log2()
        })
        .sum();

    let mut ideal_rels: Vec<f64> = gold_priority
        .iter()
        .map(|_| 2.0_f64)
        .chain(
            gold_urls
                .iter()
                .filter(|g| !gold_priority.contains(g))
                .map(|_| 1.0_f64),
        )
        .collect();
    ideal_rels.sort_by(|a, b| b.partial_cmp(a).unwrap());

    let idcg: f64 = ideal_rels
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, &rel)| rel / (i as f64 + 2.0).log2())
        .sum();

    if idcg == 0.0 { 0.0 } else { dcg / idcg }
}

pub fn compute_metrics(queries: &[BenchmarkQuery], traces: &[QueryTrace]) -> EvalMetrics {
    assert_eq!(
        queries.len(),
        traces.len(),
        "queries and traces must be aligned"
    );
    let n = queries.len() as f64;
    let mut s1 = 0.0_f64;
    let mut s3 = 0.0_f64;
    let mut s5 = 0.0_f64;
    let mut mrr_sum = 0.0_f64;
    let mut ndcg_sum = 0.0_f64;

    for (q, t) in queries.iter().zip(traces.iter()) {
        if success_at_k(&q.gold_urls, &t.final_rank, 1) {
            s1 += 1.0;
        }
        if success_at_k(&q.gold_urls, &t.final_rank, 3) {
            s3 += 1.0;
        }
        if success_at_k(&q.gold_urls, &t.final_rank, 5) {
            s5 += 1.0;
        }
        mrr_sum += reciprocal_rank(&q.gold_urls, &t.final_rank);
        ndcg_sum += ndcg_at_k(&q.gold_urls, &q.gold_priority, &t.final_rank, 10);
    }

    let round3 = |v: f64| (v * 1000.0).round() / 1000.0;

    EvalMetrics {
        query_count: queries.len(),
        success_at_1: round3(s1 / n),
        success_at_3: round3(s3 / n),
        success_at_5: round3(s5 / n),
        mrr: round3(mrr_sum / n),
        ndcg_at_10: round3(ndcg_sum / n),
    }
}

fn url_matches(candidate: &str, gold: &str) -> bool {
    let c = candidate.trim_end_matches('/');
    let g = gold.trim_end_matches('/');
    c == g || c.starts_with(&format!("{g}/")) || c.starts_with(&format!("{g}?"))
}

fn relevance(url: &str, gold_priority: &[String], gold_urls: &[String]) -> f64 {
    if gold_priority.iter().any(|g| url_matches(url, g)) {
        2.0
    } else if gold_urls.iter().any(|g| url_matches(url, g)) {
        1.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ripweb::search::{eval_types::SearchResultRecord, trace::QueryTrace};

    fn s(v: &str) -> String {
        v.to_owned()
    }
    fn make_result(url: &str) -> SearchResultRecord {
        SearchResultRecord {
            url: url.to_owned(),
            title: "Title".to_owned(),
            snippet: None,
        }
    }

    #[test]
    fn url_matches_exact() {
        assert!(url_matches(
            "https://tokio.rs/tokio/tutorial",
            "https://tokio.rs/tokio/tutorial"
        ));
    }
    #[test]
    fn url_matches_trailing_slash_normalised() {
        assert!(url_matches(
            "https://tokio.rs/tokio/tutorial/",
            "https://tokio.rs/tokio/tutorial"
        ));
    }
    #[test]
    fn url_matches_subpath() {
        assert!(url_matches(
            "https://tokio.rs/tokio/tutorial/hello_tokio",
            "https://tokio.rs/tokio/tutorial"
        ));
    }
    #[test]
    fn url_matches_with_query_string() {
        assert!(url_matches(
            "https://tokio.rs/tokio/tutorial?foo=bar",
            "https://tokio.rs/tokio/tutorial"
        ));
    }
    #[test]
    fn url_does_not_match_different_host() {
        assert!(!url_matches(
            "https://other.rs/tokio/tutorial",
            "https://tokio.rs/tokio/tutorial"
        ));
    }
    #[test]
    fn success_at_1_when_gold_is_first() {
        assert!(success_at_k(
            &[s("https://a.example.com")],
            &[s("https://a.example.com"), s("https://b.example.com")],
            1
        ));
    }
    #[test]
    fn no_success_at_1_when_gold_is_second() {
        assert!(!success_at_k(
            &[s("https://a.example.com")],
            &[s("https://b.example.com"), s("https://a.example.com")],
            1
        ));
    }
    #[test]
    fn success_at_3_when_gold_is_third() {
        assert!(success_at_k(
            &[s("https://a.example.com")],
            &[
                s("https://b.example.com"),
                s("https://c.example.com"),
                s("https://a.example.com")
            ],
            3
        ));
    }
    #[test]
    fn no_success_when_gold_absent() {
        assert!(!success_at_k(
            &[s("https://a.example.com")],
            &[s("https://b.example.com"), s("https://c.example.com")],
            5
        ));
    }
    #[test]
    fn rr_is_1_when_gold_is_rank_1() {
        assert_eq!(
            reciprocal_rank(
                &[s("https://a.example.com")],
                &[s("https://a.example.com"), s("https://b.example.com")]
            ),
            1.0
        );
    }
    #[test]
    fn rr_is_half_when_gold_is_rank_2() {
        assert_eq!(
            reciprocal_rank(
                &[s("https://a.example.com")],
                &[s("https://b.example.com"), s("https://a.example.com")]
            ),
            0.5
        );
    }
    #[test]
    fn rr_is_zero_when_gold_absent() {
        assert_eq!(
            reciprocal_rank(&[s("https://a.example.com")], &[s("https://b.example.com")]),
            0.0
        );
    }
    #[test]
    fn ndcg_is_1_when_priority_gold_is_rank_1() {
        let gold = vec![s("https://a.example.com")];
        let prio = vec![s("https://a.example.com")];
        let ranked = vec![s("https://a.example.com"), s("https://b.example.com")];
        let score = ndcg_at_k(&gold, &prio, &ranked, 10);
        assert!((score - 1.0).abs() < 1e-9, "expected 1.0, got {score}");
    }
    #[test]
    fn ndcg_is_less_than_1_when_priority_gold_not_at_rank_1() {
        let gold = vec![s("https://a.example.com")];
        let prio = vec![s("https://a.example.com")];
        let ranked = vec![s("https://b.example.com"), s("https://a.example.com")];
        let score = ndcg_at_k(&gold, &prio, &ranked, 10);
        assert!(
            score < 1.0 && score > 0.0,
            "expected 0<score<1, got {score}"
        );
    }
    #[test]
    fn ndcg_is_zero_when_gold_absent() {
        let gold = vec![s("https://a.example.com")];
        let prio = vec![s("https://a.example.com")];
        let ranked = vec![s("https://b.example.com"), s("https://c.example.com")];
        assert_eq!(ndcg_at_k(&gold, &prio, &ranked, 10), 0.0);
    }
    #[test]
    fn compute_metrics_perfect_baseline() {
        let gold_url = "https://a.example.com";
        let bq = BenchmarkQuery {
            query: "test".to_owned(),
            intent: "official_docs".to_owned(),
            gold_urls: vec![s(gold_url)],
            gold_priority: vec![s(gold_url)],
            negative_urls: vec![],
            baseline_results: vec![SearchResultRecord {
                url: s(gold_url),
                title: s("A"),
                snippet: None,
            }],
            supplemental_results: std::collections::HashMap::new(),
        };
        let trace = QueryTrace::from_engine_results("test", &bq.baseline_results);
        let metrics = compute_metrics(&[bq], &[trace]);
        assert_eq!(metrics.success_at_1, 1.0);
        assert_eq!(metrics.success_at_3, 1.0);
        assert_eq!(metrics.mrr, 1.0);
        assert_eq!(metrics.ndcg_at_10, 1.0);
    }
    #[test]
    fn compute_metrics_worst_baseline() {
        let bq = BenchmarkQuery {
            query: "test".to_owned(),
            intent: "general_technical".to_owned(),
            gold_urls: vec![s("https://a.example.com")],
            gold_priority: vec![s("https://a.example.com")],
            negative_urls: vec![],
            baseline_results: vec![make_result("https://b.example.com")],
            supplemental_results: std::collections::HashMap::new(),
        };
        let trace = QueryTrace::from_engine_results("test", &bq.baseline_results);
        let metrics = compute_metrics(&[bq], &[trace]);
        assert_eq!(metrics.success_at_1, 0.0);
        assert_eq!(metrics.success_at_5, 0.0);
        assert_eq!(metrics.mrr, 0.0);
        assert_eq!(metrics.ndcg_at_10, 0.0);
    }
}
