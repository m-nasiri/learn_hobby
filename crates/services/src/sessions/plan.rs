use rand::rng;
use rand::seq::SliceRandom;
use std::collections::HashSet;

use learn_core::model::{Card, Deck};

/// Selection result for a session build.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionPlan {
    pub cards: Vec<Card>,
    pub due_selected: usize,
    pub new_selected: usize,
    pub future_selected: usize,
}

// Some plan helpers are currently used only in tests or planned UI flows.
#[allow(dead_code)]
impl SessionPlan {
    /// Total number of cards in this plan.
    #[must_use]
    pub fn total(&self) -> usize {
        self.cards.len()
    }

    /// Returns true when no cards were selected for this session.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
}

/// Builds a micro-session by picking due and new cards according to deck settings.
pub struct SessionBuilder<'a> {
    deck: &'a Deck,
    shuffle_new: bool,
}

impl<'a> SessionBuilder<'a> {
    #[must_use]
    pub fn new(deck: &'a Deck) -> Self {
        Self {
            deck,
            shuffle_new: false,
        }
    }

    /// Enable or disable shuffling among new cards before selection.
    #[must_use]
    pub fn with_shuffle_new(mut self, shuffle: bool) -> Self {
        self.shuffle_new = shuffle;
        self
    }

    /// Build a session plan from storage-provided lists of due and new cards.
    ///
    /// - `due_cards` are assumed to already be due; they are sorted by `next_review_at`.
    /// - `new_cards` are unreviewed; they are optionally shuffled.
    /// - Selection respects deck `review_limit_per_day`, `new_cards_per_day`, and `micro_session_size`.
    pub fn build(
        self,
        due_cards: impl IntoIterator<Item = Card>,
        new_cards: impl IntoIterator<Item = Card>,
    ) -> SessionPlan {
        let settings = self.deck.settings();
        let micro_cap = usize::try_from(settings.micro_session_size()).unwrap_or(usize::MAX);
        let due_cap = if settings.protect_overload() {
            usize::try_from(settings.review_limit_per_day()).unwrap_or(usize::MAX)
        } else {
            usize::MAX
        };
        let new_cap = usize::try_from(settings.new_cards_per_day()).unwrap_or(usize::MAX);

        let mut due: Vec<Card> = due_cards.into_iter().collect();
        due.sort_by_key(|c| (c.next_review_at(), c.id().value()));

        let mut selected = Vec::new();

        let due_take = due_cap.min(micro_cap);
        let due_selected = due.into_iter().take(due_take).collect::<Vec<_>>();
        let due_count = due_selected.len();
        selected.extend(due_selected);

        let mut selected_ids: HashSet<_> = selected.iter().map(Card::id).collect();

        let remaining = micro_cap.saturating_sub(selected.len());
        let mut new_count = 0;
        if remaining > 0 && new_cap > 0 {
            let take = new_cap.min(remaining);
            let mut new_candidates: Vec<Card> = new_cards
                .into_iter()
                .filter(|c| !selected_ids.contains(&c.id()))
                .collect();

            if self.shuffle_new {
                let mut rng = rng();
                new_candidates.as_mut_slice().shuffle(&mut rng);
            } else {
                new_candidates.sort_by_key(|c| (c.created_at(), c.id().value()));
            }

            let new_cards: Vec<Card> = new_candidates.into_iter().take(take).collect();
            new_count = new_cards.len();
            selected_ids.extend(new_cards.iter().map(Card::id));
            selected.extend(new_cards);
        }

        SessionPlan {
            cards: selected,
            due_selected: due_count,
            new_selected: new_count,
            future_selected: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use learn_core::model::{CardId, DeckId, ReviewGrade, content::ContentDraft};
    use learn_core::scheduler::Scheduler;
    use learn_core::time::fixed_now;

    fn build_card(id: u64) -> Card {
        let prompt = ContentDraft::text_only("Q")
            .validate(fixed_now(), None, None)
            .unwrap();
        let answer = ContentDraft::text_only("A")
            .validate(fixed_now(), None, None)
            .unwrap();
        let now = fixed_now();
        Card::new(CardId::new(id), DeckId::new(1), prompt, answer, now, now).unwrap()
    }

    fn build_deck() -> Deck {
        Deck::new(
            DeckId::new(1),
            "Test",
            None,
            learn_core::model::DeckSettings::default_for_adhd(),
            fixed_now(),
        )
        .unwrap()
    }

    fn build_deck_with_settings(settings: learn_core::model::DeckSettings) -> Deck {
        Deck::new(DeckId::new(1), "Test", None, settings, fixed_now()).unwrap()
    }

    fn build_due_card(id: u64, reviewed_days_ago: i64) -> Card {
        let mut card = build_card(id);
        let scheduler = Scheduler::new().unwrap();
        let reviewed_at = fixed_now() - chrono::Duration::days(reviewed_days_ago);
        let applied = scheduler
            .apply_review(card.id(), None, ReviewGrade::Good, reviewed_at, 0.0)
            .unwrap();
        card.apply_review(&applied.outcome, reviewed_at);
        card
    }

    #[test]
    fn builder_prioritizes_due_and_limits_new() {
        let deck = build_deck();
        let due = build_due_card(1, 2);
        let new1 = build_card(2);
        let new2 = build_card(3);

        let plan =
            SessionBuilder::new(&deck).build(vec![due.clone()], vec![new1.clone(), new2.clone()]);

        assert_eq!(plan.due_selected, 1);
        assert!(plan.cards.iter().any(|c| c.id() == due.id()));
        assert!(plan.new_selected <= deck.settings().new_cards_per_day() as usize);
        assert!(plan.cards.len() <= deck.settings().micro_session_size() as usize);
        assert_eq!(plan.future_selected, 0);
    }

    #[test]
    fn builder_caps_micro_session_size() {
        let mut due_cards = Vec::new();
        let mut new_cards = Vec::new();
        for i in 0..10 {
            if i % 2 == 0 {
                due_cards.push(build_due_card(i, 2));
            } else {
                new_cards.push(build_card(i));
            }
        }
        let deck = build_deck();
        let plan = SessionBuilder::new(&deck).build(due_cards, new_cards);
        assert!(plan.cards.len() <= deck.settings().micro_session_size() as usize);
    }

    #[test]
    fn builder_ignores_review_limit_when_overload_protection_off() {
        let settings = learn_core::model::DeckSettings::new(5, 1, 10, false).unwrap();
        let deck = build_deck_with_settings(settings);
        let due_cards = vec![build_due_card(1, 2), build_due_card(2, 2), build_due_card(3, 2)];

        let plan = SessionBuilder::new(&deck).build(due_cards, Vec::new());

        assert!(plan.due_selected > 1);
        assert_eq!(plan.due_selected, plan.cards.len());
    }
}
