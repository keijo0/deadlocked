/// Address-keyed memory read cache.
///
/// [`MemoryCache`] stores the most-recently-read byte buffers for a set of
/// game-process addresses so that repeated reads within the same frame can be
/// served without issuing additional syscalls.
///
/// # Design
///
/// Each cache entry stores the raw bytes together with the `Instant` at which
/// they were populated.  A configurable TTL controls how long entries are
/// considered fresh.  The default TTL (16 ms) corresponds to one 60 Hz frame.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

/// Default maximum time a cache entry is considered valid (≈ one 60 Hz frame).
const DEFAULT_TTL: Duration = Duration::from_millis(16);

#[derive(Debug)]
struct Entry {
    data: Vec<u8>,
    inserted_at: Instant,
}

/// A time-bounded cache mapping `u64` addresses to raw byte buffers.
#[derive(Debug)]
pub struct MemoryCache {
    map: HashMap<u64, Entry>,
    ttl: Duration,
}

impl MemoryCache {
    /// Create a new cache with the default 16 ms TTL.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            ttl: DEFAULT_TTL,
        }
    }

    /// Return a slice for `address` if a fresh entry exists, otherwise `None`.
    pub fn get(&self, address: u64) -> Option<&[u8]> {
        let entry = self.map.get(&address)?;
        if entry.inserted_at.elapsed() < self.ttl {
            Some(&entry.data)
        } else {
            None
        }
    }

    /// Store `data` under `address`.
    pub fn insert(&mut self, address: u64, data: Vec<u8>) {
        self.map.insert(
            address,
            Entry {
                data,
                inserted_at: Instant::now(),
            },
        );
    }

    /// Remove all entries whose TTL has elapsed.
    pub fn evict_expired(&mut self) {
        let ttl = self.ttl;
        self.map
            .retain(|_, entry| entry.inserted_at.elapsed() < ttl);
    }
}

impl Default for MemoryCache {
    fn default() -> Self {
        Self::new()
    }
}
