use std::{collections::HashMap, future::Future};

use crate::{
    domain::{Image, ImageBytes, ImageCursor, ImageId, ImagePage},
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
        async move {
            let offset = match cursor {
                None => 0,
                Some(cursor) => cursor
                    .as_str()
                    .parse::<usize>()
                    .map_err(|_| InMemoryRepositoryError::InvalidCursor(cursor))?,
            };

            let images: Vec<Image> = self.images.iter().skip(offset).take(limit).cloned().collect();
            let end = offset.saturating_add(images.len());
            let next_cursor =
                (end < self.images.len()).then(|| ImageCursor::new(end.to_string()));

            Ok(ImagePage {
                images,
                next_cursor,
            })
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
