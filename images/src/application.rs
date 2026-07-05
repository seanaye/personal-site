use crate::{
    domain::{Image, ImageBytes, ImageCursor, ImageDateRange, ImageId, ImagePage, ImageSize},
    ports::ImageRepository,
};

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
    /// List images one page at a time using cursor pagination.
    ///
    /// Pass `None` to fetch the first page, then feed the returned
    /// [`ImagePage::next_cursor`] back in for subsequent pages.
    pub async fn list_images(
        &self,
        cursor: Option<ImageCursor>,
        limit: usize,
    ) -> Result<ImagePage, R::Error> {
        self.repository.list_images(cursor, limit).await
    }

    /// Fetch every image taken within `range`, walking all pages.
    pub async fn images_taken_in(&self, range: ImageDateRange) -> Result<Vec<Image>, R::Error> {
        // Each page is a full round trip to the backing store (potentially
        // through a tunnel), so prefer few large pages. Immich caps search
        // page size at 1000.
        const PAGE_SIZE: usize = 500;

        let mut images = Vec::new();
        let mut cursor = None;
        loop {
            let page = self
                .repository
                .list_images_taken_in(range, cursor, PAGE_SIZE)
                .await?;
            images.extend(page.images);
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }

        Ok(images)
    }

    /// Fetch the raw bytes for a single image by id at the requested size.
    pub async fn image_bytes(&self, id: &ImageId, size: ImageSize) -> Result<ImageBytes, R::Error> {
        self.repository.fetch_image_bytes(id, size).await
    }
}

#[cfg(test)]
mod tests {
    use url::Url;

    use crate::{
        adapters::memory::InMemoryImageRepository,
        domain::{Image, ImageDimensions, ImageId, ImageVariant},
    };

    use super::*;

    fn image(id: &str) -> Image {
        Image::new(
            ImageId::new(id),
            vec![ImageVariant::new(
                Url::parse(&format!("https://cdn.example.com/{id}")).unwrap(),
                ImageDimensions {
                    width: 1200,
                    height: 800,
                },
            )],
            Default::default(),
        )
        .unwrap()
    }

    fn service() -> ImageService<InMemoryImageRepository> {
        let repository = InMemoryImageRepository::new(vec![
            (image("a.jpg"), vec![0xff, 0xd8, 0xff]),
            (image("b.jpg"), vec![0x01]),
            (image("c.jpg"), vec![0x02]),
        ]);
        ImageService::new(repository)
    }

    #[tokio::test]
    async fn service_lists_images_with_cursor_pagination() {
        let service = service();

        let first_page = service.list_images(None, 2).await.unwrap();
        assert_eq!(first_page.images.len(), 2);
        assert_eq!(first_page.images[0].id.as_str(), "a.jpg");
        assert_eq!(first_page.images[1].id.as_str(), "b.jpg");
        let cursor = first_page.next_cursor.expect("more images should remain");

        let second_page = service.list_images(Some(cursor), 2).await.unwrap();
        assert_eq!(second_page.images.len(), 1);
        assert_eq!(second_page.images[0].id.as_str(), "c.jpg");
        assert!(second_page.next_cursor.is_none());
    }

    #[tokio::test]
    async fn service_lists_images_taken_in_date_range() {
        let mut a = image("a.jpg");
        a.metadata
            .insert("createdAt".to_owned(), "2024-01-01T00:00:00Z".to_owned());
        let mut b = image("b.jpg");
        b.metadata
            .insert("createdAt".to_owned(), "2024-06-01T00:00:00Z".to_owned());
        let mut c = image("c.jpg");
        c.metadata
            .insert("createdAt".to_owned(), "2024-12-01T00:00:00Z".to_owned());
        let repository = InMemoryImageRepository::new(vec![
            (a, vec![0x00]),
            (b, vec![0x01]),
            (c, vec![0x02]),
        ]);
        let service = ImageService::new(repository);

        let range = ImageDateRange::new(
            Some("2024-03-01T00:00:00Z".parse().unwrap()),
            Some("2024-09-01T00:00:00Z".parse().unwrap()),
        );
        let images = service.images_taken_in(range).await.unwrap();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].id.as_str(), "b.jpg");
    }

    #[tokio::test]
    async fn service_fetches_image_bytes_as_stream() {
        let service = service();

        let image_bytes = service
            .image_bytes(&ImageId::new("a.jpg"), ImageSize::default())
            .await
            .unwrap();
        let bytes = image_bytes.into_bytes().await.unwrap();

        assert_eq!(bytes.as_ref(), &[0xff, 0xd8, 0xff]);
    }

    #[tokio::test]
    async fn service_errors_on_unknown_image() {
        let service = service();

        let result = service
            .image_bytes(&ImageId::new("missing.jpg"), ImageSize::default())
            .await;

        assert!(result.is_err());
    }
}
