// src/lib.rs
// Library crate entry point

pub mod bench;
pub mod bench_saver;
pub mod process_handle;
pub mod tree;

// Re-export TreeMap for easy access
pub use tree::TreeMap;
