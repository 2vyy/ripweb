use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct ResolvedSnapshot {
    pub snapshot_url: String,
    pub requested_date: String,
    pub snapshot_date: String,
}

#[derive(Debug, thiserror::Error)]
pub enum WaybackError {
    #[error("invalid --as-of date, expected YYYY-MM-DD")]
    InvalidDate,
    #[error("network error: {0}")]
    Network(String),
    #[error("response parse error: {0}")]
    Parse(String),
    #[error("no archived snapshot found for requested date")]
    NotFound,
}

#[derive(Debug, Deserialize)]
struct WaybackResponse {
    archived_snapshots: ArchivedSnapshots,
}

#[derive(Debug, Deserialize)]
struct ArchivedSnapshots {
    closest: Option<ClosestSnapshot>,
}

#[derive(Debug, Deserialize)]
struct ClosestSnapshot {
    available: bool,
    status: String,
    url: String,
    timestamp: String,
}

pub fn validate_date(raw: &str) -> Result<String, WaybackError> {
    let parts: Vec<&str> = raw.split('-').collect();
    if parts.len() != 3 {
        return Err(WaybackError::InvalidDate);
    }

    let (year, month, day) = (
        parts[0]
            .parse::<u32>()
            .map_err(|_| WaybackError::InvalidDate)?,
        parts[1]
            .parse::<u32>()
            .map_err(|_| WaybackError::InvalidDate)?,
        parts[2]
            .parse::<u32>()
            .map_err(|_| WaybackError::InvalidDate)?,
    );

    if parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return Err(WaybackError::InvalidDate);
    }

    if month == 0 || month > 12 {
        return Err(WaybackError::InvalidDate);
    }

    let max_day = days_in_month(year, month);
    if day == 0 || day > max_day {
        return Err(WaybackError::InvalidDate);
    }

    Ok(raw.to_owned())
}

pub async fn resolve_snapshot(
    original_url: &str,
    as_of: &str,
    client: &rquest::Client,
) -> Result<ResolvedSnapshot, WaybackError> {
    let normalized_date = validate_date(as_of)?;
    let ts = normalized_date.replace('-', "") + "120000";

    let response = client
        .get("https://archive.org/wayback/available")
        .query(&[("url", original_url), ("timestamp", ts.as_str())])
        .send()
        .await
        .map_err(|e| WaybackError::Network(e.to_string()))?;

    if !response.status().is_success() {
        return Err(WaybackError::Network(format!(
            "Wayback returned HTTP {}",
            response.status().as_u16()
        )));
    }

    let body = response
        .text()
        .await
        .map_err(|e| WaybackError::Network(e.to_string()))?;
    parse_available_response(&body, &normalized_date)
}

pub fn parse_available_response(
    body: &str,
    requested_date: &str,
) -> Result<ResolvedSnapshot, WaybackError> {
    let parsed: WaybackResponse =
        serde_json::from_str(body).map_err(|e| WaybackError::Parse(e.to_string()))?;

    let Some(closest) = parsed.archived_snapshots.closest else {
        return Err(WaybackError::NotFound);
    };
    if !closest.available || !closest.status.starts_with('2') {
        return Err(WaybackError::NotFound);
    }

    Ok(ResolvedSnapshot {
        snapshot_url: closest.url,
        requested_date: requested_date.to_owned(),
        snapshot_date: format_snapshot_date(&closest.timestamp),
    })
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn format_snapshot_date(timestamp: &str) -> String {
    if timestamp.len() >= 8 {
        let y = &timestamp[0..4];
        let m = &timestamp[4..6];
        let d = &timestamp[6..8];
        return format!("{y}-{m}-{d}");
    }
    timestamp.to_owned()
}

#[cfg(test)]
mod tests {
    use super::{WaybackError, validate_date};

    #[test]
    fn accepts_valid_yyyy_mm_dd() {
        assert!(validate_date("2024-02-29").is_ok());
    }

    #[test]
    fn rejects_bad_dates() {
        assert!(matches!(
            validate_date("2024-02-31"),
            Err(WaybackError::InvalidDate)
        ));
        assert!(matches!(
            validate_date("24-2-1"),
            Err(WaybackError::InvalidDate)
        ));
    }
}
