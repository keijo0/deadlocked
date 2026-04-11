//! Performance module – memory caching, parallel reads, and GPU optimisations.
//!
//! Sub-modules:
//! - [`gpu_optimizations`] – adaptive frame skipping and frame-time tracking
//! - [`memory_cache`]      – TTL-bounded address-keyed byte-buffer cache
//! - [`parallel_reader`]   – rayon-parallel `process_vm_readv` dispatcher

pub mod gpu_optimizations;
pub mod memory_cache;
pub mod parallel_reader;

pub use gpu_optimizations::FrameTimer;
pub use memory_cache::MemoryCache;
pub use parallel_reader::{ReadRequest, ReadResult};
