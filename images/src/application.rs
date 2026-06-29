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
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use url::Url;

    use crate::{
        domain::{Image, ImageDimensions, ImageId, ImageVariant},
        ports::ImageRepository,
    };

    use super::*;

    struct FakeImageRepository;

    impl ImageRepository for FakeImageRepository {
        type Error = Infallible;

        fn fetch_images(
            &self,
        ) -> impl std::future::Future<Output = Result<Vec<Image>, Self::Error>> + Send {
            async {
                Ok(vec![Image::new(
                    ImageId::new("hero.jpg"),
                    vec![ImageVariant::new(
                        Url::parse("https://cdn.example.com/hero.jpg").unwrap(),
                        ImageDimensions {
                            width: 1200,
                            height: 800,
                        },
                    )],
                    Default::default(),
                )
                .unwrap()])
            }
        }
    }

    #[tokio::test]
    async fn service_fetches_images() {
        let service = ImageService::new(FakeImageRepository);

        let images = service.images().await.unwrap();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].variants.len(), 1);
    }
}
