use app::*;
use axum::Router;
use bucket::{get_bucket, BucketAccess};
use grid::{FromAspectRatio, RoundedAspectRatio, Size};
use leptos::prelude::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use photogrid::{PhotoLayoutData, ResponsivePhotoGrid, SrcSet};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, fs::File, io::{Read, Write}, sync::Arc};
mod bucket;

async fn photo_data() -> anyhow::Result<impl Iterator<Item = PhotoLayoutData>> {
    let bucket = BucketAccess::new(get_bucket()?, "cdn.seanaye.ca");

    let data = bucket.list_resized().await?;

    Ok(data.into_iter().filter_map(|(_key, mut value)| {
        let first = value.first_mut()?;
        let aspect_ratio = first.dimension.aspect_ratio();
        let metadata = std::mem::take(&mut first.metadata);
        Some(PhotoLayoutData {
            aspect_ratio,
            srcs: value
                .into_iter()
                .map(|c| SrcSet {
                    dimensions: c.dimension,
                    url: c.url,
                })
                .collect(),
            metadata,
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

fn cached<T>() -> T where T: DeserializeOwned {
    let mut f = File::open("data.json").unwrap();
    let mut s = String::default();
    let _ = f.read_to_string(&mut s).unwrap();
    serde_json::from_str(&s).unwrap()
}

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");
    #[cfg(debug_assertions)]
    dotenv::dotenv().unwrap();

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

    // let g = photo_data().await.unwrap().collect::<Vec<_>>();
    // write_to_file(&g);
    let data: Vec<PhotoLayoutData> = cached();
    let photos: Arc<[PhotoLayoutData]> = Arc::from(data);

    // build our application with a route
    let app = Router::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || provide_context(photos.clone()),
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log::info!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
