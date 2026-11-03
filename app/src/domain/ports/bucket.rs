use std::path::Path;
use url::Url;

pub trait BucketStorage: Send + Sync + 'static {
    /// Sign an unsigned object URL belonging to the configured bucket.
    fn sign_url(&self, url: &Url) -> impl Future<Output = Result<Url, rootcause::Report>> + Send;

    /// Sign a path relative to the adapter's configured public-file prefix.
    fn get_url(
        &self,
        path: impl AsRef<Path> + Send,
    ) -> impl Future<Output = Result<Url, rootcause::Report>> + Send;
}
