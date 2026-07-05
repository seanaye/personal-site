//! Signed photo-sharing routes.
//!
//! - `GET /photo/{token}`: `token` is a signed [`Claims::PhotoRange`]. Renders
//!   an HTML page with every Immich photo taken in that date range, laid out
//!   in the photogrid. Each image `src` is itself a signed URL.
//! - `GET /image/{token}?size={thumbnail|preview|fullsize}`: `token` is a
//!   signed [`Claims::Image`]. Streams the raw image bytes from Immich at the
//!   requested size (default `fullsize`).

use std::{collections::HashMap, convert::Infallible, sync::Arc};

use app::components::PhotoGridComponent;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use chrono::DateTime;
use futures::StreamExt;
use grid::Dimension;
use images::{
    adapters::ImmichImageRepository, Image, ImageDateRange, ImageId, ImageService, ImageSize,
    ImageVariant,
};
use leptos::prelude::*;
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

pub fn router(state: Arc<PhotoShareState>) -> Router {
    Router::new()
        .route("/photo/{token}", get(photo_share_page))
        .route("/image/{token}", get(image_bytes))
        .with_state(state)
}

/// Streamed in two chunks: the document head plus a loading indicator goes
/// out immediately (so the browser fetches CSS and paints while we talk to
/// Immich), then the photogrid replaces the indicator once the data arrives.
async fn photo_share_page(
    State(state): State<Arc<PhotoShareState>>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Response {
    let Ok(Claims::PhotoRange { after, before }) = state.signer.verify(&token) else {
        return (StatusCode::FORBIDDEN, "invalid share token").into_response();
    };

    let range = ImageDateRange::new(
        after.and_then(|secs| DateTime::from_timestamp(secs, 0)),
        before.and_then(|secs| DateTime::from_timestamp(secs, 0)),
    );

    let base_url = match base_url(&state, &headers) {
        Ok(url) => url,
        Err(response) => return response,
    };

    let rest = async move {
        // Headers are already sent by the time this resolves, so failures
        // surface as an inline message rather than an error status.
        let content = match state.service.images_taken_in(range).await {
            Ok(images) => {
                let data: Vec<PhotoLayoutData> = images
                    .iter()
                    .filter_map(|image| photo_layout_data(image, &base_url, &state.signer))
                    .collect();
                render_grid(data)
            }
            Err(error) => {
                log::error!("failed to fetch shared photos from Immich: {error}");
                r#"<p class="font-mono p-6">failed to load photos. please try again.</p>"#
                    .to_owned()
            }
        };

        format!(
            "<style>#share-loading{{display:none}}</style>{content}</main></body></html>"
        )
    };

    let stream = futures::stream::once(std::future::ready(page_head()))
        .chain(futures::stream::once(rest))
        .map(Ok::<String, Infallible>);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from_stream(stream))
        .expect("share page response should build")
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
fn base_url(state: &PhotoShareState, headers: &HeaderMap) -> Result<Url, Response> {
    if let Some(url) = &state.public_base_url {
        return Ok(url.clone());
    }

    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing host header").into_response())?;
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("http");

    Url::parse(&format!("{scheme}://{host}/"))
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid host header").into_response())
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

/// The start of the shared photo page, flushed before any data is fetched.
/// Deliberately leaves `<main>`, `<body>` and `<html>` open; the second
/// stream chunk closes them.
fn page_head() -> String {
    concat!(
        "<!DOCTYPE html><html lang=\"en\"><head>",
        "<meta charset=\"utf-8\">",
        "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">",
        "<meta name=\"robots\" content=\"noindex\">",
        "<link rel=\"stylesheet\" href=\"/pkg/personal_site.css\">",
        "<title>shared photos | seanaye.ca</title>",
        "</head><body><main class=\"relative\">",
        "<p id=\"share-loading\" class=\"font-mono p-6 animate-pulse\">",
        "loading photos\u{2026}",
        "</p>",
    )
    .to_owned()
}

/// Server-side render the photogrid fragment for the shared photo page.
fn render_grid(data: Vec<PhotoLayoutData>) -> String {
    let view = view! {
        <div class="relative z-10 min-h-lvh pt-28 pr-4 pl-6 sm:pt-24 sm:pr-6 sm:pl-24">
            {if data.is_empty() {
                view! {
                    <p class="font-mono">"No photos found for this share link."</p>
                }
                    .into_any()
            } else {
                view! { <PhotoGridComponent data=data /> }.into_any()
            }}
        </div>
    };

    view.to_html()
}
