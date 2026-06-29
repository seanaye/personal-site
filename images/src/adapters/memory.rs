use std::{convert::Infallible, future::Future};

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

impl ImageRepository for InMemoryImageRepository {
    type Error = Infallible;

    fn fetch_images(&self) -> impl Future<Output = Result<Vec<Image>, Self::Error>> + Send {
        let images = self.images.clone();
        async move { Ok(images) }
    }
}
