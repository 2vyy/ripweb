use ripweb::research::wikidata::render_markdown_table;

#[test]
fn wikidata_render_markdown_table_uses_fixture_schema() {
    let fixture = include_str!("research/wikidata_fixtures/sparql_response.json");
    let table = render_markdown_table(fixture).unwrap();

    assert!(table.contains("| item | itemLabel |"));
    assert!(table.contains("http://www.wikidata.org/entity/Q42"));
    assert!(table.contains("Douglas Adams"));
}

#[test]
fn wikidata_render_markdown_table_rejects_invalid_json() {
    let result = render_markdown_table("{invalid");
    assert!(result.is_err());
}
