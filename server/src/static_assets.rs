use topcoat::router::{Body, StatusCode, header, response::Response, route};

fn response(bytes: &'static [u8], content_type: &'static str) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(bytes))
        .expect("static response should build")
}

#[route(GET "/avatar.jpg")]
pub async fn avatar() -> topcoat::Result<Response> {
    Ok(response(
        include_bytes!("../../public/avatar.jpg"),
        "image/jpeg",
    ))
}

#[route(GET "/favicon.ico")]
pub async fn favicon() -> topcoat::Result<Response> {
    Ok(response(
        include_bytes!("../../public/favicon.ico"),
        "image/x-icon",
    ))
}

#[route(GET "/noise.svg")]
pub async fn noise() -> topcoat::Result<Response> {
    Ok(response(
        include_bytes!("../../public/noise.svg"),
        "image/svg+xml",
    ))
}

#[route(GET "/bootstrap.js")]
pub async fn bootstrap() -> topcoat::Result<Response> {
    Ok(response(
        include_bytes!("../../public/bootstrap.js"),
        "text/javascript; charset=utf-8",
    ))
}
