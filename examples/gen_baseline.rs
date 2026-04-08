use ripweb::{
    extract::{web::WebExtractor, Extractor},
    minify::collapse,
};

fn main() {
    let samples = [
        (
            "paulgraham_essay",
            "corpus/web/paulgraham_essay.html",
        ),
        (
            "mdnwebdocs_fetch",
            "corpus/web/mdnwebdocs_fetch.html",
        ),
        ("rustblog_post", "corpus/web/rustblog_post.html"),
        ("devto_article", "corpus/web/devto_article.html"),
    ];

    for (name, path) in samples {
        let bytes = std::fs::read(path).expect(&format!("Failed to read {}", path));
        let extracted =
            WebExtractor::extract(&bytes, Some("text/html; charset=utf-8")).unwrap_or_default();
        let collapsed = collapse(&extracted);
        println!("\n========== {} ==========\n", name);
        println!("{}", collapsed);
    }
}
