use std::future::Future;

use crate::domain::{ImageBytes, ImageCursor, ImageDateRange, ImageId, ImagePage};

/// Outbound port for whatever storage backend can fetch website images.
pub trait ImageRepository {
    type Error: std::error::Error + Send + Sync + 'static;

    /// List images one page at a time using cursor pagination.
    ///
    /// Pass `None` to fetch the first page, then feed the returned
    /// [`ImagePage::next_cursor`] back in to fetch subsequent pages.
    fn list_images(
        &self,
        cursor: Option<ImageCursor>,
        limit: usize,
    ) -> impl Future<Output = Result<ImagePage, Self::Error>> + Send;

    /// List images taken within `range`, one page at a time using cursor
    /// pagination (same cursor semantics as [`ImageRepository::list_images`]).
    fn list_images_taken_in(
        &self,
        range: ImageDateRange,
        cursor: Option<ImageCursor>,
        limit: usize,
    ) -> impl Future<Output = Result<ImagePage, Self::Error>> + Send;

    /// Fetch the raw bytes for a single image.
    fn fetch_image_bytes(
        &self,
        id: &ImageId,
    ) -> impl Future<Output = Result<ImageBytes, Self::Error>> + Send;
}
