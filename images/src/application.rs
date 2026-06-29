use photogrid::PhotoLayoutData;

use crate::{domain::Image, ports::ImageRepository};

/// Application service/use-case layer for image queries.
pub struct ImageService<R> {
    repository: R,
}

impl<R> ImageService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn repository(&self) -> &R {
        &self.repository
    }

    pub fn into_repository(self) -> R {
        self.repository
    }
}

impl<R> ImageService<R>
where
    R: ImageRepository + Sync,
{
    pub async fn images(&self) -> Result<Vec<Image>, R::Error> {
        self.repository.fetch_images().await
    }

    pub async fn photo_layout_data(&self) -> Result<Vec<PhotoLayoutData>, R::Error> {
        self.images()
            .await
            .map(|images| images.into_iter().map(Into::into).collect())
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use async_trait::async_trait;
    use grid::Dimension;
    use url::Url;

    use crate::{
        domain::{Image, ImageId, ImageVariant},
        ports::ImageRepository,
    };

    use super::*;

    struct FakeImageRepository;

    #[async_trait]
    impl ImageRepository for FakeImageRepository {
        type Error = Infallible;

        async fn fetch_images(&self) -> Result<Vec<Image>, Self::Error> {
            Ok(vec![Image::new(
                ImageId::new("hero.jpg"),
                vec![ImageVariant::new(
                    Url::parse("https://cdn.example.com/hero.jpg").unwrap(),
                    Dimension {
                        width: 1200,
                        height: 800,
                    },
                )],
                Default::default(),
            )
            .unwrap()])
        }
    }

    #[tokio::test]
    async fn service_maps_images_to_photo_layout_data() {
        let service = ImageService::new(FakeImageRepository);

        let photos = service.photo_layout_data().await.unwrap();

        assert_eq!(photos.len(), 1);
        assert_eq!(photos[0].srcs.len(), 1);
    }
}
