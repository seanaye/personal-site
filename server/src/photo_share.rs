//! Signed photo-sharing infrastructure.
//!
//! - [`shared_photos_fetcher`] backs the Leptos `/photo/:token` route: it is
//!   injected into the Leptos context and invoked by the `fetch_shared_photos`
//!   server function, resolving a signed [`Claims::PhotoRange`] token into
//!   photogrid layout data with signed image URLs.
//! - `GET /image/{token}?size={thumbnail|preview|fullsize}`: `token` is a
//!   signed [`Claims::Image`]. Streams the raw image bytes from Immich at the
//!   requested size (default `fullsize`).

use std::{collections::HashMap, sync::Arc};

use app::photo_share::ssr::SharedPhotosFetcher;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use chrono::DateTime;
use grid::Dimension;
use images::{
    adapters::ImmichImageRepository, Image, ImageDateRange, ImageId, ImageService, ImageSize,
    ImageVariant,
};
use photogrid::{PhotoLayoutData, SrcSet};
use serde::Deserialize;
use url::Url;

use crate::sign::{Claims, TokenSigner};

pub struct PhotoShareState {
    pub service: ImageService<ImmichImageRepository>,
    pub signer: TokenSigner,
    /// Base URL used to build absolute signed image URLs. When `None` it is
    /// derived from the incoming request's `Host` header.
    pub public_base_url: Option<Url>,
}

/// Router for the signed image byte endpoint.
pub fn image_router(state: Arc<PhotoShareState>) -> Router {
    Router::new()
        .route("/image/{token}", get(image_bytes))
        .with_state(state)
}

/// Build the [`SharedPhotosFetcher`] that the Leptos share page consumes via
/// context. Error strings are user-facing; details are logged instead.
pub fn shared_photos_fetcher(state: Arc<PhotoShareState>) -> SharedPhotosFetcher {
    SharedPhotosFetcher(Arc::new(move |token, headers| {
        let state = state.clone();
        Box::pin(async move {
            let Ok(Claims::PhotoRange { after, before }) = state.signer.verify(&token) else {
                return Err("invalid share token".to_owned());
            };

            let range = ImageDateRange::new(
                after.and_then(|secs| DateTime::from_timestamp(secs, 0)),
                before.and_then(|secs| DateTime::from_timestamp(secs, 0)),
            );

            let base_url = base_url(&state, &headers)?;

            let images = state.service.images_taken_in(range).await.map_err(|error| {
                log::error!("failed to fetch shared photos from Immich: {error}");
                "failed to fetch photos".to_owned()
            })?;

            Ok(images
                .iter()
                .filter_map(|image| photo_layout_data(image, &base_url, &state.signer))
                .collect())
        })
    }))
}

#[derive(Debug, Default, Deserialize)]
struct ImageQuery {
    #[serde(default)]
    size: ImageSize,
}

async fn image_bytes(
    State(state): State<Arc<PhotoShareState>>,
    Path(token): Path<String>,
    query: Query<ImageQuery>,
) -> Response {
    let Ok(Claims::Image { id }) = state.signer.verify(&token) else {
        return (StatusCode::FORBIDDEN, "invalid image token").into_response();
    };
    let size = query.size;

    match state.service.image_bytes(&ImageId::new(id), size).await {
        Ok(image_bytes) => {
            // `public` + `immutable` lets shared caches (the Cloudflare CDN)
            // hold the bytes for a year: a given asset+size rendition never
            // changes, tokens are deterministic per image id (stable cache
            // keys), and the signed URL itself is the access credential.
            let mut response = Response::builder()
                .status(StatusCode::OK)
                .header(
                    header::CACHE_CONTROL,
                    "public, max-age=31536000, immutable",
                );
            if let Some(content_type) = &image_bytes.content_type {
                response = response.header(header::CONTENT_TYPE, content_type);
            }
            response
                .body(Body::from_stream(image_bytes.stream))
                .expect("image response should build")
        }
        Err(error) => {
            log::error!("failed to stream image bytes from Immich: {error}");
            (StatusCode::BAD_GATEWAY, "failed to fetch image").into_response()
        }
    }
}

/// Resolve the absolute base URL that signed image URLs should be joined onto.
fn base_url(state: &PhotoShareState, headers: &HeaderMap) -> Result<Url, String> {
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

/// Map a domain [`Image`] onto the photogrid layout data, replacing direct
/// Immich URLs with signed `/image/{token}` URLs. The grid itself loads the
/// lower-quality preview while the photo links out to the fullsize rendition.
fn photo_layout_data(image: &Image, base_url: &Url, signer: &TokenSigner) -> Option<PhotoLayoutData> {
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
    let fullsize = sized("fullsize");

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
