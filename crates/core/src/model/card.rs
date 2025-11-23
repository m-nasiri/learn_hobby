use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::model::{
    content::{Content, ContentValidationError},
    ids::{CardId, DeckId},
};

//
// ─── ERRORS ────────────────────────────────────────────────────────────────────
//

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CardError {
    #[error("invalid prompt content: {0}")]
    InvalidPrompt(#[source] ContentValidationError),

    #[error("invalid answer content: {0}")]
    InvalidAnswer(#[source] ContentValidationError),
}

//
// ─── CARD ──────────────────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub id: CardId,
    pub deck_id: DeckId,
    pub prompt: Content,
    pub answer: Content,
    pub created_at: DateTime<Utc>,
}

impl Card {
    pub fn new(
        id: CardId,
        deck_id: DeckId,
        prompt: Content,
        answer: Content,
        created_at: DateTime<Utc>,
    ) -> Result<Self, CardError> {
        if prompt.text().trim().is_empty() {
            return Err(CardError::InvalidPrompt(ContentValidationError::EmptyText));
        }

        if answer.text().trim().is_empty() {
            return Err(CardError::InvalidAnswer(ContentValidationError::EmptyText));
        }

        Ok(Self {
            id,
            deck_id,
            prompt,
            answer,
            created_at,
        })
    }
}

//
// ─── TESTS ─────────────────────────────────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::content::Content;

    #[test]
    fn empty_prompt_fails() {
        let prompt = Content {
            text: "".into(),
            media: None,
        };

        let answer = Content {
            text: "ok".into(),
            media: None,
        };

        let err = Card::new(CardId(1), DeckId(1), prompt, answer, chrono::Utc::now()).unwrap_err();

        matches!(err, CardError::InvalidPrompt(_));
    }

    #[test]
    fn empty_answer_fails() {
        let prompt = Content {
            text: "hello".into(),
            media: None,
        };

        let answer = Content {
            text: "".into(),
            media: None,
        };

        let err = Card::new(CardId(1), DeckId(1), prompt, answer, chrono::Utc::now()).unwrap_err();

        matches!(err, CardError::InvalidAnswer(_));
    }

    #[test]
    fn valid_card_passes() {
        let prompt = Content {
            text: "hello".into(),
            media: None,
        };

        let answer = Content {
            text: "world".into(),
            media: None,
        };

        let card = Card::new(CardId(10), DeckId(5), prompt, answer, chrono::Utc::now()).unwrap();

        assert_eq!(card.id, CardId(10));
        assert_eq!(card.deck_id, DeckId(5));
    }
}
