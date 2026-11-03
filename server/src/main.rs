use images::{ImageService, outbound::ImmichImageRepository};
use photogrid::PhotoLayoutData;
use serde::de::DeserializeOwned;
use server::config::{AppConfig, Config};
use server::domain::services::sign::TokenSigner;
use server::inbound;
use server::outbound::{
    bucket::{BucketAccess, get_bucket},
    photo_share::ImmichSharedPhotosFetcher,
};
use std::{path::PathBuf, sync::Arc};
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::RouterBuilderDirectoryExt,
};

/// Build the photo-share state from the app configuration.
///
/// Returns `None` (and logs why) when Immich is unreachable or the config
/// still holds placeholder values (e.g. local dev without `secrets.json`).
async fn photo_share_state(config: &Config) -> Option<Arc<ImmichSharedPhotosFetcher>> {
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

    Some(Arc::new(ImmichSharedPhotosFetcher {
        service: Arc::new(ImageService::new(repository)),
        signer: TokenSigner::new(photo_share.token_secret.clone()),
        public_base_url: config.public_base_url.clone(),
    }))
}

fn cached<T>() -> T
where
    T: DeserializeOwned,
{
    let s = include_str!("../../data.json");
    serde_json::from_str(s).unwrap()
}

#[topcoat::router::route(GET "/image/{token}")]
async fn image_bytes(
    cx: &topcoat::context::Cx,
) -> topcoat::Result<topcoat::router::response::Response> {
    inbound::images::image_bytes::<ImmichImageRepository>(cx).await
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
    let site_root = std::env::var("SITE_ROOT").unwrap_or_else(|_| "target/site".to_owned());
    let bucket = BucketAccess::new(get_bucket(&app_config.inner.r2).expect("failed to get bucket"));

    let mut router = app::web::router(
        photos,
        share_state.clone(),
        bucket.with_prefix(PathBuf::from("public")),
    )
    .serve_dir("/pkg/{*file}", format!("{site_root}/pkg"));

    if let Some(share_state) = share_state {
        log::info!("photo share enabled at /photo/{{token}} and /image/{{token}}");
        router = router
            .with(share_state.service.clone())
            .with(share_state.signer.clone())
            .route(image_bytes);
    }

    let assets = AssetBundle::load()
        .expect("failed to load Topcoat asset bundle; run `topcoat asset bundle -p server`");
    topcoat::start(router.assets(assets).build()).await.unwrap();
}
