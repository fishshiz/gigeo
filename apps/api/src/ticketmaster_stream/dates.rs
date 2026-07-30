//! Splitting an arbitrary datetime range into per-calendar-day windows, so
//! Ticketmaster's `startDateTime`/`endDateTime` query params (which don't
//! tolerate huge ranges well) can be paged through a day at a time.

use chrono::{DateTime, Days, NaiveDate, TimeZone, Utc};

pub(super) fn date_windows(start: &str, end: &str) -> Result<Vec<(String, String)>, String> {
    let start_dt = DateTime::parse_from_rfc3339(start)
        .map_err(|e| format!("invalid start datetime: {e}"))?
        .with_timezone(&Utc);

    let end_dt = DateTime::parse_from_rfc3339(end)
        .map_err(|e| format!("invalid end datetime: {e}"))?
        .with_timezone(&Utc);

    if end_dt < start_dt {
        return Err("end must be >= start".to_string());
    }

    let mut out = Vec::new();
    let mut current: NaiveDate = start_dt.date_naive();
    let last: NaiveDate = end_dt.date_naive();

    while current <= last {
        let day_start = Utc.from_utc_datetime(&current.and_hms_opt(0, 0, 0).unwrap());

        let next_day = current
            .checked_add_days(Days::new(1))
            .ok_or_else(|| "date overflow".to_string())?;

        let day_end = Utc.from_utc_datetime(&next_day.and_hms_opt(0, 0, 0).unwrap());

        let window_start = if current == start_dt.date_naive() {
            start_dt
        } else {
            day_start
        };

        let window_end = if current == end_dt.date_naive() {
            end_dt
        } else {
            day_end
        };

        out.push((
            window_start.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            window_end.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        ));

        current = next_day;
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_windows_single_day_produces_one_window_clipped_to_input() {
        let windows = date_windows("2026-08-01T14:00:00Z", "2026-08-01T22:00:00Z").unwrap();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].0, "2026-08-01T14:00:00Z");
        assert_eq!(windows[0].1, "2026-08-01T22:00:00Z");
    }

    #[test]
    fn date_windows_splits_multi_day_range_by_calendar_day() {
        let windows = date_windows("2026-08-01T14:00:00Z", "2026-08-03T10:00:00Z").unwrap();
        assert_eq!(windows.len(), 3);

        // First window starts at the given start time, not midnight.
        assert_eq!(windows[0].0, "2026-08-01T14:00:00Z");
        assert_eq!(windows[0].1, "2026-08-02T00:00:00Z");

        // Middle window spans the full calendar day.
        assert_eq!(windows[1].0, "2026-08-02T00:00:00Z");
        assert_eq!(windows[1].1, "2026-08-03T00:00:00Z");

        // Last window ends at the given end time, not midnight.
        assert_eq!(windows[2].0, "2026-08-03T00:00:00Z");
        assert_eq!(windows[2].1, "2026-08-03T10:00:00Z");
    }

    #[test]
    fn date_windows_rejects_end_before_start() {
        let result = date_windows("2026-08-03T00:00:00Z", "2026-08-01T00:00:00Z");
        assert!(result.is_err());
    }

    #[test]
    fn date_windows_rejects_malformed_datetime() {
        let result = date_windows("not-a-date", "2026-08-01T00:00:00Z");
        assert!(result.is_err());
    }
}
