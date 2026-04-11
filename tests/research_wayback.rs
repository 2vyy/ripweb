use ripweb::research::wayback::{WaybackError, parse_available_response, validate_date};

#[test]
fn wayback_validate_date_accepts_iso_date() {
    let normalized = validate_date("2024-01-01");
    assert!(matches!(normalized.as_deref(), Ok("2024-01-01")));
}

#[test]
fn wayback_validate_date_rejects_invalid_date() {
    assert!(matches!(
        validate_date("2024-13-01"),
        Err(WaybackError::InvalidDate)
    ));
}

#[test]
fn wayback_parse_available_response_reads_fixture() {
    let fixture = include_str!("research/wayback_fixtures/cdx_response.json");
    let snapshot = parse_available_response(fixture, "2024-01-01").unwrap();

    assert_eq!(snapshot.requested_date, "2024-01-01");
    assert_eq!(snapshot.snapshot_date, "2024-01-01");
    assert!(
        snapshot
            .snapshot_url
            .contains("web.archive.org/web/20240101")
    );
}
