use chrono::{DateTime, Utc};

#[must_use]
pub fn format_datetime(value: &DateTime<Utc>) -> String {
    value.format("%b %-d \u{00b7} %-I:%M %p").to_string()
}
