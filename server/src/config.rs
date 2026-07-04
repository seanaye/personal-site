//! Typed application configuration, following the komake pattern.
//!
//! Configuration is layered (later sources override earlier ones):
//!
//! 1. `secrets.example.json` — committed placeholders (required in debug)
//! 2. `secrets.json` — plaintext secrets, gitignored (required in release)
//! 3. Environment variables (`_` separator, e.g. `RUST_LOG` style keys)
//!
//! `secrets.json` is produced by decrypting the committed `secrets.enc.json`
//! with sops (`just decrypt-secrets` locally; the container entrypoint does it
//! at boot using the `SOPS_AGE_KEY` secret).

use std::sync::Arc;

use config::File;
use serde::Deserialize;
use url::Url;

#[derive(Clone)]
pub struct AppConfig {
    pub inner: Arc<Config>,
}

impl AppConfig {
    pub fn new() -> Result<Self, config::ConfigError> {
        let s = config::Config::builder()
            .add_source(File::with_name("secrets.example").required(cfg!(debug_assertions)))
            .add_source(File::with_name("secrets").required(!cfg!(debug_assertions)))
            .add_source(config::Environment::default().separator("_"))
            .build()?;

        Ok(AppConfig {
            inner: Arc::new(s.try_deserialize()?),
        })
    }
}

#[derive(Deserialize)]
pub struct Config {
    /// Absolute base URL for building signed links. When `None` it is derived
    /// from the incoming request's `Host` header.
    pub public_base_url: Option<Url>,
    pub photo_share: PhotoShareConfig,
    pub r2: R2Config,
}

#[derive(Deserialize)]
pub struct PhotoShareConfig {
    /// HMAC secret used to sign photo share and image tokens.
    pub token_secret: String,
    pub immich: ImmichConfig,
}

#[derive(Deserialize)]
pub struct ImmichConfig {
    /// Immich API key, sent as the `x-api-key` header.
    pub api_key: String,
    /// Direct base URL to an Immich instance (including the `/api` prefix).
    /// Takes precedence over `iroh_ticket` when both are set.
    pub base_url: Option<String>,
    /// `dumbpipe listen-tcp` endpoint ticket for tunnelled access.
    pub iroh_ticket: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct R2Config {
    pub access_key: String,
    pub secret_key: String,
    pub bucket_name: String,
    pub account_id: String,
}
