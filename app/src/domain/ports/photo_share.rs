use photogrid::PhotoLayoutData;

/// Resolves a share token and request headers into photogrid layout data.
/// Internal errors are logged by the adapter; visitors see a generic message.
pub trait SharedPhotosFetcher: Send + Sync + 'static {
    fn fetch(
        &self,
        token: String,
        headers: http::HeaderMap,
    ) -> impl Future<Output = Result<Vec<PhotoLayoutData>, String>> + Send;
}
