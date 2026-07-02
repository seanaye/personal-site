use std::collections::HashMap;

use bytes::Bytes;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
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

/// Lazily-evaluated stream of raw image bytes.
pub type ImageByteStream = BoxStream<'static, std::io::Result<Bytes>>;

/// Raw image content fetched from the backing store.
///
/// The bytes are exposed as a lazily-evaluated [`ImageByteStream`] so callers
/// can forward them (e.g. into an HTTP response body) without buffering the
/// whole image in memory.
pub struct ImageBytes {
    pub stream: ImageByteStream,
    pub content_type: Option<String>,
}

impl ImageBytes {
    pub fn new(stream: ImageByteStream, content_type: Option<String>) -> Self {
        Self {
            stream,
            content_type,
        }
    }

    /// Wrap an already-buffered chunk of bytes as a single-item stream.
    pub fn from_bytes(bytes: impl Into<Bytes>, content_type: Option<String>) -> Self {
        let bytes = bytes.into();
        Self {
            stream: futures::stream::once(async move { Ok(bytes) }).boxed(),
            content_type,
        }
    }

    /// Drain the stream into a single buffer.
    ///
    /// Convenience for callers (and tests) that do want the whole image in
    /// memory.
    pub async fn into_bytes(self) -> std::io::Result<Bytes> {
        let chunks: Vec<Bytes> = self.stream.try_collect().await?;
        Ok(chunks.concat().into())
    }
}

impl std::fmt::Debug for ImageBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageBytes")
            .field("stream", &"<byte stream>")
            .field("content_type", &self.content_type)
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("image `{id}` has no variants")]
    MissingVariants { id: ImageId },
}
