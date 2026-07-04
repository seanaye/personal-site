//! Server-side image library.
//!
//! This crate is organized as a small hexagonal architecture:
//!
//! - [`domain`] contains the core image types.
//! - [`ports`] defines the traits the application depends on.
//! - [`application`] contains use cases built from those ports.
//! - [`adapters`] contains infrastructure implementations.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

pub use application::ImageService;
pub use domain::{
    Image, ImageByteStream, ImageBytes, ImageCursor, ImageDateRange, ImageDimensions, ImageError,
    ImageId, ImageMetadata, ImagePage, ImageVariant,
};
pub use ports::ImageRepository;
