use dumbpipe_client::{DumbpipeTunnel, EndpointTicket, TunnelConfig};
use futures::TryStreamExt;
use images::{adapters::ImmichImageRepository, ImageService};
use immich_sdk::{types::AssetMediaSize, Client, Error};
use reqwest::header::{HeaderMap, HeaderValue};
use uuid::Uuid;

const IMMICH_IROH_TICKET: &str = env!("IMMICH_IROH_TICKET");
const IMMICH_API_KEY: &str = env!("IMMICH_API_KEY");
const TEST_ASSET_ID: &str = "0dbe35b0-ea0f-4b49-80ae-40b0716b9aae";
const TEST_IMAGE_OUTPUT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../target/immich-fullsize-thumbnail.jpg"
);

fn authenticated_client(base_url: &str, api_key: &str) -> Client {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(api_key).expect("IMMICH_API_KEY should be a valid header value"),
    );
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .expect("authenticated reqwest client should build");

    Client::new_with_client(base_url, client)
}

#[tokio::test(flavor = "multi_thread")]
async fn can_ping_immich_through_dumbpipe() {
    let ticket: EndpointTicket = IMMICH_IROH_TICKET
        .parse()
        .expect("test Immich dumbpipe ticket should parse");
    let tunnel = DumbpipeTunnel::connect(ticket, TunnelConfig::default())
        .await
        .expect("dumbpipe tunnel should start");

    let client = Client::new(&format!("http://{}/api", tunnel.local_addr()));
    let response = match client.ping_server().await {
        Ok(response) => response.into_inner(),
        Err(Error::UnexpectedResponse(response)) => {
            let status = response.status();
            let body = response
                .text()
                .await
                .expect("error body should be readable");
            panic!("Immich ping failed with {status}: {body}");
        }
        Err(error) => panic!("Immich should respond to /server/ping through dumbpipe: {error:?}"),
    };

    assert_eq!(response.res, "pong");

    tunnel
        .shutdown()
        .await
        .expect("dumbpipe tunnel should shut down cleanly");
}

#[tokio::test(flavor = "multi_thread")]
async fn service_lists_images_and_fetches_bytes_through_dumbpipe() {
    let ticket: EndpointTicket = IMMICH_IROH_TICKET
        .parse()
        .expect("test Immich dumbpipe ticket should parse");
    let repository = ImmichImageRepository::connect(ticket, IMMICH_API_KEY)
        .await
        .expect("Immich repository should connect through dumbpipe");
    let service = ImageService::new(repository);

    let first_page = service
        .list_images(None, 2)
        .await
        .expect("first page of images should list");
    assert!(
        !first_page.images.is_empty(),
        "first page should contain images"
    );
    assert!(
        first_page.images.len() <= 2,
        "first page should respect the limit"
    );

    if let Some(cursor) = first_page.next_cursor.clone() {
        let second_page = service
            .list_images(Some(cursor), 2)
            .await
            .expect("second page of images should list");
        let first_ids: Vec<_> = first_page
            .images
            .iter()
            .map(|image| image.id.clone())
            .collect();
        assert!(
            second_page
                .images
                .iter()
                .all(|image| !first_ids.contains(&image.id)),
            "second page should not repeat first page images"
        );
    }

    let image = &first_page.images[0];
    let bytes = service
        .image_bytes(&image.id)
        .await
        .expect("image bytes should fetch by id");
    assert!(!bytes.bytes.is_empty(), "image bytes should not be empty");

    service
        .into_repository()
        .shutdown()
        .await
        .expect("dumbpipe tunnel should shut down cleanly");
}

#[tokio::test(flavor = "multi_thread")]
async fn can_fetch_fullsize_thumbnail_through_dumbpipe() {
    let ticket: EndpointTicket = IMMICH_IROH_TICKET
        .parse()
        .expect("test Immich dumbpipe ticket should parse");
    let tunnel = DumbpipeTunnel::connect(ticket, TunnelConfig::default())
        .await
        .expect("dumbpipe tunnel should start");

    let asset_id = Uuid::parse_str(TEST_ASSET_ID).expect("test asset id should parse");
    let client = authenticated_client(
        &format!("http://{}/api", tunnel.local_addr()),
        IMMICH_API_KEY,
    );
    let response = match client
        .view_asset(&asset_id, None, None, Some(AssetMediaSize::Fullsize), None)
        .await
    {
        Ok(response) => response,
        Err(Error::UnexpectedResponse(response)) => {
            let status = response.status();
            let body = response
                .text()
                .await
                .expect("error body should be readable");
            panic!("Immich fullsize thumbnail failed with {status}: {body}");
        }
        Err(error) => panic!(
            "Immich should respond to /assets/{TEST_ASSET_ID}/thumbnail?size=fullsize through dumbpipe: {error:?}"
        ),
    };

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let mut stream = response.into_inner_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream
        .try_next()
        .await
        .expect("fullsize thumbnail bytes should stream successfully")
    {
        bytes.extend_from_slice(&chunk);
    }

    assert!(!bytes.is_empty(), "fullsize thumbnail should not be empty");
    assert_eq!(
        bytes.get(..3),
        Some(&[0xff, 0xd8, 0xff][..]),
        "fullsize thumbnail should be JPEG bytes; content-type was {content_type:?}"
    );

    if let Some(parent) = std::path::Path::new(TEST_IMAGE_OUTPUT_PATH).parent() {
        std::fs::create_dir_all(parent).expect("thumbnail output directory should be created");
    }
    std::fs::write(TEST_IMAGE_OUTPUT_PATH, &bytes)
        .expect("fullsize thumbnail should write to disk");
    eprintln!("wrote fullsize thumbnail to {TEST_IMAGE_OUTPUT_PATH}");

    tunnel
        .shutdown()
        .await
        .expect("dumbpipe tunnel should shut down cleanly");
}
