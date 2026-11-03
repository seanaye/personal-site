use app::domain::ports::BucketStorage;
use futures::{StreamExt, stream};
use grid::Dimension;
use itertools::Itertools;
use list_bucket::{FlattenResult, ListRecursive};
use percent_encoding::percent_decode_str;
use rootcause::{option_ext::OptionExt, prelude::*};
use s3::{Region, bucket::Bucket, creds::Credentials, error::S3Error};
use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
};
use url::Url;

use crate::config::R2Config;

const PRESIGNED_URL_TTL_SECS: u32 = 60 * 60;

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
    .map(|bucket| {
        let mut bucket = bucket.with_path_style();
        bucket.set_listobjects_v2();
        bucket
    })
}

pub struct BucketAccess {
    bucket: Box<Bucket>,
}

#[derive(Debug)]
pub struct ResizedImage {
    pub url: Url,
    pub dimension: Dimension,
    pub metadata: HashMap<String, String>,
}

impl BucketAccess {
    pub fn new(bucket: Box<Bucket>) -> Self {
        Self { bucket }
    }

    /// Mint a fresh, one-hour S3 GET URL without fetching the object.
    pub async fn get_item(&self, item: &str) -> rootcause::Result<String> {
        (!item.is_empty() && item.split('/').all(|part| !matches!(part, "" | "." | "..")))
            .then_some(())
            .context("object key must be a non-empty relative path")?;

        Ok(self
            .bucket
            .presign_get(item, PRESIGNED_URL_TTL_SECS, None)
            .await
            .context("failed to sign bucket object URL")?)
    }

    pub async fn sign_url(&self, url: &Url) -> rootcause::Result<Url> {
        let base = Url::parse(&self.bucket.url()).context("invalid bucket base URL")?;
        (url.origin() == base.origin()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none())
        .then_some(())
        .context("expected an unsigned URL for the configured bucket")?;

        let prefix = format!("{}/", base.path().trim_end_matches('/'));
        let path = url
            .path()
            .strip_prefix(&prefix)
            .context("object URL is outside the configured bucket")?;
        // The S3 client encodes raw keys; decode the URL once to avoid signing
        // a different key when it contains spaces, Unicode, or literal % signs.
        let key = percent_decode_str(path)
            .decode_utf8()
            .context("object key is not UTF-8")?;
        Ok(self
            .get_item(&key)
            .await?
            .parse()
            .context("invalid signed object URL")?)
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
                    // Persist unsigned S3 URLs; sign only when serving a page.
                    let mut url = Url::parse(&self.bucket.url()).ok()?;
                    url.path_segments_mut().ok()?.extend(c.key.split('/'));
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

    pub fn with_prefix(self, prefix: PathBuf) -> BucketLister {
        BucketLister {
            bucket: self,
            prefix,
        }
    }
}

pub struct BucketLister {
    bucket: BucketAccess,
    prefix: PathBuf,
}

impl BucketStorage for BucketLister {
    async fn sign_url(&self, url: &Url) -> Result<Url, rootcause::Report> {
        self.bucket.sign_url(url).await
    }

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
