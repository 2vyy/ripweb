use ripweb::{
    extract::{web::WebExtractor, Extractor},
    minify::collapse,
};

#[test]
fn markdown_mode_preserves_source_structure() {
    let html = br#"
    <html><body>
      <main>
        <h1>Guide Title</h1>
        <p>Intro paragraph with a <a href="https://example.com/docs?utm_source=test&id=42">useful link</a>.</p>
        <h2>Steps</h2>
        <ol>
          <li>Install the tool</li>
          <li>Run the command</li>
        </ol>
        <pre><code>fn main() {
    println!("hi");
}</code></pre>
      </main>
    </body></html>
    "#;

    let result = WebExtractor::extract(html, Some("text/html; charset=utf-8")).unwrap();

    assert!(result.contains("# Guide Title"), "missing heading: {result}");
    assert!(result.contains("## Steps"), "missing nested heading: {result}");
    assert!(
        result.contains("[useful link](https://example.com/docs?id=42)"),
        "missing normalized markdown link: {result}"
    );
    assert!(result.contains("1. Install the tool"), "missing ordered list: {result}");
    assert!(result.contains("```"), "missing code fence: {result}");
    assert!(result.contains("\n\nIntro paragraph"), "missing paragraph separation: {result}");
}

#[test]
fn aggressive_mode_preserves_paragraphs_and_code_fences() {
    let markdown = "# Title\n\nFirst paragraph.\n\nSecond paragraph.\n\n```\nfn main() {\n    println!(\"hi\");\n}\n```\n";
    let result = collapse(markdown);

    assert!(result.contains("# Title"));
    assert!(result.contains("First paragraph.\n\nSecond paragraph."));
    assert!(result.contains("```\nfn main() {\n    println!(\"hi\");\n}\n```"));
    assert!(!result.contains("\n\n\n"), "aggressive mode left excessive blank lines: {result}");
}

#[test]
fn aggressive_mode_keeps_markdown_links_intact() {
    let markdown = "See [Fetch API](https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API?utm_source=test&id=42) for details.";
    let result = collapse(markdown);

    assert!(result.contains("[Fetch API]("), "link label lost: {result}");
    assert!(result.contains("id=42"), "meaningful URL component lost: {result}");
}
