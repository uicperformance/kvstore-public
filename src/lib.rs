// src/lib.rs
// Library crate entry point
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

pub mod btree;
pub mod tree;
pub mod bench;
pub mod bench_saver;
pub mod process_handle;

// Re-export TreeMap for easy access
pub use tree::TreeMap;


