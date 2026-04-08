use ripweb::{
    corpus::WEB_FIXTURES,
    extract::{web::WebExtractor, Extractor},
    minify::collapse,
};
use std::fs;

fn main() {
    for fixture in WEB_FIXTURES {
        let bytes = fs::read(fixture.html_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", fixture.html_path));
        let markdown =
            WebExtractor::extract(&bytes, Some("text/html; charset=utf-8")).unwrap_or_default();
        let aggressive = collapse(&markdown);

        println!("\n========== {} ==========\n", fixture.name);
        println!("-- markdown --\n{markdown}\n");
        println!("-- aggressive --\n{aggressive}\n");
    }
}
