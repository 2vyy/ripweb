use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum WikidataError {
    #[error("SPARQL query error: {0}")]
    Query(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("response parse error: {0}")]
    Parse(String),
}

#[derive(Debug, Deserialize)]
struct SparqlResponse {
    head: SparqlHead,
    results: SparqlResults,
}

#[derive(Debug, Deserialize)]
struct SparqlHead {
    vars: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SparqlResults {
    bindings: Vec<HashMap<String, SparqlBinding>>,
}

#[derive(Debug, Deserialize)]
struct SparqlBinding {
    value: String,
}

pub async fn execute(query: &str, client: &rquest::Client) -> Result<String, WikidataError> {
    let response = client
        .get("https://query.wikidata.org/sparql")
        .query(&[("format", "json"), ("query", query)])
        .header("accept", "application/sparql-results+json")
        .send()
        .await
        .map_err(|e| WikidataError::Network(e.to_string()))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| WikidataError::Network(e.to_string()))?;

    if status.as_u16() == 400 {
        return Err(WikidataError::Query(body));
    }
    if !status.is_success() {
        return Err(WikidataError::Network(format!(
            "Wikidata returned HTTP {}",
            status.as_u16()
        )));
    }

    render_markdown_table(&body)
}

pub fn render_markdown_table(body: &str) -> Result<String, WikidataError> {
    let parsed: SparqlResponse =
        serde_json::from_str(body).map_err(|e| WikidataError::Parse(e.to_string()))?;
    Ok(to_markdown_table(parsed))
}

fn to_markdown_table(response: SparqlResponse) -> String {
    if response.head.vars.is_empty() {
        return String::from("| result |\n| --- |\n| No columns returned |");
    }

    let headers = response
        .head
        .vars
        .iter()
        .map(|h| escape_cell(h))
        .collect::<Vec<_>>();
    let mut out = String::new();

    out.push('|');
    out.push(' ');
    out.push_str(&headers.join(" | "));
    out.push_str(" |\n| ");
    out.push_str(&vec!["---"; headers.len()].join(" | "));
    out.push_str(" |\n");

    if response.results.bindings.is_empty() {
        let mut empty_row = vec![String::new(); headers.len()];
        empty_row[0] = "No rows returned".to_owned();
        out.push('|');
        out.push(' ');
        out.push_str(&empty_row.join(" | "));
        out.push_str(" |\n");
        return out;
    }

    for row in response.results.bindings {
        let cells = response
            .head
            .vars
            .iter()
            .map(|col| {
                row.get(col)
                    .map(|binding| escape_cell(&binding.value))
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();
        out.push('|');
        out.push(' ');
        out.push_str(&cells.join(" | "));
        out.push_str(" |\n");
    }

    out.trim_end().to_owned()
}

fn escape_cell(input: &str) -> String {
    input
        .replace('|', "\\|")
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::escape_cell;

    #[test]
    fn escapes_markdown_table_breakers() {
        let escaped = escape_cell("A|B\nC");
        assert_eq!(escaped, "A\\|B C");
    }
}
