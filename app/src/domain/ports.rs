use std::path::Path;
use url::Url;

pub trait BucketStorage: Send + Sync + 'static {
    fn get_url(
        &self,
        path: impl AsRef<Path> + Send,
    ) -> impl Future<Output = Result<Url, rootcause::Report>> + Send;
}
