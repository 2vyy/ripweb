use criterion::{Criterion, black_box, criterion_group, criterion_main};
use ripweb::extract::Extractor;
use ripweb::extract::web::WebExtractor;
use tiktoken_rs::cl100k_base;

// Using a small fake HTML string to benchmark formatting parsing overhead.
const SAMPLE_HTML: &[u8] = br#"
<html><body>
<main>
    <h1>The Cost of Tokens</h1>
    <p>Tokens are expensive. Optimization is key.</p>
    <ul>
        <li>First point</li>
        <li>Second point</li>
    </ul>
</main>
</body></html>
"#;

pub fn bench_token_efficiency(c: &mut Criterion) {
    let bpe = cl100k_base().unwrap();
    let mut group = c.benchmark_group("Token Efficiency");

    // The current token measurement doesn't have a direct verbosity flag in WebExtractor.extract
    // but we can benchmark the raw extraction vs the length.
    group.bench_function("web_extractor", |b| {
        b.iter(|| {
            let res = WebExtractor::extract(black_box(SAMPLE_HTML), Some("text/html")).unwrap();
            let tokens = bpe.encode_with_special_tokens(&res);
            black_box(tokens.len());
        });
    });

    group.finish();
}

criterion_group!(benches, bench_token_efficiency);
criterion_main!(benches);
