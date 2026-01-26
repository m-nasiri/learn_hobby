pub(super) fn parse_positive_u32(value: &str) -> Option<u32> {
    let value = value.trim();
    let parsed = value.parse::<u32>().ok()?;
    if parsed == 0 {
        return None;
    }
    Some(parsed)
}

pub(super) fn parse_lapse_interval_secs(value: &str) -> Option<u32> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = trimmed.to_ascii_lowercase();
    let mut split_at = normalized.len();
    for (idx, ch) in normalized.char_indices() {
        if !ch.is_ascii_digit() {
            split_at = idx;
            break;
        }
    }
    let (number_part, suffix) = normalized.split_at(split_at);
    let amount = number_part.trim().parse::<u32>().ok()?;
    if amount == 0 {
        return None;
    }
    let unit = suffix.trim();
    match unit {
        "" | "d" | "day" | "days" => amount.checked_mul(86_400),
        "h" | "hr" | "hrs" | "hour" | "hours" => amount.checked_mul(3600),
        "m" | "min" | "mins" | "minute" | "minutes" => amount.checked_mul(60),
        "s" | "sec" | "secs" | "second" | "seconds" => Some(amount),
        _ => None,
    }
}

pub(super) fn format_lapse_interval(secs: u32) -> String {
    if secs.is_multiple_of(86_400) {
        format!("{}d", secs / 86_400)
    } else if secs.is_multiple_of(3600) {
        format!("{}h", secs / 3600)
    } else if secs.is_multiple_of(60) {
        format!("{}m", secs / 60)
    } else {
        format!("{secs}s")
    }
}

pub(super) fn parse_timer_secs(value: &str) -> Option<u32> {
    let value = value.trim();
    let parsed = value.parse::<u32>().ok()?;
    if !(5..=600).contains(&parsed) {
        return None;
    }
    Some(parsed)
}

pub(super) fn parse_retention(value: &str) -> Option<f32> {
    let trimmed = value.trim();
    let parsed = trimmed.parse::<f32>().ok()?;
    if !parsed.is_finite() || parsed <= 0.0 || parsed > 1.0 {
        return None;
    }
    Some(parsed)
}

pub(super) fn format_retention(value: f32) -> String {
    format!("{value:.2}")
}

pub(super) fn normalize_description(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
