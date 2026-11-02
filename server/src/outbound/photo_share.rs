//! Immich adapter for the shared-photo port, with signed image URLs.

use std::{collections::HashMap, sync::Arc};

use app::domain::ports::SharedPhotosFetcher;
use chrono::DateTime;
use grid::Dimension;
use http::header;
use images::{Image, ImageDateRange, ImageService, ImageVariant, outbound::ImmichImageRepository};
use photogrid::{PhotoLayoutData, SrcSet};
use url::Url;

use crate::domain::{models::Claims, services::sign::TokenSigner};

pub struct ImmichSharedPhotosFetcher {
    pub service: Arc<ImageService<ImmichImageRepository>>,
    pub signer: TokenSigner,
    /// Base URL used to build absolute signed image URLs. When `None` it is
    /// derived from the incoming request's `Host` header.
    pub public_base_url: Option<Url>,
}

impl SharedPhotosFetcher for ImmichSharedPhotosFetcher {
    async fn fetch(
        &self,
        token: String,
        headers: http::HeaderMap,
    ) -> Result<Vec<PhotoLayoutData>, String> {
        let Ok(Claims::PhotoRange { after, before }) = self.signer.verify(&token) else {
            return Err("invalid share token".to_owned());
        };

        let range = ImageDateRange::new(
            after.and_then(|secs| DateTime::from_timestamp(secs, 0)),
            before.and_then(|secs| DateTime::from_timestamp(secs, 0)),
        );

        let base_url = base_url(self, &headers)?;

        let images = self
            .service
            .images_taken_in(range)
            .await
            .map_err(|error| {
                log::error!("failed to fetch shared photos from Immich: {error}");
                "failed to fetch photos".to_owned()
            })?;

        Ok(images
            .iter()
            .filter_map(|image| photo_layout_data(image, &base_url, &self.signer))
            .collect())
    }
}

/// Resolve the absolute base URL that signed image URLs should be joined onto.
fn base_url(state: &ImmichSharedPhotosFetcher, headers: &http::HeaderMap) -> Result<Url, String> {
    if let Some(url) = &state.public_base_url {
        return Ok(url.clone());
    }

    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| "missing host header".to_owned())?;
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("http");

    Url::parse(&format!("{scheme}://{host}/")).map_err(|_| "invalid host header".to_owned())
}

/// Map a domain [`Image`] onto photogrid layout data, replacing direct Immich
/// URLs with signed `/image/{token}` URLs.
fn photo_layout_data(
    image: &Image,
    base_url: &Url,
    signer: &TokenSigner,
) -> Option<PhotoLayoutData> {
    let token = signer.sign(&Claims::Image {
        id: image.id.to_string(),
    });
    let url = base_url.join(&format!("/image/{token}")).ok()?;
    let sized = |size: &str| {
        let mut url = url.clone();
        url.set_query(Some(&format!("size={size}")));
        url
    };
    let preview = sized("preview");
    let mut fullsize = sized("fullsize");
    // Bypass previews previously cached under the original full-size URL.
    fullsize.query_pairs_mut().append_pair("v", "2");

    let srcs: Vec<SrcSet> = image
        .variants
        .iter()
        .map(|variant| SrcSet {
            dimensions: safe_dimensions(variant),
            url: preview.clone(),
        })
        .collect();
    if srcs.is_empty() {
        return None;
    }

    Some(PhotoLayoutData {
        srcs,
        metadata: image
            .metadata
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<HashMap<_, _>>(),
        href: Some(fullsize),
    })
}

/// The grid layout divides by image dimensions, so guard against Immich
/// assets that are missing width/height by falling back to 3:2.
fn safe_dimensions(variant: &ImageVariant) -> Dimension {
    let (width, height) = (variant.dimensions.width, variant.dimensions.height);
    if width == 0 || height == 0 {
        Dimension {
            width: 3,
            height: 2,
        }
    } else {
        Dimension { width, height }
    }
}
