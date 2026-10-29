#![allow(dead_code, unused_imports)]

use bucket::{BucketAccess, get_bucket};
use config::{AppConfig, Config, R2Config};
use grid::{FromSize, RoundedAspectRatio, Size};
use images::{ImageService, adapters::ImmichImageRepository};
use photo_search::PhotoAccess;
use photo_share::PhotoShareState;
use photogrid::{PhotoLayoutData, ResponsivePhotoGrid, SrcSet};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sign::TokenSigner;
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    sync::Arc,
};
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::RouterBuilderDirectoryExt,
};

mod bucket;
mod config;
mod photo_share;
mod sign;
mod static_assets;

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
    let data: Vec<PhotoLayoutData> = cached();
    let photos: Arc<[PhotoLayoutData]> = Arc::from(data);

    let share_state = photo_share_state(&app_config.inner).await;
    let shared_photos_fetcher = share_state.clone().map(photo_share::shared_photos_fetcher);
    let site_root = std::env::var("SITE_ROOT").unwrap_or_else(|_| "target/site".to_owned());

    let mut router = app::router(photos, shared_photos_fetcher)
        .serve_dir("/pkg/{*file}", format!("{site_root}/pkg"))
        .route(static_assets::avatar)
        .route(static_assets::favicon)
        .route(static_assets::noise)
        .route(static_assets::bootstrap);
    if let Some(share_state) = share_state {
        log::info!("photo share enabled at /photo/{{token}} and /image/{{token}}");
        router = router
            .app_context(share_state)
            .route(photo_share::image_bytes);
    }

    let assets = AssetBundle::load()
        .expect("failed to load Topcoat asset bundle; run `topcoat asset bundle -p server`");
    topcoat::start(router.assets(assets).build()).await.unwrap();
}
