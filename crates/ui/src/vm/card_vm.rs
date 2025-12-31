use learn_core::model::CardId;

use super::markdown_vm::{sanitize_html, strip_html_tags};

/// UI-ready summary of a card for list rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardListItemVm {
    pub id: CardId,
    pub prompt_html: String,
    pub answer_html: String,
    pub prompt_text: String,
    pub answer_text: String,
    pub prompt_preview: String,
    pub answer_preview: String,
}

impl CardListItemVm {
    #[must_use]
    pub fn new(
        id: CardId,
        prompt_html: String,
        answer_html: String,
        prompt_text: String,
        answer_text: String,
        prompt_preview: String,
        answer_preview: String,
    ) -> Self {
        Self {
            id,
            prompt_html,
            answer_html,
            prompt_text,
            answer_text,
            prompt_preview,
            answer_preview,
        }
    }
}

/// Map domain cards into list-friendly view models.
#[must_use]
pub fn map_card_list_items(cards: &[learn_core::model::Card]) -> Vec<CardListItemVm> {
    cards
        .iter()
        .map(|card| build_card_list_item(card.id(), card.prompt().text(), card.answer().text()))
        .collect()
}

/// Build a list item view model from raw prompt/answer text.
#[must_use]
pub fn build_card_list_item(id: CardId, prompt_html: &str, answer_html: &str) -> CardListItemVm {
    let prompt_html = sanitize_html(prompt_html);
    let answer_html = sanitize_html(answer_html);
    let prompt_text = strip_html_tags(&prompt_html);
    let answer_text = strip_html_tags(&answer_html);
    let prompt_preview = truncate_preview(&prompt_text, 56);
    let answer_preview = truncate_preview(&answer_text, 56);
    CardListItemVm::new(
        id,
        prompt_html,
        answer_html,
        prompt_text,
        answer_text,
        prompt_preview,
        answer_preview,
    )
}

/// Filter list items by a search query (case-insensitive).
#[must_use]
pub fn filter_card_list_items(items: &[CardListItemVm], query: &str) -> Vec<CardListItemVm> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return items.to_vec();
    }

    items
        .iter()
        .filter(|item| {
            item.prompt_text.to_lowercase().contains(&needle)
                || item.answer_text.to_lowercase().contains(&needle)
        })
        .cloned()
        .collect()
}

fn truncate_preview(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut lines = trimmed.lines();
    let first_line = lines.next().unwrap_or("");
    let has_more_lines = lines.next().is_some();

    let mut out = String::with_capacity(max_chars + 3);
    let mut count = 0usize;
    let mut cut = false;
    for ch in first_line.chars() {
        if count >= max_chars {
            cut = true;
            break;
        }
        out.push(ch);
        count = count.saturating_add(1);
    }

    if has_more_lines || cut {
        out.push_str("...");
    }

    out
}
