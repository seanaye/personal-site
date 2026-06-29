use std::future::Future;

use crate::domain::Image;

/// Outbound port for whatever storage backend can fetch website images.
pub trait ImageRepository {
    type Error: std::error::Error + Send + Sync + 'static;

    fn fetch_images(&self) -> impl Future<Output = Result<Vec<Image>, Self::Error>> + Send;
}
