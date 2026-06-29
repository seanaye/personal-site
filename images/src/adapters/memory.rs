use std::convert::Infallible;

use async_trait::async_trait;

use crate::{domain::Image, ports::ImageRepository};

#[derive(Debug, Clone, Default)]
pub struct InMemoryImageRepository {
    images: Vec<Image>,
}

impl InMemoryImageRepository {
    pub fn new(images: Vec<Image>) -> Self {
        Self { images }
    }
}

#[async_trait]
impl ImageRepository for InMemoryImageRepository {
    type Error = Infallible;

    async fn fetch_images(&self) -> Result<Vec<Image>, Self::Error> {
        Ok(self.images.clone())
    }
}
