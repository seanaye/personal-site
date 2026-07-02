use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use url::Url;

pub type ImageMetadata = HashMap<String, String>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImageId(String);

impl ImageId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ImageId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for ImageId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for ImageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageDimensions {
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageVariant {
    pub url: Url,
    pub dimensions: ImageDimensions,
}

impl ImageVariant {
    pub fn new(url: Url, dimensions: ImageDimensions) -> Self {
        Self { url, dimensions }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub id: ImageId,
    pub variants: Vec<ImageVariant>,
    pub metadata: ImageMetadata,
}

impl Image {
    pub fn new(
        id: ImageId,
        variants: Vec<ImageVariant>,
        metadata: ImageMetadata,
    ) -> Result<Self, ImageError> {
        if variants.is_empty() {
            return Err(ImageError::MissingVariants { id });
        }

        Ok(Self {
            id,
            variants,
            metadata,
        })
    }
}

/// Opaque cursor used to continue a paginated image listing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImageCursor(String);

impl ImageCursor {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ImageCursor {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for ImageCursor {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for ImageCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One page of a cursor-paginated image listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagePage {
    pub images: Vec<Image>,
    /// Cursor for the next page, or `None` when this is the last page.
    pub next_cursor: Option<ImageCursor>,
}

/// Raw image content fetched from the backing store.
#[derive(Debug, Clone)]
pub struct ImageBytes {
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("image `{id}` has no variants")]
    MissingVariants { id: ImageId },
}
