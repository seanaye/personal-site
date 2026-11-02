//! Signed photo-share page served by Topcoat.
//!
//! The server registers a [`SharedPhotosFetcher`] in application context. The
//! async grid component resolves the token directly during SSR, while Topcoat
//! streams the page shell before the Immich request completes.

use std::sync::Arc;
use topcoat::{
    Result,
    context::{Cx, try_context},
    router::{HeaderName, HeaderValue, page, path_param, request::headers},
    view::{View, component, suspense, view},
};

use crate::{
    components::photo_grid_component, domain::ports::SharedPhotosFetcher, layout_content,
};

path_param!(token);

#[page("/photo/{token}")]
pub async fn shared_photo_page<S: SharedPhotosFetcher>(cx: &Cx) -> Result<impl View> {
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
                        shared_photo_grid::<S>(token: token)
                    )
                </div>
            )
        </section>
    })
}

#[component]
async fn shared_photo_grid<S: SharedPhotosFetcher>(cx: &Cx, token: String) -> Result<impl View> {
    let fetcher = try_context::<Arc<S>>(cx);
    let result = match fetcher {
        Some(fetcher) => fetcher.fetch(token, headers(cx).clone()).await,
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
