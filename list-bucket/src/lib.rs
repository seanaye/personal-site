use futures::{stream, StreamExt};
use s3::{
    error::S3Error,
    serde_types::{ListBucketResult, Object},
    Bucket,
};
use std::future::Future;

pub trait ListRecursive {
    fn list_recursive(
        &self,
        prefix: String,
        delimiter: Option<String>,
    ) -> impl Future<Output = Result<Vec<ListBucketResult>, S3Error>> + Send;
}

impl ListRecursive for Bucket {
    fn list_recursive(
        &self,
        prefix: String,
        delimiter: Option<String>,
    ) -> impl Future<Output = Result<Vec<ListBucketResult>, S3Error>> + Send {
        async move {
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
                    async move { Box::pin(self.list_recursive(prefix, delimiter)).await }
                })
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .try_fold(res, |mut acc, cur| {
                    acc.extend(cur?);
                    Ok(acc)
                })
        }
    }
}

pub trait FlattenResult {
    fn flatten(self) -> impl Iterator<Item = Object>;
}

impl FlattenResult for Vec<ListBucketResult> {
    fn flatten(self) -> impl Iterator<Item = Object> {
        self.into_iter().flat_map(|c| c.contents.into_iter())
    }
}
