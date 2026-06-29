use std::collections::HashMap;

use grid::Dimension;
use list_bucket::{FlattenResult, ListRecursive};
use s3::{bucket::Bucket, creds::Credentials, error::S3Error, serde_types::Object, Region};
use url::Url;

use crate::{
    domain::{Image, ImageError, ImageId, ImageMetadata, ImageVariant},
    ports::ImageRepository,
};

/// Configuration for the R2/S3 image adapter.
#[derive(Debug, Clone)]
pub struct R2Config {
    pub bucket_name: String,
    pub account_id: String,
    pub access_key: String,
    pub secret_key: String,
    /// Public CDN host, without a scheme, e.g. `cdn.example.com`.
    pub public_host: String,
    pub prefix: String,
    pub delimiter: Option<String>,
}

impl R2Config {
    pub fn from_env(public_host: impl Into<String>) -> Result<Self, R2ImageRepositoryError> {
        Ok(Self {
            bucket_name: env("R2_BUCKET_NAME")?,
            account_id: env("R2_ACCOUNT_ID")?,
            access_key: env("R2_ACCESS_KEY")?,
            secret_key: env("R2_SECRET_KEY")?,
            public_host: public_host.into(),
            prefix: "resized/".to_string(),
            delimiter: Some("/".to_string()),
        })
    }
}

/// Driven adapter that fetches image variants from Cloudflare R2/S3.
pub struct R2ImageRepository {
    bucket: Box<Bucket>,
    public_host: String,
    prefix: String,
    delimiter: Option<String>,
}

impl R2ImageRepository {
    pub fn from_config(config: R2Config) -> Result<Self, R2ImageRepositoryError> {
        let credentials = Credentials {
            access_key: Some(config.access_key),
            secret_key: Some(config.secret_key),
            security_token: None,
            session_token: None,
            expiration: None,
        };

        let bucket = Bucket::new(
            &config.bucket_name,
            Region::R2 {
                account_id: config.account_id,
            },
            credentials,
        )?;

        Ok(Self::new(
            bucket,
            config.public_host,
            config.prefix,
            config.delimiter,
        ))
    }

    pub fn from_env(public_host: impl Into<String>) -> Result<Self, R2ImageRepositoryError> {
        Self::from_config(R2Config::from_env(public_host)?)
    }

    pub fn new(
        mut bucket: Box<Bucket>,
        public_host: impl Into<String>,
        prefix: impl Into<String>,
        delimiter: Option<String>,
    ) -> Self {
        bucket.set_listobjects_v2();
        Self {
            bucket,
            public_host: public_host.into(),
            prefix: prefix.into(),
            delimiter,
        }
    }

    async fn variant_from_object(
        &self,
        object: Object,
    ) -> Result<ImageVariantWithMetadata, R2ImageRepositoryError> {
        let key = object.key;
        let url = self.public_url(&key)?;
        let (mut head, _status) = self.bucket.head_object(&key).await?;
        let mut metadata = std::mem::take(&mut head.metadata).unwrap_or_default();
        let dimensions_value = metadata
            .remove("dimensions")
            .ok_or_else(|| R2ImageRepositoryError::MissingDimensions { key: key.clone() })?;
        let dimensions = dimensions_value.parse::<Dimension>().map_err(|source| {
            R2ImageRepositoryError::InvalidDimensions {
                key,
                value: dimensions_value,
                reason: source.to_string(),
            }
        })?;

        Ok(ImageVariantWithMetadata {
            variant: ImageVariant::new(url, dimensions),
            metadata,
        })
    }

    fn public_url(&self, key: &str) -> Result<Url, R2ImageRepositoryError> {
        let mut host = self.bucket.host();
        host.push('/');
        host.push_str(key);
        host.insert_str(0, "https://");

        let mut url: Url = host.parse()?;
        url.set_host(Some(&self.public_host))?;
        Ok(url)
    }
}

#[async_trait::async_trait]
impl ImageRepository for R2ImageRepository {
    type Error = R2ImageRepositoryError;

    async fn fetch_images(&self) -> Result<Vec<Image>, Self::Error> {
        let list_results = self
            .bucket
            .list_recursive(self.prefix.clone(), self.delimiter.clone())
            .await?;

        let mut groups: HashMap<ImageId, Vec<Object>> = HashMap::new();
        for object in list_results.flatten() {
            let Some(id) = image_id_from_key(&object.key) else {
                continue;
            };
            groups.entry(id).or_default().push(object);
        }

        let mut ids = groups.keys().cloned().collect::<Vec<_>>();
        ids.sort_unstable_by(|left, right| left.as_str().cmp(right.as_str()));

        let mut images = Vec::with_capacity(ids.len());
        for id in ids {
            let objects = groups.remove(&id).unwrap_or_default();
            let mut metadata: Option<ImageMetadata> = None;
            let mut variants = Vec::with_capacity(objects.len());

            for object in objects {
                let image = self.variant_from_object(object).await?;
                if metadata.is_none() {
                    metadata = Some(image.metadata);
                }
                variants.push(image.variant);
            }

            images.push(Image::new(id, variants, metadata.unwrap_or_default())?);
        }

        Ok(images)
    }
}

struct ImageVariantWithMetadata {
    variant: ImageVariant,
    metadata: ImageMetadata,
}

fn image_id_from_key(key: &str) -> Option<ImageId> {
    key.rsplit('/')
        .next()
        .filter(|file_name| !file_name.is_empty())
        .map(ImageId::new)
}

fn env(name: &'static str) -> Result<String, R2ImageRepositoryError> {
    std::env::var(name)
        .map_err(|source| R2ImageRepositoryError::MissingEnvironment { name, source })
}

#[derive(Debug, thiserror::Error)]
pub enum R2ImageRepositoryError {
    #[error("missing environment variable `{name}`")]
    MissingEnvironment {
        name: &'static str,
        #[source]
        source: std::env::VarError,
    },

    #[error(transparent)]
    S3(#[from] S3Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error("object `{key}` is missing `dimensions` metadata")]
    MissingDimensions { key: String },

    #[error("object `{key}` has invalid dimensions metadata `{value}`: {reason}")]
    InvalidDimensions {
        key: String,
        value: String,
        reason: String,
    },

    #[error(transparent)]
    Image(#[from] ImageError),
}
