//! Opaque, HMAC-signed tokens for sharing photos.
//!
//! A token is `base64url(json claims) + "." + base64url(hmac-sha256(json claims))`.
//! Tokens are opaque to clients: they can only be minted and verified by the
//! server holding the secret.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// What a signed token grants access to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Claims {
    /// Access to the photos taken within a unix-timestamp (seconds) range.
    /// `None` bounds are unbounded on that side.
    PhotoRange {
        after: Option<i64>,
        before: Option<i64>,
    },
    /// Access to the bytes of a single image.
    Image { id: String },
}

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("token is malformed")]
    Malformed,
    #[error("token signature is invalid")]
    InvalidSignature,
}

/// Mints and verifies signed tokens with an HMAC-SHA256 secret.
#[derive(Clone)]
pub struct TokenSigner {
    secret: Vec<u8>,
}

impl TokenSigner {
    pub fn new(secret: impl Into<Vec<u8>>) -> Self {
        Self {
            secret: secret.into(),
        }
    }

    fn mac(&self) -> HmacSha256 {
        HmacSha256::new_from_slice(&self.secret).expect("hmac accepts any key length")
    }

    /// Serialize and sign `claims` into an opaque, URL-safe token.
    pub fn sign(&self, claims: &Claims) -> String {
        let payload = serde_json::to_vec(claims).expect("claims serialize to json");
        let mut mac = self.mac();
        mac.update(&payload);
        let signature = mac.finalize().into_bytes();

        format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(&payload),
            URL_SAFE_NO_PAD.encode(signature)
        )
    }

    /// Verify a token's signature and decode its claims.
    pub fn verify(&self, token: &str) -> Result<Claims, TokenError> {
        let (payload, signature) = token.split_once('.').ok_or(TokenError::Malformed)?;
        let payload = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| TokenError::Malformed)?;
        let signature = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| TokenError::Malformed)?;

        let mut mac = self.mac();
        mac.update(&payload);
        mac.verify_slice(&signature)
            .map_err(|_| TokenError::InvalidSignature)?;

        serde_json::from_slice(&payload).map_err(|_| TokenError::Malformed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_claims() {
        let signer = TokenSigner::new("test-secret");
        let claims = Claims::PhotoRange {
            after: Some(1727827200),
            before: Some(1728259200),
        };

        let token = signer.sign(&claims);
        let verified = signer.verify(&token).unwrap();

        assert_eq!(verified, claims);
    }

    #[test]
    fn rejects_tampered_payload() {
        let signer = TokenSigner::new("test-secret");
        let token = signer.sign(&Claims::Image {
            id: "abc".to_owned(),
        });

        let (_, signature) = token.split_once('.').unwrap();
        let forged_payload = URL_SAFE_NO_PAD.encode(r#"{"kind":"image","id":"xyz"}"#);
        let forged = format!("{forged_payload}.{signature}");

        assert!(matches!(
            signer.verify(&forged),
            Err(TokenError::InvalidSignature)
        ));
    }

    #[test]
    fn rejects_other_secret() {
        let signer = TokenSigner::new("secret-a");
        let other = TokenSigner::new("secret-b");
        let token = signer.sign(&Claims::PhotoRange {
            after: None,
            before: None,
        });

        assert!(matches!(
            other.verify(&token),
            Err(TokenError::InvalidSignature)
        ));
    }

    #[test]
    fn rejects_garbage() {
        let signer = TokenSigner::new("secret");
        assert!(signer.verify("not-a-token").is_err());
        assert!(signer.verify("a.b").is_err());
    }
}
