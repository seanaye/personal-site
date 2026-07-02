//! Adapters connect the hexagonal ports to concrete infrastructure.

pub mod immich;
pub mod memory;

pub use immich::{ImmichImageRepository, ImmichRepositoryError};
pub use memory::InMemoryImageRepository;
