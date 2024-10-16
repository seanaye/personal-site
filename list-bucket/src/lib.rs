use futures::{stream, StreamExt};
use async_recursion::async_recursion;
use s3::{error::S3Error, serde_types::{ListBucketResult, Object}, Bucket};

#[async_trait::async_trait]
pub trait ListRecursive {
    async fn list_recursive(&self, prefix: String, delimiter: Option<String>) -> Result<Vec<ListBucketResult>, S3Error>;
}

impl ListRecursive for Bucket {
    #[async_recursion]
    async fn list_recursive(&self, prefix: String, delimiter: Option<String>) -> Result<Vec<ListBucketResult>, S3Error> {
        let mut res = self.list(prefix, delimiter.clone()).await?;

        let common: Vec<_> = res
            .iter_mut()
            .flat_map(|r| {
                r.common_prefixes
                    .iter_mut()
                    .flat_map(|v| v.iter_mut())
                    .map(|v| std::mem::take(&mut v.prefix))
            })
            .collect();
        stream::iter(common)
            .then(|prefix| {
                let delimiter = delimiter.clone();
                async move { self.list_recursive(prefix, delimiter).await} })
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .try_fold(res, |mut acc, cur| {
                acc.extend(cur?);
                Ok(acc)
            })
    }
}

pub trait FlattenResult {
    fn flatten(self) -> impl Iterator<Item = Object>;
}

impl FlattenResult for Vec<ListBucketResult> {
    fn flatten(self) -> impl Iterator<Item = Object> {
        self
            .into_iter()
            .flat_map(|c| c.contents.into_iter())
    }
}
