// src/lib.rs
// Library crate entry point

pub mod tree;
pub mod bench;
pub mod bench_saver;
pub mod process_handle;

// Re-export TreeMap for easy access
pub use tree::TreeMap;


