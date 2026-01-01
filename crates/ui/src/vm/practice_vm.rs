use learn_core::model::{Deck, DeckId};
use services::{DeckPracticeStats, TagPracticeStats};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PracticeTagPillVm {
    pub name: String,
    pub due_label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PracticeDeckCardVm {
    pub id: DeckId,
    pub name: String,
    pub avatar: String,
    pub due_label: String,
    pub new_label: String,
    pub total_label: String,
    pub tag_pills: Vec<PracticeTagPillVm>,
    pub extra_tag_label: Option<String>,
}

#[must_use]
pub fn map_practice_deck_card(
    deck: &Deck,
    stats: DeckPracticeStats,
    tag_stats: &[TagPracticeStats],
) -> PracticeDeckCardVm {
    let due_label = if stats.due == 0 {
        "0 Due".to_string()
    } else {
        format!("{} Due", stats.due)
    };

    let new_label = format!("{} New", stats.new);
    let total_label = format!("{} Total", stats.total);

    let avatar = deck
        .name()
        .chars()
        .next()
        .map_or_else(|| "?".to_string(), |ch| ch.to_string());

    let mut tag_items = tag_stats
        .iter()
        .map(|tag| {
            let due_label = if tag.due == 0 {
                None
            } else {
                Some(format!("{} Due", tag.due))
            };
            PracticeTagPillVm {
                name: tag.name.as_str().to_string(),
                due_label,
            }
        })
        .collect::<Vec<_>>();
    tag_items.sort_by(|left, right| {
        right
            .due_label
            .is_some()
            .cmp(&left.due_label.is_some())
            .then_with(|| left.name.cmp(&right.name))
    });

    let extra_tag_label = if tag_items.len() > 2 {
        let extra = tag_items.len() - 2;
        tag_items.truncate(2);
        Some(format!("+{extra}"))
    } else {
        None
    };

    PracticeDeckCardVm {
        id: deck.id(),
        name: deck.name().to_string(),
        avatar,
        due_label,
        new_label,
        total_label,
        tag_pills: tag_items,
        extra_tag_label,
    }
}

impl PracticeDeckCardVm {
    #[must_use]
    pub fn matches_query(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        self.name.to_lowercase().contains(query)
    }
}
