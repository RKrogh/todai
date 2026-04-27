use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};

/// Parse a user-supplied datetime string into UTC.
///
/// Accepted forms:
/// - `today`, `tomorrow`           → 00:00 local time on that date
/// - `this-week`, `next-week`      → end of that week (Sunday 23:59 local)
/// - `YYYY-MM-DD`                  → 00:00 local time on that date
/// - `YYYY-MM-DDTHH:MM`            → that local time
/// - `YYYY-MM-DDTHH:MM:SSZ`        → already UTC
pub fn parse_user_datetime(input: &str) -> Result<DateTime<Utc>> {
    let s = input.trim();
    if s.is_empty() {
        bail!("empty datetime string");
    }
    match s.to_lowercase().as_str() {
        "today" => return Ok(local_midnight(0)),
        "tomorrow" => return Ok(local_midnight(1)),
        "this-week" => return Ok(end_of_week_local(0)),
        "next-week" => return Ok(end_of_week_local(7)),
        _ => {}
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return naive_local_to_utc(naive);
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M") {
        return naive_local_to_utc(naive);
    }
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let naive = date.and_hms_opt(0, 0, 0).unwrap();
        return naive_local_to_utc(naive);
    }
    Err(anyhow!("could not parse datetime '{}'. Try YYYY-MM-DD or 'today'", s))
}

fn naive_local_to_utc(naive: NaiveDateTime) -> Result<DateTime<Utc>> {
    match Local.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => Ok(dt.with_timezone(&Utc)),
        chrono::LocalResult::Ambiguous(early, _) => Ok(early.with_timezone(&Utc)),
        chrono::LocalResult::None => Err(anyhow!("no valid local time for '{naive}' (DST gap?)")),
    }
}

fn local_midnight(days_offset: i64) -> DateTime<Utc> {
    let date = Local::now().date_naive() + Duration::days(days_offset);
    let naive = date.and_hms_opt(0, 0, 0).unwrap();
    Local
        .from_local_datetime(&naive)
        .single()
        .unwrap_or_else(|| Local.from_local_datetime(&naive).earliest().unwrap())
        .with_timezone(&Utc)
}

fn end_of_week_local(days_offset: i64) -> DateTime<Utc> {
    let today = Local::now().date_naive() + Duration::days(days_offset);
    let weekday = today.weekday().num_days_from_monday() as i64;
    let sunday = today + Duration::days(6 - weekday);
    let naive = sunday.and_hms_opt(23, 59, 59).unwrap();
    Local
        .from_local_datetime(&naive)
        .single()
        .unwrap_or_else(|| Local.from_local_datetime(&naive).earliest().unwrap())
        .with_timezone(&Utc)
}

/// Format a UTC datetime as local time with timezone label.
pub fn format_local(dt: DateTime<Utc>) -> String {
    let local = dt.with_timezone(&Local);
    local.format("%Y-%m-%d %H:%M %Z").to_string()
}

pub fn format_local_short(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string()
}

pub fn is_today_local(dt: DateTime<Utc>) -> bool {
    dt.with_timezone(&Local).date_naive() == Local::now().date_naive()
}

pub fn is_overdue(dt: DateTime<Utc>) -> bool {
    dt < Utc::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_iso_with_tz() {
        let dt = parse_user_datetime("2026-05-16T14:00:00Z").unwrap();
        assert_eq!(dt.to_rfc3339(), "2026-05-16T14:00:00+00:00");
    }

    #[test]
    fn parses_date_only() {
        let dt = parse_user_datetime("2026-05-16").unwrap();
        // Should be local midnight of that date — exact UTC depends on TZ but date should match locally
        assert_eq!(dt.with_timezone(&Local).date_naive().to_string(), "2026-05-16");
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_user_datetime("not a date").is_err());
    }
}
