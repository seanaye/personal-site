//! Immich-backed [`ImageRepository`] reached over an Iroh dumbpipe tunnel.
//!
//! The adapter connects to a remote Immich instance exposed via
//! `dumbpipe listen-tcp` and talks to it with the generated `immich-sdk`
//! client. Cursor pagination maps onto Immich's `/search/metadata` endpoint,
//! whose `nextPage` token is treated as the opaque cursor.

use std::future::Future;

use dumbpipe_client::{DumbpipeTunnel, EndpointTicket, TunnelConfig};
use futures::TryStreamExt;
use immich_sdk::{
    types::{AssetMediaSize, AssetResponseDto, AssetTypeEnum, MetadataSearchDto},
    Client,
};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use url::Url;
use uuid::Uuid;

use crate::{
    domain::{
        Image, ImageBytes, ImageCursor, ImageDimensions, ImageError, ImageId, ImageMetadata,
        ImagePage, ImageVariant,
    },
    ports::ImageRepository,
};

/// [`ImageRepository`] backed by Immich, optionally tunnelled over Iroh.
pub struct ImmichImageRepository {
    client: Client,
    base_url: String,
    /// Keeps the dumbpipe tunnel alive for the lifetime of the repository.
    _tunnel: Option<DumbpipeTunnel>,
}

impl ImmichImageRepository {
    /// Connect to a remote Immich instance through an Iroh dumbpipe tunnel.
    ///
    /// `ticket` is the `dumbpipe listen-tcp` endpoint ticket and `api_key` is
    /// an Immich API key sent as the `x-api-key` header.
    pub async fn connect(
        ticket: EndpointTicket,
        api_key: &str,
    ) -> Result<Self, ImmichRepositoryError> {
        let tunnel = DumbpipeTunnel::connect(ticket, TunnelConfig::default()).await?;
        let base_url = format!("http://{}/api", tunnel.local_addr());
        let mut repository = Self::with_base_url(&base_url, api_key)?;
        repository._tunnel = Some(tunnel);
        Ok(repository)
    }

    /// Talk to an Immich instance that is directly reachable at `base_url`
    /// (including the `/api` prefix).
    pub fn with_base_url(base_url: &str, api_key: &str) -> Result<Self, ImmichRepositoryError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(api_key).map_err(|_| ImmichRepositoryError::InvalidApiKey)?,
        );
        let http_client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client: Client::new_with_client(base_url, http_client),
            base_url: base_url.trim_end_matches('/').to_owned(),
            _tunnel: None,
        })
    }

    /// Shut down the underlying dumbpipe tunnel, if any.
    pub async fn shutdown(mut self) -> Result<(), ImmichRepositoryError> {
        if let Some(tunnel) = self._tunnel.take() {
            tunnel.shutdown().await?;
        }
        Ok(())
    }

    fn asset_to_image(&self, asset: &AssetResponseDto) -> Result<Image, ImmichRepositoryError> {
        let url = Url::parse(&format!(
            "{}/assets/{}/thumbnail?size=fullsize",
            self.base_url, asset.id
        ))?;
        let dimensions = ImageDimensions {
            width: asset.width.unwrap_or_default() as usize,
            height: asset.height.unwrap_or_default() as usize,
        };

        let mut metadata = ImageMetadata::new();
        metadata.insert(
            "originalFileName".to_owned(),
            asset.original_file_name.clone(),
        );
        metadata.insert("createdAt".to_owned(), asset.created_at.to_rfc3339());
        if let Some(mime_type) = &asset.original_mime_type {
            metadata.insert("originalMimeType".to_owned(), mime_type.clone());
        }
        if let Some(thumbhash) = &asset.thumbhash {
            metadata.insert("thumbhash".to_owned(), thumbhash.clone());
        }

        Ok(Image::new(
            ImageId::new(asset.id.clone()),
            vec![ImageVariant::new(url, dimensions)],
            metadata,
        )?)
    }
}

impl ImageRepository for ImmichImageRepository {
    type Error = ImmichRepositoryError;

    fn list_images(
        &self,
        cursor: Option<ImageCursor>,
        limit: usize,
    ) -> impl Future<Output = Result<ImagePage, Self::Error>> + Send {
        async move {
            let page = match &cursor {
                None => None,
                Some(cursor) => Some(cursor.as_str().parse::<f64>().map_err(|_| {
                    ImmichRepositoryError::InvalidCursor(cursor.clone())
                })?),
            };

            let body = MetadataSearchDto {
                page,
                size: Some(limit as f64),
                type_: Some(AssetTypeEnum::Image),
                with_exif: Some(true),
                ..Default::default()
            };

            let response = self
                .client
                .search_assets(&body)
                .await
                .map_err(ImmichRepositoryError::api)?
                .into_inner();

            let images = response
                .assets
                .items
                .iter()
                .map(|asset| self.asset_to_image(asset))
                .collect::<Result<Vec<_>, _>>()?;
            let next_cursor = response.assets.next_page.map(ImageCursor::new);

            Ok(ImagePage {
                images,
                next_cursor,
            })
        }
    }

    fn fetch_image_bytes(
        &self,
        id: &ImageId,
    ) -> impl Future<Output = Result<ImageBytes, Self::Error>> + Send {
        let asset_id = Uuid::parse_str(id.as_str())
            .map_err(|_| ImmichRepositoryError::InvalidImageId(id.clone()));

        async move {
            let asset_id = asset_id?;
            let response = self
                .client
                .view_asset(&asset_id, None, None, Some(AssetMediaSize::Fullsize), None)
                .await
                .map_err(ImmichRepositoryError::api)?;

            let content_type = response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);

            let mut stream = response.into_inner_stream();
            let mut bytes = Vec::new();
            while let Some(chunk) = stream
                .try_next()
                .await
                .map_err(ImmichRepositoryError::Stream)?
            {
                bytes.extend_from_slice(&chunk);
            }

            Ok(ImageBytes {
                bytes,
                content_type,
            })
        }
    }
}

/// Errors produced by [`ImmichImageRepository`].
#[derive(Debug, thiserror::Error)]
pub enum ImmichRepositoryError {
    #[error(transparent)]
    Tunnel(#[from] dumbpipe_client::Error),

    #[error("Immich API key is not a valid header value")]
    InvalidApiKey,

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error("Immich API error: {0}")]
    Api(String),

    #[error("failed to stream image bytes: {0}")]
    Stream(reqwest::Error),

    #[error("invalid pagination cursor `{0}`")]
    InvalidCursor(ImageCursor),

    #[error("image id `{0}` is not a valid UUID")]
    InvalidImageId(ImageId),

    #[error(transparent)]
    InvalidUrl(#[from] url::ParseError),

    #[error(transparent)]
    Domain(#[from] ImageError),
}

impl ImmichRepositoryError {
    fn api(error: immich_sdk::Error<()>) -> Self {
        Self::Api(error.to_string())
    }
}
