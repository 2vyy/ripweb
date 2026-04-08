use ripweb::{
    corpus::{GeneratedMode, WEB_FIXTURES},
    extract::{web::WebExtractor, Extractor},
    minify::collapse,
};
use std::fs;

fn main() {
    println!("Syncing generated outputs from fixture corpus...");

    for fixture in WEB_FIXTURES {
        if !fixture.generate_expected_outputs {
            continue;
        }

        let bytes = fs::read(fixture.html_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", fixture.html_path));

        let markdown =
            WebExtractor::extract(&bytes, Some("text/html; charset=utf-8")).unwrap_or_default();
        let aggressive = collapse(&markdown);

        let markdown_path = fixture.generated_output_path(GeneratedMode::Markdown);
        let aggressive_path = fixture.generated_output_path(GeneratedMode::Aggressive);

        if let Some(parent) = std::path::Path::new(&markdown_path).parent() {
            fs::create_dir_all(parent).expect("create markdown output dir");
        }
        if let Some(parent) = std::path::Path::new(&aggressive_path).parent() {
            fs::create_dir_all(parent).expect("create aggressive output dir");
        }

        fs::write(&markdown_path, markdown).expect("write markdown output");
        fs::write(&aggressive_path, aggressive).expect("write aggressive output");

        println!(
            "  {} -> {}, {}",
            fixture.name,
            markdown_path,
            aggressive_path
        );
    }

    println!("Done.");
}
