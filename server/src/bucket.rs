use std::collections::HashMap;

use itertools::Itertools;
use url::Url;
use futures::{stream, Stream, StreamExt};
use s3::{bucket::Bucket, creds::Credentials, error::S3Error, Region};

pub fn get_bucket() -> Result<Box<Bucket>, S3Error> {

    let key = std::env::var("R2_ACCESS_KEY").unwrap();
    let secret = std::env::var("R2_SECRET_KEY").unwrap();
    let credentials = Credentials {
        access_key: Some(key),
        secret_key: Some(secret),
        security_token: None,
        session_token: None,
        expiration: None,
    };
    Bucket::new(
        std::env::var("R2_BUCKET_NAME").unwrap().as_str(),
        Region::R2 {
            account_id: std::env::var("R2_ACCOUNT_ID").unwrap(),
        },
        credentials,
    )
}

pub struct BucketAccess {
    bucket: Box<Bucket>
}



pub struct ResizedImage {
    pub url: Url,
    pub aspect_ratio: String,
}

impl BucketAccess {
    pub fn new(bucket: Box<Bucket>) -> Self {
        Self { bucket }
    }

    pub async fn list_resized(&self) -> anyhow::Result<HashMap<String, Vec<ResizedImage>>> {
        let res = self.bucket.list("resized/".into(), Some("/".into())).await?;
        let mut objects: Vec<_> = res.into_iter().flat_map(|c| c.contents.into_iter()).filter_map(|c| Some((c.key.split("/").last()?.to_string(), c))).collect();
        objects.sort_unstable_by_key(|(key, _)| key.clone());
        let mut out = HashMap::new();
        for (key, val) in &objects.into_iter().chunk_by(|(key, _)| key.clone()) {
            let resized_images: Vec<_> = stream::iter(val).filter_map(|(_key, c)| async move {
                let mut host = self.bucket.host();
                host.push_str(&c.key);
                let url: Url = host.parse().ok()?;
                let (head, _status) = self.bucket.head_object(c.key).await.ok()?;
                let metadata = &mut head.metadata?;
                let aspect_ratio = metadata.get_mut("aspect-ratio")?;
                Some(ResizedImage {
                    url,
                    aspect_ratio: std::mem::take(aspect_ratio)
                })
            }).collect().await;

            out.insert(key.to_string(), resized_images);

            
        }

        Ok(out)
        
    }
}


