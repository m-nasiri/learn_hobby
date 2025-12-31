use learn_core::model::TagName;
use services::CardListSort;

pub fn sort_value(sort: CardListSort) -> &'static str {
    match sort {
        CardListSort::Created => "created",
        CardListSort::Alpha => "alpha",
        _ => "recent",
    }
}

pub fn sort_from_value(value: &str) -> CardListSort {
    match value {
        "created" => CardListSort::Created,
        "alpha" => CardListSort::Alpha,
        _ => CardListSort::Recent,
    }
}

pub fn tag_filter_key(tags: &[String]) -> String {
    let mut items = tags.to_vec();
    items.sort();
    items.join("|")
}

pub fn tag_names_from_strings(tags: &[String]) -> Vec<TagName> {
    tags.iter().filter_map(|tag| TagName::new(tag.clone()).ok()).collect()
}

pub fn tags_equal(left: &[String], right: &[String]) -> bool {
    let mut left_sorted = left.to_vec();
    let mut right_sorted = right.to_vec();
    left_sorted.sort();
    right_sorted.sort();
    left_sorted == right_sorted
}

pub fn build_tag_suggestions(deck_tags: &[String], current: &[String], query: &str) -> Vec<String> {
    let needle = query.trim();
    if needle.is_empty() {
        return Vec::new();
    }
    deck_tags
        .iter()
        .filter(|tag| !current.contains(tag))
        .filter(|tag| tag.contains(needle))
        .take(6)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_value_round_trip() {
        for sort in [CardListSort::Recent, CardListSort::Created, CardListSort::Alpha] {
            let value = sort_value(sort);
            assert_eq!(sort_from_value(value), sort);
        }
        assert_eq!(sort_from_value("unknown"), CardListSort::Recent);
    }

    #[test]
    fn tag_filter_key_is_order_insensitive() {
        let left = vec!["b".to_string(), "a".to_string()];
        let right = vec!["a".to_string(), "b".to_string()];
        assert_eq!(tag_filter_key(&left), tag_filter_key(&right));
    }

    #[test]
    fn tag_names_from_strings_skips_invalid() {
        let tags = vec!["valid".to_string(), "".to_string()];
        let mapped = tag_names_from_strings(&tags);
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].as_str(), "valid");
    }

    #[test]
    fn tags_equal_ignores_order() {
        let left = vec!["alpha".to_string(), "beta".to_string()];
        let right = vec!["beta".to_string(), "alpha".to_string()];
        assert!(tags_equal(&left, &right));
    }

    #[test]
    fn build_tag_suggestions_filters_current_and_query() {
        let deck_tags = vec![
            "alpha".to_string(),
            "beta".to_string(),
            "gamma".to_string(),
        ];
        let current = vec!["beta".to_string()];
        let suggestions = build_tag_suggestions(&deck_tags, &current, "a");
        assert_eq!(
            suggestions,
            vec!["alpha".to_string(), "gamma".to_string()]
        );
    }
}
