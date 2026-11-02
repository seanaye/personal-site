//! Signed photo-sharing infrastructure.
//!
//! - [`PhotoShareState`] resolves `/photo/{token}` pages into photogrid
//!   layout data with signed image URLs.
//! - `GET /image/{token}?size={thumbnail|preview|fullsize}` streams raw image
//!   bytes from Immich through a Topcoat response body.

use std::{collections::HashMap, sync::Arc};

use app::domain::ports::SharedPhotosFetcher;
use chrono::DateTime;
use futures::StreamExt;
use grid::Dimension;
use http_body::Frame;
use http_body_util::StreamBody;
use images::{
    Image, ImageDateRange, ImageId, ImageService, ImageSize, ImageVariant,
    adapters::ImmichImageRepository,
};
use photogrid::{PhotoLayoutData, SrcSet};
use topcoat::{
    context::{Cx, context},
    router::{
        Body, HeaderValue, StatusCode, error::bad_request, header, path_param, query_params,
        response::Response, route,
    },
};
use url::Url;

use crate::sign::{Claims, TokenSigner};

pub struct PhotoShareState {
    pub service: ImageService<ImmichImageRepository>,
    pub signer: TokenSigner,
    /// Base URL used to build absolute signed image URLs. When `None` it is
    /// derived from the incoming request's `Host` header.
    pub public_base_url: Option<Url>,
}

path_param!(token);

#[query_params]
#[derive(Debug, Default)]
struct ImageQuery {
    size: Option<ImageSize>,
}

fn text_response(status: StatusCode, text: &'static str) -> Response {
    Response::builder()
        .status(status)
        .body(Body::from(text))
        .expect("text response should build")
}

#[route(GET "/image/{token}")]
pub async fn image_bytes(cx: &Cx) -> topcoat::Result<Response> {
    let state: &Arc<PhotoShareState> = context(cx);
    let token = path_param::<Token>(cx);
    let Ok(Claims::Image { id }) = state.signer.verify(token) else {
        return Ok(text_response(StatusCode::FORBIDDEN, "invalid image token"));
    };

    let query = query_params::<ImageQuery>(cx)
        .map_err(|error| bad_request(format!("invalid image query: {error}")))?;
    let size = query.size.unwrap_or_default();

    match state.service.image_bytes(&ImageId::new(id), size).await {
        Ok(image_bytes) => {
            let stream = image_bytes.stream.map(|chunk| chunk.map(Frame::data));
            let body = Body::new(StreamBody::new(stream));

            let mut response = Response::builder().status(StatusCode::OK).header(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
            if let Some(content_type) = &image_bytes.content_type {
                response = response.header(header::CONTENT_TYPE, content_type);
            }
            Ok(response.body(body).expect("image response should build"))
        }
        Err(error) => {
            log::error!("failed to stream image bytes from Immich: {error}");
            Ok(text_response(
                StatusCode::BAD_GATEWAY,
                "failed to fetch image",
            ))
        }
    }
}

impl SharedPhotosFetcher for PhotoShareState {
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
fn base_url(state: &PhotoShareState, headers: &http::HeaderMap) -> Result<Url, String> {
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
