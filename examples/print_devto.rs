use ripweb::{
    extract::{web::WebExtractor, Extractor},
    minify::collapse,
};

fn main() {
    let bytes = std::fs::read("corpus/web/devto_article.html").unwrap();
    let extracted =
        WebExtractor::extract(&bytes, Some("text/html; charset=utf-8")).unwrap_or_default();
    let output = collapse(&extracted);
    println!("{}", output);
}
