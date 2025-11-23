use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::model::{
    content::{Content, ContentDraft, ContentValidationError, ImageMeta, MediaHash},
    ids::{CardId, DeckId},
};

//
// ─── CARD TYPES ────────────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardDraft {
    pub deck_id: DeckId,
    pub prompt: ContentDraft,
    pub answer: ContentDraft,
}

impl CardDraft {
    pub fn validate(
        self,
        now: DateTime<Utc>,
        prompt_meta: Option<ImageMeta>,
        answer_meta: Option<ImageMeta>,
        prompt_checksum: Option<MediaHash>,
        answer_checksum: Option<MediaHash>,
    ) -> Result<ValidatedCard, CardValidationError> {
        let prompt = self
            .prompt
            .validate(now, prompt_meta, prompt_checksum)
            .map_err(CardValidationError::Prompt)?;

        let answer = self
            .answer
            .validate(now, answer_meta, answer_checksum)
            .map_err(CardValidationError::Answer)?;

        Ok(ValidatedCard {
            deck_id: self.deck_id,
            prompt,
            answer,
            created_at: now,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedCard {
    pub deck_id: DeckId,
    pub prompt: Content,
    pub answer: Content,
    pub created_at: DateTime<Utc>,
}

impl ValidatedCard {
    pub fn assign_id(self, id: CardId) -> Card {
        Card {
            id,
            deck_id: self.deck_id,
            prompt: self.prompt,
            answer: self.answer,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub id: CardId,
    pub deck_id: DeckId,
    pub prompt: Content,
    pub answer: Content,
    pub created_at: DateTime<Utc>,
}

//
// ─── CARD VALIDATION ERRORS ────────────────────────────────────────────────────
//

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CardValidationError {
    #[error("invalid prompt content: {0}")]
    Prompt(#[source] ContentValidationError),

    #[error("invalid answer content: {0}")]
    Answer(#[source] ContentValidationError),
}

//
// ─── TESTS ─────────────────────────────────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::content::{MediaDraft, MediaUri};
    use chrono::Utc;

    #[test]
    fn card_fails_if_prompt_text_empty() {
        let draft = CardDraft {
            deck_id: DeckId(1),
            prompt: ContentDraft::text_only("   "),
            answer: ContentDraft::text_only("ok"),
        };

        let err = draft
            .validate(Utc::now(), None, None, None, None)
            .unwrap_err();

        assert!(matches!(err, CardValidationError::Prompt(_)));
    }

    #[test]
    fn card_fails_if_answer_text_empty() {
        let draft = CardDraft {
            deck_id: DeckId(1),
            prompt: ContentDraft::text_only("ok"),
            answer: ContentDraft::text_only(" "),
        };

        let err = draft
            .validate(Utc::now(), None, None, None, None)
            .unwrap_err();

        assert!(matches!(err, CardValidationError::Answer(_)));
    }

    #[test]
    fn card_prompt_media_requires_meta() {
        let md = MediaDraft::new_image(MediaUri::from_file("img.png").unwrap(), None);

        let draft = CardDraft {
            deck_id: DeckId(1),
            prompt: ContentDraft::with_media("hello", md),
            answer: ContentDraft::text_only("ok"),
        };

        let err = draft
            .validate(Utc::now(), None, None, None, None)
            .unwrap_err();

        assert!(matches!(err, CardValidationError::Prompt(_)));
    }

    #[test]
    fn valid_card_validates_and_assigns_id() {
        let md = MediaDraft::new_image(MediaUri::from_file("img.png").unwrap(), None);

        let draft = CardDraft {
            deck_id: DeckId(1),
            prompt: ContentDraft::with_media("hello", md),
            answer: ContentDraft::text_only("ok"),
        };

        let meta = ImageMeta::new(120, 80).unwrap();
        let validated = draft
            .validate(Utc::now(), Some(meta), None, None, None)
            .unwrap();

        let card = validated.assign_id(CardId(42));
        assert_eq!(card.id, CardId(42));
        assert_eq!(card.deck_id, DeckId(1));
        assert_eq!(card.prompt.text(), "hello");
        assert_eq!(card.answer.text(), "ok");
    }
}
