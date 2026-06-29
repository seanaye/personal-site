//! Adapters connect the hexagonal ports to concrete infrastructure.

pub mod memory;

pub use memory::InMemoryImageRepository;
