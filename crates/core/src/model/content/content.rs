use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::model::MediaId;
use crate::model::content::{ImageMeta, MediaDraft, MediaHash, MediaValidationError};

//
// ─── CONTENT TYPES ─────────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentDraft {
    pub text: String,
    pub media: Option<MediaDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    pub text: String,
    pub media: Option<MediaId>,
}

//
// ─── CONTENT VALIDATION ERRORS ─────────────────────────────────────────────────
//

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ContentValidationError {
    #[error("Text content cannot be empty.")]
    EmptyText,

    #[error("Image metadata must be provided when media is present.")]
    MissingImageMeta,

    #[error(transparent)]
    Media(#[from] MediaValidationError),
}

//
// ─── CONTENT DRAFT IMPL ─────────────────────────────────────────────────────────
//

impl ContentDraft {
    pub fn new(text: impl Into<String>, media: Option<MediaDraft>) -> Self {
        Self {
            text: text.into(),
            media,
        }
    }

    pub fn text_only(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            media: None,
        }
    }

    pub fn with_media(text: impl Into<String>, media: MediaDraft) -> Self {
        Self {
            text: text.into(),
            media: Some(media),
        }
    }

    pub fn validate(
        self,
        now: DateTime<Utc>,
        meta: Option<ImageMeta>,
        checksum: Option<MediaHash>,
    ) -> Result<Content, ContentValidationError> {
        if self.text.trim().is_empty() {
            return Err(ContentValidationError::EmptyText);
        }

        let media_id = match self.media {
            None => None,
            Some(draft) => {
                let meta = meta.ok_or(ContentValidationError::MissingImageMeta)?;
                let item = draft.validate(now, meta, checksum)?;
                Some(item.id)
            }
        };

        Ok(Content {
            text: self.text,
            media: media_id,
        })
    }
}

//
// ─── OPTIONAL ACCESSORS ────────────────────────────────────────────────────────
//

impl Content {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn media_id(&self) -> Option<MediaId> {
        self.media
    }

    pub fn has_media(&self) -> bool {
        self.media.is_some()
    }
}

//
// ─── TESTS ─────────────────────────────────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::content::media::MediaUri;
    #[test]
    fn empty_text_fails() {
        let d = ContentDraft::text_only("   ");
        let err = d.validate(Utc::now(), None, None).unwrap_err();
        assert_eq!(err, ContentValidationError::EmptyText);
    }

    #[test]
    fn text_only_passes() {
        let d = ContentDraft::text_only("hello");
        let c = d.validate(Utc::now(), None, None).unwrap();
        assert_eq!(c.text(), "hello");
        assert!(!c.has_media());
    }

    #[test]
    fn media_requires_meta() {
        let md = MediaDraft::new_image(MediaUri::from_file("img.png").unwrap(), None);
        let d = ContentDraft::with_media("hello", md);
        let err = d.validate(Utc::now(), None, None).unwrap_err();
        assert_eq!(err, ContentValidationError::MissingImageMeta);
    }

    #[test]
    fn text_plus_media_passes() {
        let md = MediaDraft::new_image(MediaUri::from_file("img.png").unwrap(), None);
        let d = ContentDraft::with_media("hello", md);

        let meta = ImageMeta::new(100, 50).unwrap();
        let c = d.validate(Utc::now(), Some(meta), None).unwrap();

        assert_eq!(c.text(), "hello");
        assert!(c.has_media());
        assert_eq!(c.media_id(), Some(MediaId(0)));
    }
}
