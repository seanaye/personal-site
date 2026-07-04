//! Mint a signed photo-share token for a date range.
//!
//! Usage (run from the repo root so `secrets.json` is found; see
//! `just decrypt-secrets`):
//!   cargo run -p server --example generate_photo_token -- <after_unix> <before_unix>
//!
//! Either bound may be `-` to leave it unbounded. Prints the `/photo/{token}`
//! path to stdout.

#[path = "../src/config.rs"]
#[allow(dead_code)]
mod config;
#[path = "../src/sign.rs"]
#[allow(dead_code)]
mod sign;

use config::AppConfig;
use sign::{Claims, TokenSigner};

fn parse_bound(arg: Option<&str>, name: &str) -> Option<i64> {
    match arg {
        None | Some("-") => None,
        Some(value) => Some(
            value
                .parse()
                .unwrap_or_else(|_| panic!("{name} must be a unix timestamp in seconds or `-`")),
        ),
    }
}

fn main() {
    let app_config = AppConfig::new().expect("failed to load configuration");
    let secret = app_config.inner.photo_share.token_secret.clone();
    assert_ne!(
        secret, "placeholder",
        "photo_share.token_secret is a placeholder; run `just decrypt-secrets` first"
    );

    let args: Vec<String> = std::env::args().skip(1).collect();
    let after = parse_bound(args.first().map(String::as_str), "after");
    let before = parse_bound(args.get(1).map(String::as_str), "before");

    let signer = TokenSigner::new(secret);
    let token = signer.sign(&Claims::PhotoRange { after, before });

    println!("/photo/{token}");
}
