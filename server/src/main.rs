#![allow(dead_code, unused_imports)]

use app::*;
use axum::Router;
use bucket::{get_bucket, BucketAccess};
use config::{AppConfig, Config, R2Config};
use grid::{FromSize, RoundedAspectRatio, Size};
use leptos::prelude::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use photo_search::PhotoAccess;
use photogrid::{PhotoLayoutData, ResponsivePhotoGrid, SrcSet};
use images::{adapters::ImmichImageRepository, ImageService};
use photo_share::PhotoShareState;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sign::TokenSigner;
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    sync::Arc,
};
mod bucket;
mod config;
mod photo_share;
mod sign;

/// Build the photo-share state from the app configuration.
///
/// Returns `None` (and logs why) when Immich is unreachable or the config
/// still holds placeholder values (e.g. local dev without `secrets.json`).
async fn photo_share_state(config: &Config) -> Option<Arc<PhotoShareState>> {
    let photo_share = &config.photo_share;
    if photo_share.token_secret.is_empty() || photo_share.token_secret == "placeholder" {
        log::warn!("photo_share.token_secret is a placeholder; photo share routes disabled");
        return None;
    }

    let immich = &photo_share.immich;
    let repository = if let Some(base_url) = &immich.base_url {
        ImmichImageRepository::with_base_url(base_url, &immich.api_key)
            .inspect_err(|error| log::error!("failed to build Immich client: {error}"))
            .ok()?
    } else if let Some(ticket) = &immich.iroh_ticket {
        let ticket = ticket
            .parse()
            .inspect_err(|error| log::error!("invalid photo_share.immich.iroh_ticket: {error}"))
            .ok()?;
        ImmichImageRepository::connect(ticket, &immich.api_key)
            .await
            .inspect_err(|error| log::error!("failed to connect Immich dumbpipe tunnel: {error}"))
            .ok()?
    } else {
        log::warn!(
            "neither photo_share.immich.base_url nor photo_share.immich.iroh_ticket set; photo share routes disabled"
        );
        return None;
    };

    Some(Arc::new(PhotoShareState {
        service: ImageService::new(repository),
        signer: TokenSigner::new(photo_share.token_secret.clone()),
        public_base_url: config.public_base_url.clone(),
    }))
}

async fn photo_data(r2: &R2Config) -> anyhow::Result<impl Iterator<Item = PhotoLayoutData>> {
    let bucket = BucketAccess::new(get_bucket(r2)?, "cdn.seanaye.ca");

    let data = bucket.list_resized().await?;

    Ok(data.into_values().filter_map(|mut value| {
        let first = value.first_mut()?;
        let metadata = std::mem::take(&mut first.metadata);
        Some(PhotoLayoutData {
            srcs: value
                .into_iter()
                .map(|c| SrcSet {
                    dimensions: c.dimension,
                    url: c.url,
                })
                .collect(),
            metadata,
            href: None,
        })
    }))
}

fn write_to_file<T>(data: &T)
where
    T: Serialize,
{
    let mut f = File::create("data.json").unwrap();
    let s = serde_json::to_string(data).unwrap();
    f.write_all(s.as_bytes()).unwrap();
}

fn cached<T>() -> T
where
    T: DeserializeOwned,
{
    let s = include_str!("../../data.json");
    serde_json::from_str(s).unwrap()
}

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");
    #[cfg(debug_assertions)]
    let _ = dotenv::dotenv();

    let app_config = AppConfig::new().expect("failed to load configuration");

    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    let conf = get_configuration(if cfg!(debug_assertions) {
        None
    } else {
        Some("Cargo.toml")
    })
    .unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);
    // a 1727827200
    // b 1728259200

    // let mut g = photo_data().await.unwrap().collect::<Vec<_>>();
    // g.sort_unstable_by_key(|item| {
    //     item.get_timestamp()
    // });
    // write_to_file(&g);
    let data: Vec<PhotoLayoutData> = cached();

    let photos: Arc<[PhotoLayoutData]> = Arc::from(data);

    let share_state = photo_share_state(&app_config.inner).await;
    // Injected into the Leptos context so the `/photo/:token` route's server
    // function can resolve share tokens (see `app::photo_share`).
    let shared_photos_fetcher = share_state
        .clone()
        .map(photo_share::shared_photos_fetcher);

    // build our application with a route
    let mut app = Router::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || {
                provide_context(photos.clone());
                if let Some(fetcher) = shared_photos_fetcher.clone() {
                    provide_context(fetcher);
                }
            },
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    if let Some(share_state) = share_state {
        log::info!("photo share enabled at /photo/{{token}} and /image/{{token}}");
        app = app.merge(photo_share::image_router(share_state));
    }

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log::info!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
