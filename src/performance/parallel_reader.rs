/// Rayon-backed parallel memory reader.
///
/// [`read_parallel`] accepts a list of `(address, length)` read requests and
/// dispatches them across Rayon's global thread pool, returning results in the
/// same order.
///
/// Compared to sequential reads this reduces total wall-clock time when the
/// game-loop needs to read many independent memory regions (e.g. per-player
/// bone positions, entity lists).

use rayon::prelude::*;

/// A single memory-read request.
#[derive(Debug, Clone)]
pub struct ReadRequest {
    /// Remote process virtual address.
    pub address: u64,
    /// Number of bytes to read.
    pub length: usize,
}

/// Result of one completed read request.
#[derive(Debug)]
pub struct ReadResult {
    /// Bytes read from the remote process. Empty on failure.
    pub data: Vec<u8>,
}

/// Performs multiple memory reads in parallel using `process_vm_readv`.
///
/// Each request is dispatched to the Rayon thread pool; the results are
/// collected and returned in request-order.
///
/// # Safety
///
/// `process_vm_readv` is called from multiple threads simultaneously.
/// Each read has its own local and remote `iovec` pair so there is no shared
/// mutable state between threads.
pub fn read_parallel(pid: i32, requests: &[ReadRequest]) -> Vec<ReadResult> {
    if requests.is_empty() {
        return vec![];
    }

    requests
        .par_iter()
        .map(|req| {
            let data = read_one(pid, req.address, req.length);
            ReadResult {
                data,
            }
        })
        .collect()
}

/// Read `length` bytes from `address` in process `pid` using one
/// `process_vm_readv` syscall.
fn read_one(pid: i32, address: u64, length: usize) -> Vec<u8> {
    use nix::libc::{self, iovec, process_vm_readv};

    let mut buf = vec![0u8; length];
    if length == 0 {
        return buf;
    }

    let local_iov = iovec {
        iov_base: buf.as_mut_ptr() as *mut libc::c_void,
        iov_len: length,
    };
    let remote_iov = iovec {
        iov_base: address as *mut libc::c_void,
        iov_len: length,
    };

    unsafe {
        process_vm_readv(pid, &local_iov, 1, &remote_iov, 1, 0);
    }

    buf
}
