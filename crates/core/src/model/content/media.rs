use crate::model::ids::MediaId;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use thiserror::Error;
use url::Url;

//
// ─── ERRORS (domain validation) ────────────────────────────────────────────────
//

/// Errors that can occur during media validation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MediaValidationError {
    #[error("Media URI cannot be empty.")]
    EmptyMediaUri,

    #[error("Image dimensions cannot be zero.")]
    InvalidImageDimensions,

    #[error("Alt text cannot be empty.")]
    EmptyAltText,
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
    /// Creates a `MediaUri` from a file path.
    ///
    /// # Errors
    ///
    /// Returns `MediaValidationError::EmptyMediaUri` if the path is empty.
    pub fn from_file(path: impl Into<PathBuf>) -> Result<Self, MediaValidationError> {
        let p = path.into();
        if p.as_os_str().is_empty() {
            return Err(MediaValidationError::EmptyMediaUri);
        }
        Ok(MediaUri::FilePath(p))
    }

    /// Creates a `MediaUri` from a URL string.
    ///
    /// # Errors
    ///
    /// Returns `MediaValidationError::EmptyMediaUri` if the URL is empty or invalid.
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

    #[must_use]
    pub fn as_path(&self) -> Option<&Path> {
        match self {
            MediaUri::FilePath(p) => Some(p.as_path()),
            MediaUri::Url(_) => None,
        }
    }

    #[must_use]
    pub fn as_url(&self) -> Option<&Url> {
        match self {
            MediaUri::Url(u) => Some(u),
            MediaUri::FilePath(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaHash(String);

impl MediaHash {
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    #[must_use]
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
    /// Creates new image metadata with dimensions.
    ///
    /// # Errors
    ///
    /// Returns `MediaValidationError::InvalidImageDimensions` if width or height is zero.
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
    /// Creates new alt text for media.
    ///
    /// # Errors
    ///
    /// Returns `MediaValidationError::EmptyAltText` if the text is empty or whitespace-only.
    pub fn new(raw: impl Into<String>) -> Result<Self, MediaValidationError> {
        let s = raw.into();
        if s.trim().is_empty() {
            return Err(MediaValidationError::EmptyAltText);
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

//
// ─── VALIDATED DOMAIN ENTITY ───────────────────────────────────────────────────
//

/// A validated media item with metadata.
///
/// `MediaItem` is created via `MediaDraft::validate()` and guarantees:
/// - Valid URI (non-empty file path or URL)
/// - Valid image dimensions (width and height > 0)
/// - Assigned ID (initially 0, updated via `with_id()`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaItem {
    id: MediaId,
    kind: MediaKind,
    uri: MediaUri,
    created_at: DateTime<Utc>,
    checksum: Option<MediaHash>,
    meta: ImageMeta,
    alt_text: Option<MediaAltText>,
}

impl MediaItem {
    /// Updates the `MediaItem` with a database-assigned ID.
    ///
    /// This consumes self and returns a new `MediaItem` with the updated ID.
    #[must_use]
    pub fn with_id(mut self, id: MediaId) -> Self {
        self.id = id;
        self
    }

    // Accessors
    #[must_use]
    pub fn id(&self) -> MediaId {
        self.id
    }

    #[must_use]
    pub fn kind(&self) -> &MediaKind {
        &self.kind
    }

    #[must_use]
    pub fn uri(&self) -> &MediaUri {
        &self.uri
    }

    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    #[must_use]
    pub fn checksum(&self) -> Option<&MediaHash> {
        self.checksum.as_ref()
    }

    #[must_use]
    pub fn meta(&self) -> &ImageMeta {
        &self.meta
    }

    #[must_use]
    pub fn alt_text(&self) -> Option<&MediaAltText> {
        self.alt_text.as_ref()
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
    #[must_use]
    pub fn new_image(uri: MediaUri, alt_text: Option<MediaAltText>) -> Self {
        Self {
            kind: MediaKind::Image,
            uri,
            alt_text,
        }
    }

    /// Validates the media draft and creates a `MediaItem`.
    ///
    /// # Errors
    ///
    /// Returns `MediaValidationError::EmptyMediaUri` if the URI is empty.
    /// Returns `MediaValidationError::InvalidImageDimensions` if width or height is zero.
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
            id: MediaId::new(0),
            kind: self.kind,
            uri: self.uri,
            created_at: now,
            checksum,
            meta,
            alt_text: self.alt_text,
        })
    }
}

//
// ─── UNIT TESTS ──────────────────────────────────────────
//

#[cfg(test)]
mod tests {
    use super::*;

    // ─── MediaUri Tests ─────────────────────────────────────────────────

    #[test]
    fn test_media_uri_from_file_valid() {
        let uri = MediaUri::from_file("/path/to/image.png").unwrap();
        assert!(matches!(uri, MediaUri::FilePath(_)));
        assert_eq!(
            uri.as_path().unwrap().to_str().unwrap(),
            "/path/to/image.png"
        );
    }

    #[test]
    fn test_media_uri_from_file_empty_path() {
        let result = MediaUri::from_file("");
        assert_eq!(result, Err(MediaValidationError::EmptyMediaUri));
    }

    #[test]
    fn test_media_uri_from_url_valid() {
        let uri = MediaUri::from_url("https://example.com/image.jpg").unwrap();
        assert!(matches!(uri, MediaUri::Url(_)));
        assert_eq!(
            uri.as_url().unwrap().as_str(),
            "https://example.com/image.jpg"
        );
    }

    #[test]
    fn test_media_uri_from_url_empty_string() {
        let result = MediaUri::from_url("");
        assert_eq!(result, Err(MediaValidationError::EmptyMediaUri));
    }

    #[test]
    fn test_media_uri_from_url_whitespace_only() {
        let result = MediaUri::from_url("   ");
        assert_eq!(result, Err(MediaValidationError::EmptyMediaUri));
    }

    #[test]
    fn test_media_uri_from_url_invalid_url() {
        let result = MediaUri::from_url("not a valid url");
        assert_eq!(result, Err(MediaValidationError::EmptyMediaUri));
    }

    #[test]
    fn test_media_uri_as_path_returns_none_for_url() {
        let uri = MediaUri::from_url("https://example.com/image.jpg").unwrap();
        assert!(uri.as_path().is_none());
    }

    #[test]
    fn test_media_uri_as_url_returns_none_for_filepath() {
        let uri = MediaUri::from_file("/path/to/image.png").unwrap();
        assert!(uri.as_url().is_none());
    }

    #[test]
    fn test_media_uri_validate_non_empty_filepath_success() {
        let uri = MediaUri::from_file("/path/to/image.png").unwrap();
        assert!(uri.validate_non_empty().is_ok());
    }

    #[test]
    fn test_media_uri_validate_non_empty_url_success() {
        let uri = MediaUri::from_url("https://example.com/image.jpg").unwrap();
        assert!(uri.validate_non_empty().is_ok());
    }

    // ─── MediaHash Tests ────────────────────────────────────────────────

    #[test]
    fn test_media_hash_new() {
        let hash = MediaHash::new("abc123");
        assert_eq!(hash.as_str(), "abc123");
    }

    #[test]
    fn test_media_hash_clone() {
        let hash1 = MediaHash::new("abc123");
        let hash2 = hash1.clone();
        assert_eq!(hash1, hash2);
    }

    // ─── ImageMeta Tests ────────────────────────────────────────────────

    #[test]
    fn test_image_meta_new_valid() {
        let meta = ImageMeta::new(1920, 1080).unwrap();
        assert_eq!(meta.width, 1920);
        assert_eq!(meta.height, 1080);
    }

    #[test]
    fn test_image_meta_zero_width() {
        let result = ImageMeta::new(0, 1080);
        assert_eq!(result, Err(MediaValidationError::InvalidImageDimensions));
    }

    #[test]
    fn test_image_meta_zero_height() {
        let result = ImageMeta::new(1920, 0);
        assert_eq!(result, Err(MediaValidationError::InvalidImageDimensions));
    }

    #[test]
    fn test_image_meta_both_zero() {
        let result = ImageMeta::new(0, 0);
        assert_eq!(result, Err(MediaValidationError::InvalidImageDimensions));
    }

    // ─── MediaAltText Tests ─────────────────────────────────────────────

    #[test]
    fn test_media_alt_text_new_valid() {
        let alt = MediaAltText::new("A beautiful sunset").unwrap();
        assert_eq!(alt.as_str(), "A beautiful sunset");
    }

    #[test]
    fn test_media_alt_text_empty_string() {
        let result = MediaAltText::new("");
        assert_eq!(result, Err(MediaValidationError::EmptyAltText));
    }

    #[test]
    fn test_media_alt_text_whitespace_only() {
        let result = MediaAltText::new("   ");
        assert_eq!(result, Err(MediaValidationError::EmptyAltText));
    }

    #[test]
    fn test_media_alt_text_with_leading_trailing_whitespace() {
        let alt = MediaAltText::new("  text with spaces  ").unwrap();
        assert_eq!(alt.as_str(), "  text with spaces  ");
    }

    // ─── MediaDraft Tests ───────────────────────────────────────────────

    #[test]
    fn test_media_draft_new_image() {
        let uri = MediaUri::from_file("/path/to/image.png").unwrap();
        let alt = MediaAltText::new("Test image").ok();
        let draft = MediaDraft::new_image(uri.clone(), alt.clone());

        assert!(matches!(draft.kind, MediaKind::Image));
        assert_eq!(draft.uri, uri);
        assert_eq!(draft.alt_text, alt);
    }

    #[test]
    fn test_media_draft_validate_success() {
        let uri = MediaUri::from_file("/path/to/image.png").unwrap();
        let alt = MediaAltText::new("Test image").ok();
        let draft = MediaDraft::new_image(uri, alt);

        let now = Utc::now();
        let meta = ImageMeta::new(1920, 1080).unwrap();
        let checksum = Some(MediaHash::new("abc123"));

        let result = draft.validate(now, meta.clone(), checksum.clone());
        assert!(result.is_ok());

        let media_item = result.unwrap();
        assert!(matches!(media_item.kind(), &MediaKind::Image));
        assert_eq!(media_item.meta(), &meta);
        assert_eq!(media_item.checksum(), checksum.as_ref());
        assert_eq!(media_item.created_at(), now);
    }

    #[test]
    fn test_media_draft_validate_invalid_dimensions() {
        let uri = MediaUri::from_file("/path/to/image.png").unwrap();
        let draft = MediaDraft::new_image(uri, None);

        let now = Utc::now();
        let meta = ImageMeta {
            width: 0,
            height: 1080,
        };

        let result = draft.validate(now, meta, None);
        assert_eq!(result, Err(MediaValidationError::InvalidImageDimensions));
    }

    #[test]
    fn test_media_draft_validate_no_checksum() {
        let uri = MediaUri::from_url("https://example.com/image.jpg").unwrap();
        let draft = MediaDraft::new_image(uri, None);

        let now = Utc::now();
        let meta = ImageMeta::new(800, 600).unwrap();

        let result = draft.validate(now, meta, None);
        assert!(result.is_ok());
        assert!(result.unwrap().checksum.is_none());
    }

    // ─── MediaItem Tests ────────────────────────────────────────────────

    #[test]
    fn test_media_item_with_id() {
        let uri = MediaUri::from_file("/path/to/image.png").unwrap();
        let draft = MediaDraft::new_image(uri, None);

        let now = Utc::now();
        let meta = ImageMeta::new(1920, 1080).unwrap();

        let media_item = draft.validate(now, meta, None).unwrap();
        assert_eq!(media_item.id(), MediaId::new(0));

        let updated = media_item.with_id(MediaId::new(42));
        assert_eq!(updated.id(), MediaId::new(42));
    }

    #[test]
    fn test_media_item_complete_workflow() {
        // Create URI
        let uri = MediaUri::from_url("https://example.com/photo.jpg").unwrap();

        // Create alt text
        let alt = MediaAltText::new("A scenic photo").unwrap();

        // Create draft
        let draft = MediaDraft::new_image(uri.clone(), Some(alt.clone()));

        // Validate and create media item
        let now = Utc::now();
        let meta = ImageMeta::new(4096, 2160).unwrap();
        let checksum = MediaHash::new("sha256:abcdef");

        let media_item = draft
            .validate(now, meta.clone(), Some(checksum.clone()))
            .unwrap()
            .with_id(MediaId::new(100));

        // Verify all fields
        assert_eq!(media_item.id(), MediaId::new(100));
        assert!(matches!(media_item.kind(), &MediaKind::Image));
        assert_eq!(media_item.uri(), &uri);
        assert_eq!(media_item.created_at(), now);
        assert_eq!(media_item.checksum(), Some(&checksum));
        assert_eq!(media_item.meta(), &meta);
        assert_eq!(media_item.alt_text(), Some(&alt));
    }

    // ─── Edge Cases and Integration Tests ──────────────────────────────

    #[test]
    fn test_media_uri_with_special_characters() {
        let uri = MediaUri::from_file("/path/with spaces/and-special_chars.png").unwrap();
        assert!(uri.as_path().is_some());
    }

    #[test]
    fn test_media_uri_url_with_query_params() {
        let uri =
            MediaUri::from_url("https://example.com/image.jpg?size=large&format=png").unwrap();
        assert!(uri.as_url().unwrap().as_str().contains("size=large"));
    }

    #[test]
    fn test_image_meta_very_large_dimensions() {
        let meta = ImageMeta::new(u32::MAX, u32::MAX).unwrap();
        assert_eq!(meta.width, u32::MAX);
        assert_eq!(meta.height, u32::MAX);
    }

    #[test]
    fn test_media_hash_empty_string() {
        let hash = MediaHash::new("");
        assert_eq!(hash.as_str(), "");
    }

    #[test]
    fn test_media_alt_text_unicode() {
        let alt = MediaAltText::new("图片描述 🌅 ñoño").unwrap();
        assert_eq!(alt.as_str(), "图片描述 🌅 ñoño");
    }

    #[test]
    fn test_media_draft_with_url_and_alt() {
        let uri = MediaUri::from_url("https://cdn.example.com/images/photo123.webp").unwrap();
        let alt = MediaAltText::new("Product showcase photo").unwrap();
        let draft = MediaDraft::new_image(uri.clone(), Some(alt.clone()));

        let now = Utc::now();
        let meta = ImageMeta::new(2048, 1536).unwrap();

        let media_item = draft.validate(now, meta, None).unwrap();

        assert_eq!(media_item.uri(), &uri);
        assert_eq!(media_item.alt_text(), Some(&alt));
        assert!(media_item.checksum().is_none());
    }
}
