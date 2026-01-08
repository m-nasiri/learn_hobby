use chrono::{DateTime, Duration, Utc};

#[must_use]
pub fn format_datetime(value: &DateTime<Utc>) -> String {
    value.format("%b %-d \u{00b7} %-I:%M %p").to_string()
}

#[must_use]
pub fn format_relative_datetime(value: &DateTime<Utc>, now: &DateTime<Utc>) -> String {
    let value_day = value.date_naive();
    let now_day = now.date_naive();
    let time_label = value.format("%-I:%M %p").to_string();

    if value_day == now_day {
        return format!("Today \u{00b7} {time_label}");
    }
    if value_day == now_day - Duration::days(1) {
        return format!("Yesterday \u{00b7} {time_label}");
    }

    format_datetime(value)
}
