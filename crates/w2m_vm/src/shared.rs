// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! What the processes of a run share on the host (Sprint 197; the Decision "Sequence after Sprint 196",
//! docs/design-rust-host-2026-10-24.md Change 4): a file created and unlinked at once, which a child `w2m-vm` inherits
//! by its descriptor and both map shared, so no file outlives the run; and a page of such a file whose words carry the
//! turn, waited on with `os_sync_wait_on_address` and OS_SYNC_WAIT_ON_ADDRESS_SHARED (os/os_sync_wait_on_address.h
//! 61-65, 153-157, 295-299: both flags 1; macOS 14.4).

use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicU64, Ordering};

unsafe extern "C" {
    /// `sys/fcntl.h` 605, variadic
    fn fcntl(fd: i32, cmd: i32, ...) -> i32;
    /// `os/os_sync_wait_on_address.h` 153-157
    fn os_sync_wait_on_address(addr: *mut std::ffi::c_void, value: u64, size: usize, flags: u32) -> i32;
    /// the same header: wakes every waiter on `addr`
    fn os_sync_wake_by_address_all(addr: *mut std::ffi::c_void, size: usize, flags: u32) -> i32;
    /// the same header 268-273: a wait that returns after `timeout_ns` on clock `clockid`
    fn os_sync_wait_on_address_with_timeout(
        addr: *mut std::ffi::c_void,
        value: u64,
        size: usize,
        flags: u32,
        clockid: u32,
        timeout_ns: u64,
    ) -> i32;
}

/// `OS_CLOCK_MACH_ABSOLUTE_TIME` (os/clock.h 15)
const MACH_ABSOLUTE_TIME: u32 = 32;
/// how often a waiter looks whether the process it waits for is still there
const ALIVE_CHECK_NS: u64 = 100_000_000;

/// `sys/fcntl.h` 235, 354
const F_SETFD: i32 = 2;
/// OS_SYNC_WAIT_ON_ADDRESS_SHARED and OS_SYNC_WAKE_BY_ADDRESS_SHARED
const SHARED: u32 = 1;

/// The number in the next anonymous file's name.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// A file of `len` bytes (sparse), created in the temporary directory and unlinked before it is returned.
pub fn anonymous_file(len: u64) -> io::Result<File> {
    anonymous_file_in(&std::env::temp_dir(), len)
}

/// `anonymous_file` in `dir`, which a test gives its own.
fn anonymous_file_in(dir: &std::path::Path, len: u64) -> io::Result<File> {
    // the next name when one is taken, left by a process of the same id killed between the open and the unlink, as
    // `FileBridge::new` passes over a drive left behind (Sprint 247)
    let (file, path) = loop {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = dir.join(format!("w2m-shared-{}-{n}", std::process::id()));
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => break (file, path),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    };
    std::fs::remove_file(&path)?;
    file.set_len(len)?;
    Ok(file)
}

