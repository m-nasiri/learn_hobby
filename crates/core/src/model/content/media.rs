use crate::model::ids::MediaId;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use thiserror::Error;
use url::Url;

//
// ─── ERRORS (domain validation) ────────────────────────────────────────────────
//

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MediaValidationError {
    #[error("Media URI cannot be empty.")]
    EmptyMediaUri,

    #[error("Image dimensions cannot be zero.")]
    InvalidImageDimensions,
}

//
// ─── MEDIA CORE TYPES ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaKind {
    Image,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaUri {
    FilePath(PathBuf),
    Url(Url),
}

impl MediaUri {
    pub fn from_file(path: impl Into<PathBuf>) -> Result<Self, MediaValidationError> {
        let p = path.into();
        if p.as_os_str().is_empty() {
            return Err(MediaValidationError::EmptyMediaUri);
        }
        Ok(MediaUri::FilePath(p))
    }

    pub fn from_url(url: impl AsRef<str>) -> Result<Self, MediaValidationError> {
        let s = url.as_ref().trim();
        if s.is_empty() {
            return Err(MediaValidationError::EmptyMediaUri);
        }
        let u = Url::parse(s).map_err(|_| MediaValidationError::EmptyMediaUri)?;
        Ok(MediaUri::Url(u))
    }

    fn validate_non_empty(&self) -> Result<(), MediaValidationError> {
        match self {
            MediaUri::FilePath(p) => {
                if p.as_os_str().is_empty() {
                    Err(MediaValidationError::EmptyMediaUri)
                } else {
                    Ok(())
                }
            }
            MediaUri::Url(u) => {
                if u.as_str().trim().is_empty() {
                    Err(MediaValidationError::EmptyMediaUri)
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn as_path(&self) -> Option<&Path> {
        match self {
            MediaUri::FilePath(p) => Some(p.as_path()),
            _ => None,
        }
    }

    pub fn as_url(&self) -> Option<&Url> {
        match self {
            MediaUri::Url(u) => Some(u),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaHash(String);

impl MediaHash {
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageMeta {
    pub width: u32,
    pub height: u32,
}

impl ImageMeta {
    pub fn new(width: u32, height: u32) -> Result<Self, MediaValidationError> {
        if width == 0 || height == 0 {
            return Err(MediaValidationError::InvalidImageDimensions);
        }
        Ok(Self { width, height })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaAltText(String);

impl MediaAltText {
    pub fn new(raw: impl Into<String>) -> Result<Self, MediaValidationError> {
        let s = raw.into();
        if s.trim().is_empty() {
            return Err(MediaValidationError::EmptyMediaUri);
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

//
// ─── VALIDATED DOMAIN ENTITY ───────────────────────────────────────────────────
//

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaItem {
    pub id: MediaId,
    pub kind: MediaKind,
    pub uri: MediaUri,
    pub created_at: DateTime<Utc>,
    pub checksum: Option<MediaHash>,
    pub meta: ImageMeta,
    pub alt_text: Option<MediaAltText>,
}

impl MediaItem {
    pub fn with_id(mut self, id: MediaId) -> Self {
        self.id = id;
        self
    }
}

//
// ─── DRAFT ENTITY (unvalidated input) ──────────────────────────────────────────
//

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaDraft {
    pub kind: MediaKind,
    pub uri: MediaUri,
    pub alt_text: Option<MediaAltText>,
}

impl MediaDraft {
    pub fn new_image(uri: MediaUri, alt_text: Option<MediaAltText>) -> Self {
        Self {
            kind: MediaKind::Image,
            uri,
            alt_text,
        }
    }

    pub fn validate(
        self,
        now: DateTime<Utc>,
        meta: ImageMeta,
        checksum: Option<MediaHash>,
    ) -> Result<MediaItem, MediaValidationError> {
        self.uri.validate_non_empty()?;

        if meta.width == 0 || meta.height == 0 {
            return Err(MediaValidationError::InvalidImageDimensions);
        }

        Ok(MediaItem {
            id: MediaId(0),
            kind: self.kind,
            uri: self.uri,
            created_at: now,
            checksum,
            meta,
            alt_text: self.alt_text,
        })
    }
}
