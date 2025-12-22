use chrono::{DateTime, Utc};

#[must_use]
pub fn format_datetime(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}
