//! Adapters connect the hexagonal ports to concrete infrastructure.

pub mod memory;
pub mod r2;

pub use memory::InMemoryImageRepository;
pub use r2::R2ImageRepository;
