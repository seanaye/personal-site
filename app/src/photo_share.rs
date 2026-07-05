//! Signed photo share page (`/photo/:token`), streamed the Leptos way.
//!
//! The route uses in-order streaming SSR ([`SsrMode::InOrder`]): everything
//! before the `<Suspense>` (including the document head, so CSS loads early)
//! is flushed immediately, then the stream pauses until the photos have been
//! fetched from Immich. In-order mode never renders the Suspense fallback,
//! so the loading indicator is a plain element ahead of the grid that a
//! `:has()` rule hides once the grid content streams in.
//!
//! Out-of-order streaming cannot be used here: it leaves suspense swap
//! artifacts (`<template>`, `<script>`, marker comments) in the DOM, which
//! desync the islands router's lockstep tree diff on the next navigation.
//!
//! The app crate stays decoupled from the Immich/token-signing machinery: the
//! server injects a [`ssr::SharedPhotosFetcher`] into context, and the server
//! function below simply invokes it.
//!
//! [`SsrMode::InOrder`]: leptos_router::SsrMode::InOrder

use leptos::prelude::*;
use leptos_meta::{Meta, Title};
use leptos_router::{
    hooks::use_params,
    params::{Params, ParamsError, ParamsMap},
};
use photogrid::PhotoLayoutData;

use crate::{components::PhotoGridComponent, LayoutContent};

#[cfg(feature = "ssr")]
pub mod ssr {
    use std::{future::Future, pin::Pin, sync::Arc};

    use photogrid::PhotoLayoutData;

    /// Boxed future returned by [`SharedPhotosFetcher`].
    pub type SharedPhotosFuture =
        Pin<Box<dyn Future<Output = Result<Vec<PhotoLayoutData>, String>> + Send>>;

    /// Server-side port that resolves a share token (plus the request headers,
    /// used to build absolute image URLs) into photogrid layout data.
    ///
    /// Provided via context by the server binary; the error string is shown
    /// to the visitor, so it should not leak internals.
    #[derive(Clone)]
    pub struct SharedPhotosFetcher(
        pub Arc<dyn Fn(String, http::HeaderMap) -> SharedPhotosFuture + Send + Sync>,
    );
}

#[server]
async fn fetch_shared_photos(token: String) -> Result<Vec<PhotoLayoutData>, ServerFnError> {
    let fetcher = use_context::<ssr::SharedPhotosFetcher>()
        .ok_or_else(|| ServerFnError::new("photo sharing is not configured"))?;
    let headers: http::HeaderMap = leptos_axum::extract().await?;

    (fetcher.0)(token, headers).await.map_err(ServerFnError::new)
}

#[derive(Debug, PartialEq, Clone)]
struct ShareParams {
    token: String,
}

impl Params for ShareParams {
    fn from_map(map: &ParamsMap) -> Result<Self, ParamsError> {
        Ok(Self {
            token: map.get_str("token").unwrap_or_default().to_owned(),
        })
    }
}

#[component]
pub fn SharedPhotoPage() -> impl IntoView {
    let params = use_params::<ShareParams>();
    let photos = Resource::new(
        move || {
            params
                .get()
                .map(|params| params.token)
                .unwrap_or_default()
        },
        fetch_shared_photos,
    );

    view! {
        <Title text="shared photos | seanaye.ca" />
        <Meta name="robots" content="noindex" />
        <section class="contents">
            <LayoutContent>
                <style>
                    "#share-content:has(#share-grid) #share-loading { display: none; }"
                </style>
                <div id="share-content">
                    <p id="share-loading" class="font-mono animate-pulse">
                        "loading photos…"
                    </p>
                    <Suspense>
                        {move || Suspend::new(async move {
                            let inner = match photos.await {
                                Ok(data) if data.is_empty() => {
                                    view! {
                                        <p class="font-mono">
                                            "No photos found for this share link."
                                        </p>
                                    }
                                        .into_any()
                                }
                                Ok(data) => {
                                    view! { <PhotoGridComponent data=data /> }.into_any()
                                }
                                Err(_) => {
                                    view! {
                                        <p class="font-mono">
                                            "This share link is invalid or unavailable."
                                        </p>
                                    }
                                        .into_any()
                                }
                            };
                            view! { <div id="share-grid">{inner}</div> }
                        })}
                    </Suspense>
                </div>
            </LayoutContent>
        </section>
    }
}
