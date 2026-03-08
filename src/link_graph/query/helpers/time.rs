use chrono::{DateTime, NaiveDate, TimeZone, Utc};

pub(in crate::link_graph::query) fn parse_timestamp(raw: &str) -> Option<i64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(epoch) = trimmed.parse::<i64>() {
        return Some(epoch);
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(dt.timestamp());
    }
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return date
            .and_hms_opt(0, 0, 0)
            .map(|naive| Utc.from_utc_datetime(&naive).timestamp());
    }
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y/%m/%d") {
        return date
            .and_hms_opt(0, 0, 0)
            .map(|naive| Utc.from_utc_datetime(&naive).timestamp());
    }
    None
}

pub(in crate::link_graph::query) fn parse_time_filter(
    token: &str,
    created_after: &mut Option<i64>,
    created_before: &mut Option<i64>,
    modified_after: &mut Option<i64>,
    modified_before: &mut Option<i64>,
) -> bool {
    let lower = token.trim().to_lowercase();
    let rules = [
        ("created>=", "created_after"),
        ("created<=", "created_before"),
        ("created>", "created_after"),
        ("created<", "created_before"),
        ("modified>=", "modified_after"),
        ("modified<=", "modified_before"),
        ("modified>", "modified_after"),
        ("modified<", "modified_before"),
        ("updated>=", "modified_after"),
        ("updated<=", "modified_before"),
        ("updated>", "modified_after"),
        ("updated<", "modified_before"),
    ];
    for (prefix, slot) in rules {
        if !lower.starts_with(prefix) {
            continue;
        }
        let value = token[prefix.len()..].trim().trim_start_matches(':');
        let Some(parsed) = parse_timestamp(value) else {
            return false;
        };
        match slot {
            "created_after" => *created_after = Some(parsed),
            "created_before" => *created_before = Some(parsed),
            "modified_after" => *modified_after = Some(parsed),
            "modified_before" => *modified_before = Some(parsed),
            _ => {}
        }
        return true;
    }
    false
}
