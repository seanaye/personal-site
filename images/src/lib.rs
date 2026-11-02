//! Server-side image library.
//!
//! - [`domain::models`] contains the core image types.
//! - [`domain::ports`] defines the repository contract.
//! - [`domain::services`] contains use cases built from that port.
//! - [`outbound`] contains infrastructure implementations.

pub mod domain;
pub mod outbound;

pub use domain::models::{
    Image, ImageByteStream, ImageBytes, ImageCursor, ImageDateRange, ImageDimensions, ImageError,
    ImageId, ImageMetadata, ImagePage, ImageSize, ImageVariant,
};
pub use domain::ports::ImageRepository;
pub use domain::services::ImageService;