/// The descriptor stays open across `exec`, for a child to inherit (Rust opens every file close-on-exec).
pub fn inheritable(file: &File) -> io::Result<i32> {
    let fd = file.as_raw_fd();
    if unsafe { fcntl(fd, F_SETFD, 0) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(fd)
}

/// A page of 64-bit words both processes map from one file.
pub struct Page {
    memory: hypervisor::GuestMemory,
}

impl Page {
    /// `len` bytes of the file `fd`, shared, a multiple of 16 KB.
    pub fn map(fd: i32, len: usize) -> io::Result<Page> {
        Ok(Page {
            memory: hypervisor::GuestMemory::shared(fd, len)?,
        })
    }

    fn word(&self, index: usize) -> &AtomicU64 {
        let words = self.memory.len() / 8;
        assert!(index < words, "word {index} of a page of {words}");
        // the mapping is 16 KB aligned, so every word is 8-byte aligned; AtomicU64 has u64's layout
        unsafe { &*(self.memory.as_slice().as_ptr().add(8 * index) as *const AtomicU64) }
    }

    pub fn load(&self, index: usize) -> u64 {
        self.word(index).load(Ordering::SeqCst)
    }

    pub fn store(&self, index: usize, value: u64) {
        self.word(index).store(value, Ordering::SeqCst);
    }

    /// Stores `value` and wakes every process waiting on the word.
    pub fn post(&self, index: usize, value: u64) {
        self.store(index, value);
        let addr = self.word(index) as *const AtomicU64 as *mut std::ffi::c_void;
        unsafe { os_sync_wake_by_address_all(addr, 8, SHARED) };
    }

    /// Waits until the word holds a value `done` accepts; returns it.
    pub fn wait_until(&self, index: usize, done: impl Fn(u64) -> bool) -> u64 {
        loop {
            let seen = self.load(index);
            if done(seen) {
                return seen;
            }
            let addr = self.word(index) as *const AtomicU64 as *mut std::ffi::c_void;
            // returns at a wake, a change from `seen`, or a signal; each is looked at again
            unsafe { os_sync_wait_on_address(addr, seen, 8, SHARED) };
        }
    }

    /// `wait_until`, looking every 100 ms whether the process that should post is still there (`alive`); None when
    /// it is gone without posting, so no waiter outlives the process it waits for.
    pub fn wait_until_alive(
        &self,
        index: usize,
        done: impl Fn(u64) -> bool,
        mut alive: impl FnMut() -> bool,
    ) -> Option<u64> {
        loop {
            let seen = self.load(index);
            if done(seen) {
                return Some(seen);
            }
            if !alive() {
                let last = self.load(index);
                return done(last).then_some(last);
            }
            let addr = self.word(index) as *const AtomicU64 as *mut std::ffi::c_void;
            unsafe { os_sync_wait_on_address_with_timeout(addr, seen, 8, SHARED, MACH_ABSOLUTE_TIME, ALIVE_CHECK_NS) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A name left by a killed process of the same id is passed over, not an error (Sprint 247).
    #[test]
    fn a_name_left_behind_is_passed_over() {
        // a directory of its own, so the names it leaves are not the other tests' to count
        let dir = std::env::temp_dir().join(format!("w2m-shared-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        let from = NEXT.load(Ordering::Relaxed);
        for k in from..from + 1024 {
            std::fs::write(dir.join(format!("w2m-shared-{}-{k}", std::process::id())), b"").unwrap();
        }
        let made = anonymous_file_in(&dir, 4096);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(made.unwrap().metadata().unwrap().len(), 4096);
    }

    /// The turn's round trip (the Decision "Sequence after Sprint 196" revisits the transport above 10 µs median): two
    /// threads, each with its own mapping, hand the turn word back and forth 10,000 times; the median is printed
    /// (6,292 ns, p99 12,750 ns on 2026-10-28, an M5 Max) and held under 1 ms, a gross failure only, since the gates run
    /// it beside other tests.
    #[test]
    fn a_turn_goes_and_comes_back_within_a_millisecond() {
        const ROUNDS: u64 = 10_000;
        let file = anonymous_file(16 * 1024).unwrap();
        let fd = file.as_raw_fd();
        let other = std::thread::spawn(move || {
            let page = Page::map(fd, 16 * 1024).unwrap();
            for r in 0..ROUNDS {
                page.wait_until(0, |v| v == 2 * r + 1);
                page.post(0, 2 * r + 2);
            }
        });
        let page = Page::map(fd, 16 * 1024).unwrap();
        let mut ns = Vec::with_capacity(ROUNDS as usize);
        for r in 0..ROUNDS {
            let t = std::time::Instant::now();
            page.post(0, 2 * r + 1);
            page.wait_until(0, |v| v == 2 * r + 2);
            ns.push(t.elapsed().as_nanos() as u64);
        }
        other.join().unwrap();
        ns.sort_unstable();
        let median = ns[ns.len() / 2];
        println!(
            "turn round trip: median {median} ns, p99 {} ns over {ROUNDS}",
            ns[ns.len() * 99 / 100]
        );
        assert!(median < 1_000_000, "median round trip {median} ns");
    }

    /// The turn's round trip between two processes, as a supervisor and a child `w2m-vm` hand it (Sprint 199;
    /// reviewer-2's Medium 5 of its review of 196-198: measured between threads before): this test runs its own binary
    /// again as the other process, which inherits the page's descriptor and hands the turn back 10,000 times; the
    /// median is printed and held under 1 ms, a gross failure only.
    #[test]
    fn a_turn_goes_to_another_process_and_comes_back() {
        const ROUNDS: u64 = 10_000;
        if let Ok(fd) = std::env::var("W2M_TURN_PEER") {
            let page = Page::map(fd.parse().unwrap(), 16 * 1024).unwrap();
            for r in 0..ROUNDS {
                page.wait_until(0, |v| v == 2 * r + 1);
                page.post(0, 2 * r + 2);
            }
            return;
        }
        let file = anonymous_file(16 * 1024).unwrap();
        let fd = inheritable(&file).unwrap();
        let page = Page::map(fd, 16 * 1024).unwrap();
        let mut peer = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "shared::tests::a_turn_goes_to_another_process_and_comes_back",
                "--nocapture",
            ])
            .env("W2M_TURN_PEER", fd.to_string())
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let mut ns = Vec::with_capacity(ROUNDS as usize);
        for r in 0..ROUNDS {
            let t = std::time::Instant::now();
            page.post(0, 2 * r + 1);
            page.wait_until(0, |v| v == 2 * r + 2);
            ns.push(t.elapsed().as_nanos() as u64);
        }
        assert!(peer.wait().unwrap().success());
        ns.sort_unstable();
        let median = ns[ns.len() / 2];
        println!(
            "turn round trip across two processes: median {median} ns, p99 {} ns over {ROUNDS}",
            ns[ns.len() * 99 / 100]
        );
        assert!(median < 1_000_000, "median round trip {median} ns");
    }

    /// The turn word between two mappings of one unlinked file: a post through one ends a wait through the other, and
    /// the file is gone from the temporary directory while both map it.
    #[test]
    fn a_post_through_one_mapping_ends_a_wait_through_another() {
        let file = anonymous_file(16 * 1024).unwrap();
        let fd = file.as_raw_fd();
        let a = Page::map(fd, 16 * 1024).unwrap();
        let waiter = std::thread::spawn(move || Page::map(fd, 16 * 1024).unwrap().wait_until(3, |v| v == 4100));
        std::thread::sleep(std::time::Duration::from_millis(20));
        a.post(3, 4100);
        assert_eq!(waiter.join().unwrap(), 4100);
        let left = std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(&format!("w2m-shared-{}-", std::process::id()))
            })
            .count();
        assert_eq!(left, 0, "an unlinked file stayed in the temporary directory");
    }
}
