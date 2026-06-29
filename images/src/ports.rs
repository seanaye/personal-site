use async_trait::async_trait;

use crate::domain::Image;

/// Outbound port for whatever storage backend can fetch website images.
#[async_trait]
pub trait ImageRepository {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn fetch_images(&self) -> Result<Vec<Image>, Self::Error>;
}
