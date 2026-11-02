//! Streams signed image requests through a Topcoat response body.

use std::sync::Arc;

use futures::StreamExt;
use http_body::Frame;
use http_body_util::StreamBody;
use images::{ImageId, ImageRepository, ImageService, ImageSize};
use topcoat::{
    context::{Cx, context},
    router::{
        Body, HeaderValue, StatusCode, error::bad_request, header, path_param, query_params,
        response::Response,
    },
};

use crate::domain::{models::Claims, services::sign::TokenSigner};

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

pub async fn image_bytes<R: ImageRepository + Send + Sync + 'static>(
    cx: &Cx,
) -> topcoat::Result<Response> {
    let service: &Arc<ImageService<R>> = context(cx);
    let signer: &TokenSigner = context(cx);
    let token = path_param::<Token>(cx);
    let Ok(Claims::Image { id }) = signer.verify(token) else {
        return Ok(text_response(StatusCode::FORBIDDEN, "invalid image token"));
    };

    let query = query_params::<ImageQuery>(cx)
        .map_err(|error| bad_request(format!("invalid image query: {error}")))?;
    let size = query.size.unwrap_or_default();

    match service.image_bytes(&ImageId::new(id), size).await {
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
