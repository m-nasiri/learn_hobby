use learn_core::model::CardId;

/// UI-ready summary of a card for list rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardListItemVm {
    pub id: CardId,
    pub prompt: String,
    pub answer: String,
    pub prompt_preview: String,
    pub answer_preview: String,
}

impl CardListItemVm {
    #[must_use]
    pub fn new(
        id: CardId,
        prompt: String,
        answer: String,
        prompt_preview: String,
        answer_preview: String,
    ) -> Self {
        Self {
            id,
            prompt,
            answer,
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
pub fn build_card_list_item(id: CardId, prompt: &str, answer: &str) -> CardListItemVm {
    let prompt = prompt.to_owned();
    let answer = answer.to_owned();
    let prompt_preview = truncate_preview(&prompt, 56);
    let answer_preview = truncate_preview(&answer, 56);
    CardListItemVm::new(id, prompt, answer, prompt_preview, answer_preview)
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
