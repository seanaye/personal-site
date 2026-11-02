use serde::{Deserialize, Serialize};

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
