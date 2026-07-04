use std::{collections::HashMap, future::Future};

use chrono::{DateTime, Utc};

use crate::{
    domain::{Image, ImageBytes, ImageCursor, ImageDateRange, ImageId, ImagePage},
    ports::ImageRepository,
};

#[derive(Debug, Clone, Default)]
pub struct InMemoryImageRepository {
    images: Vec<Image>,
    bytes: HashMap<ImageId, Vec<u8>>,
}

impl InMemoryImageRepository {
    pub fn new(images: Vec<(Image, Vec<u8>)>) -> Self {
        let bytes = images
            .iter()
            .map(|(image, bytes)| (image.id.clone(), bytes.clone()))
            .collect();
        let images = images.into_iter().map(|(image, _)| image).collect();
        Self { images, bytes }
    }

    /// When the image was taken, read from the `createdAt` metadata entry.
    fn taken_at(image: &Image) -> Option<DateTime<Utc>> {
        image
            .metadata
            .get("createdAt")
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
    }

    fn paginate(
        images: Vec<Image>,
        cursor: Option<ImageCursor>,
        limit: usize,
    ) -> Result<ImagePage, InMemoryRepositoryError> {
        let offset = match cursor {
            None => 0,
            Some(cursor) => cursor
                .as_str()
                .parse::<usize>()
                .map_err(|_| InMemoryRepositoryError::InvalidCursor(cursor))?,
        };

        let total = images.len();
        let page: Vec<Image> = images.into_iter().skip(offset).take(limit).collect();
        let end = offset.saturating_add(page.len());
        let next_cursor = (end < total).then(|| ImageCursor::new(end.to_string()));

        Ok(ImagePage {
            images: page,
            next_cursor,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InMemoryRepositoryError {
    #[error("invalid pagination cursor `{0}`")]
    InvalidCursor(ImageCursor),

    #[error("image `{0}` not found")]
    NotFound(ImageId),
}

impl ImageRepository for InMemoryImageRepository {
    type Error = InMemoryRepositoryError;

    fn list_images(
        &self,
        cursor: Option<ImageCursor>,
        limit: usize,
    ) -> impl Future<Output = Result<ImagePage, Self::Error>> + Send {
        async move { Self::paginate(self.images.clone(), cursor, limit) }
    }

    fn list_images_taken_in(
        &self,
        range: ImageDateRange,
        cursor: Option<ImageCursor>,
        limit: usize,
    ) -> impl Future<Output = Result<ImagePage, Self::Error>> + Send {
        async move {
            let images: Vec<Image> = self
                .images
                .iter()
                .filter(|image| Self::taken_at(image).is_some_and(|taken| range.contains(taken)))
                .cloned()
                .collect();

            Self::paginate(images, cursor, limit)
        }
    }

    fn fetch_image_bytes(
        &self,
        id: &ImageId,
    ) -> impl Future<Output = Result<ImageBytes, Self::Error>> + Send {
        let bytes = self.bytes.get(id).cloned();
        let id = id.clone();
        async move {
            bytes
                .map(|bytes| ImageBytes::from_bytes(bytes, None))
                .ok_or(InMemoryRepositoryError::NotFound(id))
        }
    }
}
