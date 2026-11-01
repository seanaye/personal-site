use app::domain::ports::BucketStorage;
use futures::{StreamExt, stream};
use grid::Dimension;
use itertools::Itertools;
use list_bucket::{FlattenResult, ListRecursive};
use rootcause::{option_ext::OptionExt, prelude::*};
use s3::{Region, bucket::Bucket, creds::Credentials, error::S3Error};
use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
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

    /// Public object URL; signing credentials are not used for CDN downloads.
    pub async fn get_item(&self, item: &str) -> rootcause::Result<String> {
        let mut url =
            Url::parse(&format!("https://{}/", self.host)).context("invalid public bucket host")?;
        // Treat keys as path segments, not URL syntax. Spaces, %, ?, and #
        // remain part of the object key rather than changing the destination.
        url.path_segments_mut()
            .ok()
            .context("invalid public bucket base URL")?
            .clear()
            .extend(item.split('/'));
        Ok(url.into())
    }

    pub async fn list_resized(&self) -> rootcause::Result<HashMap<String, Vec<ResizedImage>>> {
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

    pub fn with_prefix(self, prefix: PathBuf) -> BucketLister<'a> {
        BucketLister {
            bucket: self,
            prefix,
        }
    }
}

pub struct BucketLister<'a> {
    bucket: BucketAccess<'a>,
    prefix: PathBuf,
}

impl BucketStorage for BucketLister<'static> {
    async fn get_url(&self, path: impl AsRef<Path> + Send) -> Result<Url, rootcause::Report> {
        let path = path.as_ref();
        (!path.as_os_str().is_empty()
            && path
                .components()
                .all(|part| matches!(part, Component::Normal(_))))
        .then_some(())
        .context("file path must be relative and stay within the public prefix")?;

        let mut p = self.prefix.clone();
        p.push(path);
        Ok(self
            .bucket
            .get_item(p.to_str().context("invalid path")?)
            .await?
            .parse()
            .context("invalid url")?)
    }
}
