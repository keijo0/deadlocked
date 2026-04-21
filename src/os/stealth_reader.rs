#![allow(dead_code)]
/// Syscall pattern obfuscation for `process_vm_readv`.
///
/// [`StealthReader`] wraps the raw `process_vm_readv` syscall with several
/// evasion techniques:
///
/// - **Jitter**: Adds a small, pseudo-random inter-frame delay so the syscall
///   cadence does not form an obvious fixed-rate pattern.
/// - **`/proc/pid/mem` fallback**: For larger contiguous regions the reader
///   opens `/proc/{pid}/mem`, issues a single `pread`, then closes the file
///   immediately so that no persistent file descriptor is held between reads.
/// - **Background noise thread**: An optional background thread issues
///   additional, harmless `process_vm_readv` calls at randomised addresses
///   and intervals to break fixed-rate detection heuristics.

use std::{
    fs::OpenOptions,
    os::unix::fs::FileExt,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use bytemuck::Pod;
use nix::libc::{self, iovec, process_vm_readv};
use utils::log;

/// Minimum size (in bytes) above which a read is routed through
/// `/proc/pid/mem` instead of `process_vm_readv`.
const PROCMEM_THRESHOLD: usize = 4096;

/// Maximum jitter added per game-loop frame (≈ 500 µs).
const JITTER_MAX_NS: u64 = 500_000;

/// Simple linear-congruential generator seeded from wall-clock time.
/// Avoids an external `rand` dependency while providing sufficient entropy
/// for timing jitter.
pub struct Lcg(u64);

impl Lcg {
    pub fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(0xDEAD_BEEF_1234_5678);
        Self(seed)
    }

    #[inline]
    pub fn next(&mut self) -> u64 {
        // Knuth's multiplicative LCG
        self.0 = self.0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    /// Return a value in `[0, max)`.
    #[inline]
    pub fn next_below(&mut self, max: u64) -> u64 {
        if max == 0 {
            return 0;
        }
        self.next() % max
    }
}

impl Default for Lcg {
    fn default() -> Self {
        Self::new()
    }
}

/// A memory reader for a target process that applies syscall-pattern evasion.
pub struct StealthReader {    pid: i32,
    lcg: Lcg,
    /// Counts of reads routed through each method (for debugging).
    reads_vm: Arc<AtomicU64>,
    reads_proc: Arc<AtomicU64>,
    /// Shared syscall counter fed into [`FrequencyDefense`].
    syscall_counter: Arc<AtomicU64>,
    /// Set to `false` to stop the background noise thread.
    noise_running: Arc<AtomicBool>,
    _noise_thread: Option<std::thread::JoinHandle<()>>,
}

impl StealthReader {
    /// Create a reader for `pid`, sharing `syscall_counter` with a
    /// [`FrequencyDefense`] instance.
    pub fn new(pid: i32, syscall_counter: Arc<AtomicU64>) -> Self {
        Self {
            pid,
            lcg: Lcg::new(),
            reads_vm: Arc::new(AtomicU64::new(0)),
            reads_proc: Arc::new(AtomicU64::new(0)),
            syscall_counter,
            noise_running: Arc::new(AtomicBool::new(false)),
            _noise_thread: None,
        }
    }

    /// Read `size_of::<T>()` bytes from `address` in the target process.
    ///
    /// Small reads use `process_vm_readv`; reads ≥ [`PROCMEM_THRESHOLD`] use
    /// `/proc/{pid}/mem`.
    pub fn read<T: Pod + Default>(&mut self, address: u64) -> T {
        let size = size_of::<T>();
        if size >= PROCMEM_THRESHOLD {
            self.read_via_procmem(address)
        } else {
            self.read_via_vmreadv(address)
        }
    }

    /// Read raw bytes from the target process via `process_vm_readv`.
    pub fn read_bytes_vm(&mut self, address: u64, len: usize) -> Vec<u8> {
        let mut buf = vec![0u8; len];
        let local_iov = iovec {
            iov_base: buf.as_mut_ptr() as *mut libc::c_void,
            iov_len: len,
        };
        let remote_iov = iovec {
            iov_base: address as *mut libc::c_void,
            iov_len: len,
        };
        unsafe {
            process_vm_readv(self.pid, &local_iov, 1, &remote_iov, 1, 0);
        }
        self.syscall_counter.fetch_add(1, Ordering::Relaxed);
        self.reads_vm.fetch_add(1, Ordering::Relaxed);
        buf
    }

    /// Return a jitter `Duration` drawn uniformly from `[0, JITTER_MAX_NS)`.
    ///
    /// Call this once per game-loop iteration (not per-read) and sleep for the
    /// returned duration to vary the overall syscall cadence.
    pub fn frame_jitter(&mut self) -> Duration {
        let ns = self.lcg.next_below(JITTER_MAX_NS);
        Duration::from_nanos(ns)
    }

    // --- private helpers ---

    fn read_via_vmreadv<T: Pod + Default>(&mut self, address: u64) -> T {
        let mut t = T::default();
        let buffer = bytemuck::bytes_of_mut(&mut t);
        let local_iov = iovec {
            iov_base: buffer.as_mut_ptr() as *mut libc::c_void,
            iov_len: buffer.len(),
        };
        let remote_iov = iovec {
            iov_base: address as *mut libc::c_void,
            iov_len: buffer.len(),
        };
        unsafe {
            process_vm_readv(self.pid, &local_iov, 1, &remote_iov, 1, 0);
        }
        self.syscall_counter.fetch_add(1, Ordering::Relaxed);
        self.reads_vm.fetch_add(1, Ordering::Relaxed);
        t
    }

    fn read_via_procmem<T: Pod + Default>(&mut self, address: u64) -> T {
        let mut t = T::default();
        let buffer = bytemuck::bytes_of_mut(&mut t);
        let path = format!("/proc/{}/mem", self.pid);
        match OpenOptions::new().read(true).open(&path) {
            Ok(f) => {
                let _ = f.read_at(buffer, address);
                self.reads_proc.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                log::warn!("stealth_reader: procmem open failed: {e}");
            }
        }
        t
    }

    /// Start an optional background noise thread that issues periodic
    /// harmless `process_vm_readv` calls at randomised addresses.
    ///
    /// This breaks fixed-rate detection heuristics by adding irregular
    /// syscall patterns between real reads.  The noise addresses are always
    /// within the valid mapped range `[min, max)`.
    pub fn start_noise_thread(&mut self, min: u64, max: u64) {
        if self.noise_running.load(Ordering::Relaxed) {
            return;
        }
        self.noise_running.store(true, Ordering::Relaxed);
        let running = self.noise_running.clone();
        let counter = self.syscall_counter.clone();
        let pid = self.pid;

        std::thread::spawn(move || {
            let mut lcg = Lcg::new();
            let range = max.saturating_sub(min).max(1);
            loop {
                if !running.load(Ordering::Relaxed) {
                    break;
                }
                // Sleep a random interval between 5 ms and 25 ms.
                let sleep_ms = 5 + lcg.next_below(20);
                std::thread::sleep(Duration::from_millis(sleep_ms));

                // Issue a harmless read to a random address.
                let addr = min + lcg.next_below(range);
                let mut dummy = [0u8; 8];
                let local_iov = iovec {
                    iov_base: dummy.as_mut_ptr() as *mut libc::c_void,
                    iov_len: dummy.len(),
                };
                let remote_iov = iovec {
                    iov_base: addr as *mut libc::c_void,
                    iov_len: dummy.len(),
                };
                unsafe {
                    process_vm_readv(pid, &local_iov, 1, &remote_iov, 1, 0);
                }
                counter.fetch_add(1, Ordering::Relaxed);
            }
        });
    }

    /// Stop the background noise thread (if running).
    pub fn stop_noise_thread(&mut self) {
        self.noise_running.store(false, Ordering::Relaxed);
    }
}

impl std::fmt::Debug for StealthReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StealthReader")
            .field("pid", &self.pid)
            .finish_non_exhaustive()
    }
}

impl Drop for StealthReader {
    fn drop(&mut self) {
        self.stop_noise_thread();
    }
}
