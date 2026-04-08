use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ripweb::{
    extract::{Extractor, web::WebExtractor},
    minify::collapse,
};
use std::hint::black_box;

/// Names of the corpus files that benchmarks will load.
/// Files are read at runtime from `corpus/web/` so that:
///   - the directory can be gitignored (live scraped pages, often >1 MB each)
///   - the bench compiles even when the corpus is absent (missing files are skipped)
///   - re-seeding the corpus never requires a recompile
const SAMPLE_NAMES: &[&str] = &[
    "arxiv_1706.03762.html",
    "aws_ai_ml.html",
    "base64_img_bomb.txt",
    "docs_factory_ai.html",
    "docs_rs_axum.html",
    "github_tokio_1879.html",
    "hn_47326101.html",
    "hn_47340079.html",
    "legacy_img_bomb.html",
    "react_dev_usestate.html",
    "reddit_wallstreetbets_spacex.html",
    "stackoverflow_11227809.html",
    "theverge_gemini_google_maps.html",
];

/// Discover and load all corpus files that exist on disk.
/// Returns `(display_name, bytes)` pairs; silently skips missing files.
fn load_samples() -> Vec<(String, Vec<u8>)> {
    let corpus_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/web");

    let mut loaded = Vec::new();
    for file_name in SAMPLE_NAMES {
        let path = corpus_dir.join(file_name);
        match std::fs::read(&path) {
            Ok(bytes) => {
                // Strip extension for a cleaner benchmark ID.
                let name = std::path::Path::new(file_name)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| file_name.to_string());
                loaded.push((name, bytes));
            }
            Err(_) => {
                // Corpus file absent — skip without failing.
                // Run `cargo run -p ripweb --example seed_corpus` to populate.
                eprintln!("bench: skipping missing corpus file: {}", path.display());
            }
        }
    }

    if loaded.is_empty() {
        eprintln!(
            "bench: no corpus files found in {}. \
             Populate corpus/web/ with real HTML pages to get meaningful results.",
            corpus_dir.display()
        );
    }

    loaded
}

/// Benchmark the extraction step (HTML → Markdown).
fn bench_extract(c: &mut Criterion) {
    let samples = load_samples();
    if samples.is_empty() {
        return;
    }

    let mut group = c.benchmark_group("extract");
    for (name, bytes) in &samples {
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

/// Benchmark the minification / collapse step (Markdown → minimised).
fn bench_collapse(c: &mut Criterion) {
    let samples = load_samples();
    if samples.is_empty() {
        return;
    }

    // Pre-extract once so we measure only the collapse step.
    let extracted: Vec<(String, String)> = samples
        .iter()
        .filter_map(|(name, bytes)| {
            let text = WebExtractor::extract(bytes, Some("text/html; charset=utf-8")).ok()?;
            Some((name.clone(), text))
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
    let samples = load_samples();
    if samples.is_empty() {
        return;
    }

    let mut group = c.benchmark_group("pipeline");
    for (name, bytes) in &samples {
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
