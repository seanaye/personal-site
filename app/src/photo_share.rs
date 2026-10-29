//! Signed photo-share page served by Topcoat.
//!
//! The server registers a [`SharedPhotosFetcher`] in application context. The
//! async grid component resolves the token directly during SSR, while Topcoat
//! streams the page shell before the Immich request completes.

use std::{future::Future, pin::Pin, sync::Arc};

use photogrid::PhotoLayoutData;
use topcoat::{
    Result,
    context::{Cx, try_app_context},
    router::{HeaderName, HeaderValue, page, path_param, request::headers},
    view::{View, component, suspense, view},
};

use crate::{components::photo_grid_component, layout_content};

pub type SharedPhotosFuture =
    Pin<Box<dyn Future<Output = Result<Vec<PhotoLayoutData>, String>> + Send>>;

/// Server-side port that resolves a share token and request headers into
/// photogrid layout data. Error strings are logged by the server; visitors see
/// a generic unavailable message.
#[derive(Clone)]
pub struct SharedPhotosFetcher(
    pub Arc<dyn Fn(String, http::HeaderMap) -> SharedPhotosFuture + Send + Sync>,
);

path_param!(token);

#[page("/photo/{token}")]
pub async fn shared_photo_page(cx: &Cx) -> Result<impl View> {
    let token = path_param::<Token>(cx).to_owned();

    Ok(view! {
        ((HeaderName::from_static("x-robots-tag"), HeaderValue::from_static("noindex")))
        <section class="contents">
            layout_content(
                <style>"#share-content:has(#share-grid) #share-loading { display: none; }"</style>
                <div id="share-content">
                    <p id="share-loading" class="font-mono animate-pulse">
                        "loading photos…"
                    </p>
                    suspense(
                        fallback: view! { "" },
                        shared_photo_grid(token: token)
                    )
                </div>
            )
        </section>
    })
}

#[component]
async fn shared_photo_grid(cx: &Cx, token: String) -> Result<impl View> {
    let fetcher = try_app_context::<SharedPhotosFetcher>(cx).cloned();
    let result = match fetcher {
        Some(fetcher) => (fetcher.0)(token, headers(cx).clone()).await,
        None => Err("photo sharing is not configured".to_owned()),
    };

    Ok(view! {
        <div id="share-grid">
            match result {
                Ok(data) => {
                    if data.is_empty() {
                        <p class="font-mono">"No photos found for this share link."</p>
                    } else {
                        photo_grid_component(data: data)
                    }
                },
                Err(_) => {
                    <p class="font-mono">"This share link is invalid or unavailable."</p>
                },
            }
        </div>
    })
}
