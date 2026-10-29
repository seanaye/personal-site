use std::collections::HashMap;

use futures::{StreamExt, stream};
use grid::{AspectRatio, Dimension};
use itertools::Itertools;
use list_bucket::{FlattenResult, ListRecursive};
use s3::{
    Region, bucket::Bucket, creds::Credentials, error::S3Error, serde_types::ListBucketResult,
};
use url::Url;

use crate::config::R2Config;

pub fn get_bucket(config: &R2Config) -> Result<Box<Bucket>, S3Error> {
    let credentials = Credentials {
        access_key: Some(config.access_key.clone()),
        secret_key: Some(config.secret_key.clone()),
        security_token: None,
        session_token: None,
        expiration: None,
    };
    Bucket::new(
        config.bucket_name.as_str(),
        Region::R2 {
            account_id: config.account_id.clone(),
        },
        credentials,
    )
    .map(|mut bucket| {
        bucket.set_listobjects_v2();
        bucket
    })
}

pub struct BucketAccess<'a> {
    bucket: Box<Bucket>,
    host: &'a str,
}

#[derive(Debug)]
pub struct ResizedImage {
    pub url: Url,
    pub dimension: Dimension,
    pub metadata: HashMap<String, String>,
}

impl<'a> BucketAccess<'a> {
    pub fn new(bucket: Box<Bucket>, host: &'a str) -> Self {
        Self { bucket, host }
    }

    pub async fn list_resized(&self) -> anyhow::Result<HashMap<String, Vec<ResizedImage>>> {
        let res = self
            .bucket
            .list_recursive("resized/".into(), Some("/".into()))
            .await?;
        let mut objects: Vec<_> = res
            .flatten()
            .filter_map(|c| Some((c.key.split("/").last()?.to_string(), c)))
            .collect();
        objects.sort_unstable_by_key(|(key, _)| key.clone());
        let mut out = HashMap::new();
        for (key, val) in &objects.into_iter().chunk_by(|(key, _)| key.clone()) {
            let resized_images: Vec<_> = stream::iter(val)
                .filter_map(|(_key, c)| async move {
                    let mut host = self.bucket.host();
                    host.push('/');
                    host.push_str(&c.key);
                    host.replace_range(0..0, "https://");
                    let mut url: Url = host.parse().ok()?;
                    url.set_host(Some(self.host)).ok()?;
                    let (mut head, _status) = self.bucket.head_object(c.key).await.ok()?;
                    let mut metadata = std::mem::take(&mut head.metadata)?;
                    let dimension: Dimension = metadata.remove("dimensions")?.parse().ok()?;
                    Some(ResizedImage {
                        url,
                        dimension,
                        metadata,
                    })
                })
                .collect()
                .await;

            out.insert(key.to_string(), resized_images);
        }

        Ok(out)
    }
}
