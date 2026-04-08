use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ripweb::{
    extract::{web::WebExtractor, Extractor},
    minify::collapse,
};
use std::hint::black_box;

/// All HTML/text fixtures in the shared corpus that we benchmark.
const SAMPLES: &[(&str, &[u8])] = &[
    (
        "arxiv_1706.03762",
        include_bytes!("../corpus/web/arxiv_1706.03762.html"),
    ),
    ("aws_ai_ml", include_bytes!("../corpus/web/aws_ai_ml.html")),
    (
        "base64_img_bomb",
        include_bytes!("../corpus/web/base64_img_bomb.txt"),
    ),
    (
        "docs_factory_ai",
        include_bytes!("../corpus/web/docs_factory_ai.html"),
    ),
    (
        "docs_rs_axum",
        include_bytes!("../corpus/web/docs_rs_axum.html"),
    ),
    (
        "github_tokio_1879",
        include_bytes!("../corpus/web/github_tokio_1879.html"),
    ),
    (
        "hn_47326101",
        include_bytes!("../corpus/web/hn_47326101.html"),
    ),
    (
        "hn_47340079",
        include_bytes!("../corpus/web/hn_47340079.html"),
    ),
    (
        "legacy_img_bomb",
        include_bytes!("../corpus/web/legacy_img_bomb.html"),
    ),
    (
        "react_dev_usestate",
        include_bytes!("../corpus/web/react_dev_usestate.html"),
    ),
    (
        "reddit_wallstreetbets_spacex",
        include_bytes!("../corpus/web/reddit_wallstreetbets_spacex.html"),
    ),
    (
        "stackoverflow_11227809",
        include_bytes!("../corpus/web/stackoverflow_11227809.html"),
    ),
    (
        "theverge_gemini",
        include_bytes!("../corpus/web/theverge_gemini_google_maps.html"),
    ),
];

/// Benchmark the extraction step (HTML → plain text).
fn bench_extract(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract");
    for (name, bytes) in SAMPLES {
        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), bytes, |b, bytes| {
            b.iter(|| {
                let result = black_box(WebExtractor::extract(
                    black_box(bytes),
                    Some("text/html; charset=utf-8"),
                ));
                black_box(result.unwrap_or_default())
            });
        });
    }
    group.finish();
}

/// Benchmark the minification / collapse step (plain text → minimised).
fn bench_collapse(c: &mut Criterion) {
    // Pre-extract once so we measure only the collapse step.
    let extracted: Vec<(&str, String)> = SAMPLES
        .iter()
        .filter_map(|(name, bytes)| {
            let text = WebExtractor::extract(bytes, Some("text/html; charset=utf-8")).ok()?;
            Some((*name, text))
        })
        .collect();

    let mut group = c.benchmark_group("collapse");
    for (name, text) in &extracted {
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            text.as_str(),
            |b, text| {
                b.iter(|| black_box(collapse(black_box(text))));
            },
        );
    }
    group.finish();
}

/// Benchmark the full pipeline: extract + collapse.
fn bench_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline");
    for (name, bytes) in SAMPLES {
        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), bytes, |b, bytes| {
            b.iter(|| {
                let text =
                    WebExtractor::extract(black_box(bytes), Some("text/html; charset=utf-8"))
                        .unwrap_or_default();
                black_box(collapse(&text))
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_extract, bench_collapse, bench_pipeline);
criterion_main!(benches);
