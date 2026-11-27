// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! What crosses processes on several vCPUs, answered one request at a time (Sprint 363; card 4f2 of
//! `docs/design-a-process-that-starts-another-2026-11-26.md`, "The request server" and "Which lock or request each
//! crossing takes"). On several vCPUs no Machine runs: each process runs at once on its own vCPUs, and a thread of the
//! first `w2m-vm`, the request server, answers a child's KERNEL 33 under `kernel` (`run::KernelLock`), the host
//! design's Change 4 supervisor. It holds no guest word and no guest lock, and serves a request against a view of the
//! child's mapping (`kernel::MappedMemoryView`), never a slice, since the child's other vCPUs run meanwhile.
//!
//! The page is a second layout on the same anonymous 64 KB file the turn form maps (`turns::PAGE_SIZE`,
//! `shared::anonymous_file`): word DOORBELL, which the server waits on, then a slot of SLOT words per child. A request
//! is one round trip: the client writes its message and arguments, stores its state POSTED, raises the doorbell and
//! waits on its state; the server serves it, writes the result and second word, then the state, ANSWERED, by a release
//! store after the answer's words, and wakes the slot. Words two processes wait on are waited on with
//! `os_sync_wait_on_address` and OS_SYNC_WAIT_ON_ADDRESS_SHARED, as `shared.rs` waits; the doorbell wait times out
//! every 100 ms, and the server then looks whether each child is still there, as `shared::Page::wait_until_alive`
//! looks. No run reaches the server yet: card 4f3 (Sprint 364) makes it at a spawn on several vCPUs.

use std::ffi::c_void;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::process::Child;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::kernel::MappedMemoryView;
use crate::run::KernelLock;
use crate::shared::{anonymous_file, inheritable};

unsafe extern "C" {
    /// `os/os_sync_wait_on_address.h` 153-157: blocks while `addr` holds `value`, until a wake
    fn os_sync_wait_on_address(addr: *mut c_void, value: u64, size: usize, flags: u32) -> i32;
    /// the same header 265-273: the wait, returning -1 with ETIMEDOUT once `timeout_ns` of clock `clockid` pass
    fn os_sync_wait_on_address_with_timeout(
        addr: *mut c_void,
        value: u64,
        size: usize,
        flags: u32,
        clockid: u32,
        timeout_ns: u64,
    ) -> i32;
    /// the same header 302-341: "wakes up one waiter out of all those blocked"
    fn os_sync_wake_by_address_any(addr: *mut c_void, size: usize, flags: u32) -> i32;
    /// the same header 343-372: wakes every waiter
    fn os_sync_wake_by_address_all(addr: *mut c_void, size: usize, flags: u32) -> i32;
}

/// OS_SYNC_WAIT_ON_ADDRESS_SHARED and OS_SYNC_WAKE_BY_ADDRESS_SHARED (the same header 61-66, 295-300)
const SHARED: u32 = 1;
/// `OS_CLOCK_MACH_ABSOLUTE_TIME` (os/clock.h 15)
const MACH_ABSOLUTE_TIME: u32 = 32;
/// `sys/errno.h` 177
const ETIMEDOUT: i32 = 60;
/// how often the server looks whether each child is still there (`shared.rs`' ALIVE_CHECK_NS)
const LOOK_EVERY: Duration = Duration::from_millis(100);
/// How long a client looks at its state before it blocks on it: spin, then park, as parking_lot's `SpinWait::spin`
/// ("Spins until the sleep threshold has been reached ... at which point further spinning has diminishing returns and
/// the thread should be parked instead", `core/src/spinwait.rs` 32-52 at 85827ffa2) and std's futex mutex ("Spin first
/// to speed things up if the lock is released quickly", `library/std/src/sys/sync/mutex/futex.rs` 35) do. An answer
/// comes in about 8 µs from a server woken from its doorbell wait, past their few hundred loads, so the bound is a time.
/// Measured by Sprint 363's two-process test alone, requests 50 µs apart: a median of 8,458 to 8,750 ns with it,
/// 9,875 to 10,417 ns blocking at once. It holds the requesting vCPU's core busy up to 20 µs a request: at Change 4's
/// bound of 376 requests a second, 7.5 ms of a core a second at most.
const ANSWER_SPIN: Duration = Duration::from_micros(20);

/// The page: the turn form's 64 KB, the doorbell, then a slot of SLOT words per child.
const PAGE_SIZE: usize = crate::turns::PAGE_SIZE;
const DOORBELL: usize = 0;
const SLOT: usize = 1024;
const SLOTS: usize = PAGE_SIZE / 8 / SLOT - 1;
/// A slot's words: the request's state, its message, its sub-operation and three arguments, then its answer.
const STATE: usize = 0;
const MESSAGE: usize = 1;
const ARGS: usize = 2;
const RESULT: usize = 6;
const SECOND: usize = 7;
/// the code an ENDED message carries
const EXIT_CODE: usize = 8;
/// raised by the server after every waking answer, waited on by the child's link thread (card 4f3)
const WAKES: usize = 9;
/// a STOPPED message's text: its length in bytes, then its bytes, 1,024 at most
const STOP_LEN: usize = 10;
const STOP_TEXT: usize = 11;
const STOP_WORDS: usize = 128;
/// the child's maps, (VA, IPA, size) each, posted once at its start (the layout is fixed after load): their count,
/// then the triples, 256 at most
const MAPS_LEN: usize = STOP_TEXT + STOP_WORDS;
const MAPS: usize = MAPS_LEN + 1;
const MAPS_MAX: usize = 256;
const _: () = assert!(MAPS + 3 * MAPS_MAX <= SLOT);

/// A slot's states.
const IDLE: u64 = 0;
const POSTED: u64 = 1;
const ANSWERED: u64 = 2;
/// A posted slot's messages: a KERNEL 33 request, the child's end, its stop.
const REQUEST: u64 = 1;
const ENDED: u64 = 2;
const STOPPED: u64 = 3;

/// The index of word `w` of slot `slot`.
fn word(slot: usize, w: usize) -> usize {
    (1 + slot) * SLOT + w
}

/// The page as one thread or process maps it: its words as atomics, each reached by atomics of its own size only.
struct Words {
    memory: hypervisor::GuestMemory,
}

impl Words {
    fn map(fd: i32) -> Result<Words, String> {
        Ok(Words {
            memory: hypervisor::GuestMemory::shared(fd, PAGE_SIZE).map_err(|e| format!("the request page: {e}"))?,
        })
    }

    fn at(&self, index: usize) -> &AtomicU64 {
        crate::guest::memory_view(&self.memory).atomic_u64(8 * index as u64)
    }

    fn addr(&self, index: usize) -> *mut c_void {
        self.at(index) as *const AtomicU64 as *mut c_void
    }

    fn wake_one(&self, index: usize) {
        // ENOENT, no waiter, is no fault: the waiter compares the word before it blocks
        unsafe { os_sync_wake_by_address_any(self.addr(index), 8, SHARED) };
    }

    fn wake_every(&self, index: usize) {
        unsafe { os_sync_wake_by_address_all(self.addr(index), 8, SHARED) };
    }

    /// Blocks while the word holds `seen`, until a wake; at once when it holds another value. A signal or a spurious
    /// return is looked at again by the caller.
    fn wait(&self, index: usize, seen: u64) {
        unsafe { os_sync_wait_on_address(self.addr(index), seen, 8, SHARED) };
    }

    /// `wait`, `timeout` at most: true when the timeout ended it.
    fn wait_for(&self, index: usize, seen: u64, timeout: Duration) -> bool {
        let ns = (timeout.as_nanos() as u64).max(1);
        let r =
            unsafe { os_sync_wait_on_address_with_timeout(self.addr(index), seen, 8, SHARED, MACH_ABSOLUTE_TIME, ns) };
        r == -1 && std::io::Error::last_os_error().raw_os_error() == Some(ETIMEDOUT)
    }
}

/// The first process's own wake count (the design's "an in-process count and condition the server and its own
/// requests raise", since the header says SHARED "should not be used when synchronizing among multiple threads of a
/// single process"): raised after every waking answer; card 4f3's link thread waits on it.
#[derive(Default)]
pub struct WakeCount {
    count: Mutex<u64>,
    raised: Condvar,
}

impl WakeCount {
    pub fn now(&self) -> u64 {
        *self.count.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn raise(&self) {
        *self.count.lock().unwrap_or_else(|e| e.into_inner()) += 1;
        self.raised.notify_all();
    }

    /// Waits until the count passes `seen`, `timeout` at most; the count then.
    pub fn wait_past(&self, seen: u64, timeout: Duration) -> u64 {
        let held = self.count.lock().unwrap_or_else(|e| e.into_inner());
        *self
            .raised
            .wait_timeout_while(held, timeout, |n| *n <= seen)
            .unwrap_or_else(|e| e.into_inner())
            .0
    }
}

/// A value only the request server's thread makes: `KernelLock::take_for_request_server` asks for it, so no other
/// taker of `kernel` passes `take`'s assertion of a runtime word.
pub struct RequestServerThread(());

/// A child the server answers.
struct ServedChild {
    slot: usize,
    pid: u64,
    process: Child,
    memory: hypervisor::GuestMemory,
    /// its maps as offsets in `memory`, (VA, offset, size), read at its first request
    maps: Option<Vec<(u64, u64, u64)>>,
    /// its end or stop answered: its exit is no stop of the run
    ended: bool,
    /// its process waited for
    gone: bool,
    /// a request refused, the run stopped by it: the slot is not served again
    refused: bool,
}

/// What the server's thread and its owner share.
struct Served {
    kernel: Arc<KernelLock>,
    children: Mutex<Vec<ServedChild>>,
    ending: AtomicBool,
    wakes: WakeCount,
    stop: Mutex<Option<String>>,
    stopped: Condvar,
    #[cfg(test)]
    answer_pause: AnswerPause,
}

impl Served {
    /// The run's stop, the first one kept (card 4f3 hands it to the run's `RunStop`).
    fn stop(&self, why: String) {
        self.stop.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert(why);
        self.stopped.notify_all();
    }
}

/// The request server: one thread of the first `w2m-vm`, from its start to its end.
pub struct Server {
    served: Arc<Served>,
    page_file: File,
    page: Words,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    /// The page made and the server's thread started, taking `kernel` for each request.
    pub fn start(kernel: Arc<KernelLock>) -> Result<Server, String> {
        let page_file = anonymous_file(PAGE_SIZE as u64).map_err(|e| format!("the request page: {e}"))?;
        let page = Words::map(page_file.as_raw_fd())?;
        let thread_page = Words::map(page_file.as_raw_fd())?;
        let served = Arc::new(Served {
            kernel,
            children: Mutex::new(Vec::new()),
            ending: AtomicBool::new(false),
            wakes: WakeCount::default(),
            stop: Mutex::new(None),
            stopped: Condvar::new(),
            #[cfg(test)]
            answer_pause: AnswerPause::default(),
        });
        let theirs = served.clone();
        let thread = std::thread::Builder::new()
            .name("w2m-request-server".into())
            .spawn(move || serve_until_ended(&theirs, &thread_page))
            .map_err(|e| format!("the request server's thread: {e}"))?;
        Ok(Server {
            served,
            page_file,
            page,
            thread: Some(thread),
        })
    }

    /// The page's descriptor, left open across `exec` for a child to inherit.
    pub fn page_descriptor(&self) -> Result<i32, String> {
        inheritable(&self.page_file).map_err(|e| format!("the request page: {e}"))
    }

    /// Child `pid`, in `slot`, its process `process`, its memory the shared file `memory`: answered from now on, and
    /// looked at every 100 ms. The doorbell is raised, since the child may have posted before it was adopted.
    pub fn adopt(&self, slot: usize, pid: u64, process: Child, memory: &File) -> Result<(), String> {
        if slot >= SLOTS {
            return Err(format!("slot {slot} of a page of {SLOTS}"));
        }
        let len = memory.metadata().map_err(|e| e.to_string())?.len() as usize;
        let memory = hypervisor::GuestMemory::shared(memory.as_raw_fd(), len).map_err(|e| e.to_string())?;
        self.served
            .children
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(ServedChild {
                slot,
                pid,
                process,
                memory,
                maps: None,
                ended: false,
                gone: false,
                refused: false,
            });
        ring(&self.page);
        Ok(())
    }

    /// A test's pause, 1 ms before every `every`th answer's words; 0 turns it off.
    #[cfg(test)]
    fn pause_before_answers(&self, every: u64) {
        self.served.answer_pause.every.store(every, Ordering::Relaxed);
    }

    /// The first process's wake count.
    pub fn wakes(&self) -> &WakeCount {
        &self.served.wakes
    }

    /// The run's stop, if the server has made one: a child's stop, a refused request, a child gone without a word.
    pub fn stopped(&self) -> Option<String> {
        self.served.stop.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Waits for the run's stop, `timeout` at most.
    pub fn wait_for_stop(&self, timeout: Duration) -> Option<String> {
        let held = self.served.stop.lock().unwrap_or_else(|e| e.into_inner());
        self.served
            .stopped
            .wait_timeout_while(held, timeout, |s| s.is_none())
            .unwrap_or_else(|e| e.into_inner())
            .0
            .clone()
    }

    /// The run ended: the thread stopped and joined, every child process stopped and waited for.
    pub fn end(&mut self) {
        let Some(thread) = self.thread.take() else {
            return;
        };
        self.served.ending.store(true, Ordering::Release);
        ring(&self.page);
        let _ = thread.join();
        for c in self
            .served
            .children
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter_mut()
        {
            let _ = c.process.kill();
            let _ = c.process.wait();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.end();
    }
}

/// The doorbell raised: its count, then a wake for the server.
fn ring(page: &Words) {
    page.at(DOORBELL).fetch_add(1, Ordering::Release);
    page.wake_one(DOORBELL);
}

/// The server's thread: every posted slot served, then a wait on the doorbell from the count read before the slots
/// were looked at, so a request posted meanwhile ends the wait at once; every 100 ms a look at each child.
fn serve_until_ended(served: &Served, page: &Words) {
    let me = RequestServerThread(());
    let mut looked = Instant::now();
    loop {
        let rung = page.at(DOORBELL).load(Ordering::Acquire);
        if served.ending.load(Ordering::Acquire) {
            return;
        }
        serve_posted(served, page, &me);
        if looked.elapsed() >= LOOK_EVERY {
            look_at_children(served);
            looked = Instant::now();
        }
        page.wait_for(DOORBELL, rung, LOOK_EVERY.saturating_sub(looked.elapsed()));
    }
}

/// Every posted slot of a child not refused, served and answered. `kernel` is taken first, then the children's table,
/// which is under it as the design puts it ("the children's table is under it too"): a spawn made under `kernel` adopts
/// its child with no other order to meet, and the look at the children takes the table alone.
fn serve_posted(served: &Served, page: &Words, me: &RequestServerThread) {
    let posted: Vec<usize> = (0..SLOTS)
        .filter(|&s| page.at(word(s, STATE)).load(Ordering::Acquire) == POSTED)
        .collect();
    if posted.is_empty() {
        return;
    }
    let mut kernel = served.kernel.take_for_request_server(me);
    let mut children = served.children.lock().unwrap_or_else(|e| e.into_inner());
    let mut wake = false;
    for c in children.iter_mut().filter(|c| !c.refused && posted.contains(&c.slot)) {
        let s = c.slot;
        let message = page.at(word(s, MESSAGE)).load(Ordering::Relaxed);
        let a = std::array::from_fn(|k| page.at(word(s, ARGS + k)).load(Ordering::Relaxed));
        let answer = match message {
            REQUEST => {
                let maps = c.maps.get_or_insert_with(|| read_maps(page, s));
                let mut memory = MappedMemoryView::new(crate::guest::memory_view(&c.memory), maps);
                match kernel.objects.request(c.pid, a, &mut memory) {
                    Ok(answer) if answer.spawn.is_some() => Err("a grandchild process: not served yet".into()),
                    Ok(answer) => Ok((answer.result, answer.second, answer.wake)),
                    Err(e) => Err(e),
                }
            }
            ENDED => {
                c.ended = true;
                let code = page.at(word(s, EXIT_CODE)).load(Ordering::Relaxed);
                Ok((0, 0, kernel.objects.end(c.pid, code)))
            }
            STOPPED => {
                c.ended = true;
                served.stop(format!("child process {}: {}", c.pid, read_stop_text(page, s)));
                Ok((0, 0, false))
            }
            other => Err(format!("message {other}")),
        };
        let (result, second, waking) = match answer {
            Ok(answer) => answer,
            Err(e) => {
                // left posted, unanswered: the stop ends the run, and with it the child
                c.refused = true;
                served.stop(format!("child process {}: {e}", c.pid));
                continue;
            }
        };
        #[cfg(test)]
        served.answer_pause.before_the_answers_words();
        page.at(word(s, RESULT)).store(result, Ordering::Relaxed);
        page.at(word(s, SECOND)).store(second, Ordering::Relaxed);
        page.at(word(s, STATE)).store(ANSWERED, Ordering::Release);
        page.wake_every(word(s, STATE));
        wake |= waking;
    }
    let live: Vec<usize> = children
        .iter()
        .filter(|c| !c.gone && !c.ended)
        .map(|c| c.slot)
        .collect();
    drop(children);
    drop(kernel);
    if wake {
        // `Answer::wake` (a pipe written or read, an event set, a close that broke a pipe, an end): every live
        // process's wake count raised
        for s in live {
            page.at(word(s, WAKES)).fetch_add(1, Ordering::Release);
            page.wake_every(word(s, WAKES));
        }
        served.wakes.raise();
    }
}

/// A test's pause in one server before an answer's words (Sprint 363; plant 1's window): every `every`th answer waits
/// 1 ms there. With the state stored after the words, the client only waits longer; with it stored before them, the
/// client, spinning on its state, reads the last answer's RESULT at every pause. Off (0) unless a test sets it through
/// `Server::pause_before_answers`.
#[cfg(test)]
#[derive(Default)]
struct AnswerPause {
    every: AtomicU64,
    answers: AtomicU64,
}

#[cfg(test)]
impl AnswerPause {
    fn before_the_answers_words(&self) {
        let every = self.every.load(Ordering::Relaxed);
        if every != 0 && self.answers.fetch_add(1, Ordering::Relaxed) % every == every - 1 {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

/// The maps slot `s` posted, as offsets in its memory.
fn read_maps(page: &Words, s: usize) -> Vec<(u64, u64, u64)> {
    let n = (page.at(word(s, MAPS_LEN)).load(Ordering::Relaxed) as usize).min(MAPS_MAX);
    (0..n)
        .map(|k| {
            let w = |i: usize| page.at(word(s, MAPS + 3 * k + i)).load(Ordering::Relaxed);
            (w(0), w(1).wrapping_sub(guest_runtime::layout::IPA_BASE), w(2))
        })
        .collect()
}

/// The stop text slot `s` posted.
fn read_stop_text(page: &Words, s: usize) -> String {
    let len = (page.at(word(s, STOP_LEN)).load(Ordering::Relaxed) as usize).min(8 * STOP_WORDS);
    let bytes: Vec<u8> = (0..len.div_ceil(8))
        .flat_map(|k| page.at(word(s, STOP_TEXT + k)).load(Ordering::Relaxed).to_le_bytes())
        .take(len)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Each child not yet gone: a process that exited without its end or stop posted stops the run by name.
fn look_at_children(served: &Served) {
    let mut children = served.children.lock().unwrap_or_else(|e| e.into_inner());
    for c in children.iter_mut().filter(|c| !c.gone) {
        match c.process.try_wait() {
            Ok(Some(status)) => {
                c.gone = true;
                if !c.ended {
                    served.stop(format!("child process {}: gone without a word ({status})", c.pid));
                }
            }
            Ok(None) => {}
            Err(e) => served.stop(format!("child process {}: {e}", c.pid)),
        }
    }
}

/// A child's side of the page: its slot, posted and waited on.
pub struct Client {
    page: Words,
    slot: usize,
}

impl Client {
    /// The page of descriptor `page_fd`, which the child inherited, and its slot.
    pub fn open(page_fd: i32, slot: usize) -> Result<Client, String> {
        if slot >= SLOTS {
            return Err(format!("slot {slot} of a page of {SLOTS}"));
        }
        Ok(Client {
            page: Words::map(page_fd)?,
            slot,
        })
    }

    /// The maps, (VA, IPA, size) each, posted once at the child's start, before its first request, whose release of
    /// its state publishes them.
    pub fn post_maps(&self, maps: &[(u64, u64, u64)]) -> Result<(), String> {
        if maps.len() > MAPS_MAX {
            return Err(format!("{} maps, {MAPS_MAX} at most", maps.len()));
        }
        for (k, &(va, ipa, size)) in maps.iter().enumerate() {
            for (i, w) in [va, ipa, size].into_iter().enumerate() {
                self.page
                    .at(word(self.slot, MAPS + 3 * k + i))
                    .store(w, Ordering::Relaxed);
            }
        }
        self.page
            .at(word(self.slot, MAPS_LEN))
            .store(maps.len() as u64, Ordering::Relaxed);
        Ok(())
    }

    /// A KERNEL 33 request, (sub-operation, a1, a2, a3): its result and second word.
    pub fn request(&self, a: [u64; 4]) -> (u64, u64) {
        for (k, w) in a.into_iter().enumerate() {
            self.page.at(word(self.slot, ARGS + k)).store(w, Ordering::Relaxed);
        }
        self.round_trip(REQUEST)
    }

    /// The child's end, with its exit code: the server closes what it held and wakes every process.
    pub fn end(&self, exit_code: u64) {
        self.page
            .at(word(self.slot, EXIT_CODE))
            .store(exit_code, Ordering::Relaxed);
        self.round_trip(ENDED);
    }

    /// The child's stop, `why` its first 1,024 bytes: the server stops the run naming the child.
    pub fn stop(&self, why: &str) {
        let mut n = why.len().min(8 * STOP_WORDS);
        while !why.is_char_boundary(n) {
            n -= 1;
        }
        for (k, chunk) in why.as_bytes()[..n].chunks(8).enumerate() {
            let mut w = [0u8; 8];
            w[..chunk.len()].copy_from_slice(chunk);
            self.page
                .at(word(self.slot, STOP_TEXT + k))
                .store(u64::from_le_bytes(w), Ordering::Relaxed);
        }
        self.page
            .at(word(self.slot, STOP_LEN))
            .store(n as u64, Ordering::Relaxed);
        self.round_trip(STOPPED);
    }

    /// The message posted (its words written before), the doorbell raised, and the answer waited for: its result and
    /// second word, read after the state's acquire, which the server's release store follows the answer's words.
    fn round_trip(&self, message: u64) -> (u64, u64) {
        let s = self.slot;
        self.page.at(word(s, MESSAGE)).store(message, Ordering::Relaxed);
        self.page.at(word(s, STATE)).store(POSTED, Ordering::Release);
        ring(&self.page);
        let posted = Instant::now();
        loop {
            let seen = self.page.at(word(s, STATE)).load(Ordering::Acquire);
            if seen == ANSWERED {
                break;
            }
            if posted.elapsed() < ANSWER_SPIN {
                std::hint::spin_loop();
                continue;
            }
            self.page.wait(word(s, STATE), seen);
        }
        let answer = (
            self.page.at(word(s, RESULT)).load(Ordering::Relaxed),
            self.page.at(word(s, SECOND)).load(Ordering::Relaxed),
        );
        self.page.at(word(s, STATE)).store(IDLE, Ordering::Relaxed);
        answer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concurrent_memory::ConcurrentGuestMemory;
    use crate::kernel::{CLOSE, PIPE_CREATE, PIPE_READ, PIPE_WRITE, REF};
    use crate::monitor_code::{host_release_runtime_word, host_take_runtime_word};

    const REQUESTS: usize = 10_000;
    /// set in the second process: the page's descriptor, the memory's, the slot and the pipe's write end
    const PEER: &str = "W2M_CROSS_PROCESS_PEER";
    /// set for a run of the test alone, which holds the median under 10 µs
    const ALONE: &str = "W2M_ROUND_TRIP_ALONE";
    const NAME: &str = "cross_process::tests::ten_thousand_pipe_writes_from_another_process_arrive_whole_and_in_order";
    const PAUSED: &str =
        "cross_process::tests::an_answer_paused_before_its_words_is_read_only_once_its_words_are_written";
    const PARENT: u64 = 4096;
    const CHILD: u64 = 4100;
    /// where each process's request buffer is, by the guest's address
    const BUFFER_VA: u64 = 0x7000_0000;
    const CHILD_MEMORY: usize = 64 * 1024;
    /// the first process's memory, mapped as a guest's: its run table's runtime word, then its read buffer at READ_AT
    const READ_AT: u64 = guest_runtime::layout::RUN_TABLE + 0x10_0000;
    const OWN_MEMORY: usize = ((READ_AT + 0x4000).div_ceil(0x4000) * 0x4000) as usize;
    /// the second process's wait after an answer before its next request, so each request finds the server asleep on
    /// the doorbell, as a request in play does (376 a second at most, Change 4's bound, 2.7 ms apart): back to back, the
    /// next request came while the server was still past its answer and not yet waiting, and a request found with no
    /// wake from the doorbell went unseen (Sprint 363's plant 2 passed the 1 ms median at 5,208 ns that way)
    const REQUEST_GAP: Duration = Duration::from_micros(50);
    /// the p99's ceiling, half the server's doorbell timeout: a request whose wake is lost takes the whole timeout,
    /// while a loaded host's scheduling delays a round trip some milliseconds (the gates, no plant live: p99 4,479,000
    /// ns over 1,636 requests, which a 1 ms ceiling failed)
    const P99_CEILING: Duration = LOOK_EVERY.checked_div(2).unwrap();

    /// The stream the second process writes, byte `i`.
    fn stream_byte(i: u64) -> u8 {
        (i.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 56) as u8
    }

    /// The length of request `k`, 1 to 100 bytes.
    fn request_length(k: usize) -> usize {
        1 + (k * 7919 + 13) % 100
    }

    /// The card's judge (Sprint 363): this test runs its own binary again as a second process, as `shared.rs`' turn
    /// test does. The second process makes 10,000 PIPE_WRITE requests of 1 to 100 bytes through its slot, each
    /// answered by the server under `kernel` against a view of its memory; this process's thread makes the PIPE_READs
    /// under its runtime word and `kernel`. The bytes read equal the bytes written, in order. The round trip's median
    /// and p99 are printed. The median is held under 1 ms, a gross failure only, since the gates run the test beside
    /// others, and under 10 µs in a run alone (W2M_ROUND_TRIP_ALONE=1), the revisit line of the Decision "Sequence
    /// after Sprint 196". The p99 is held under P99_CEILING: a doorbell wake lost on a fraction of requests leaves
    /// each to the server's 100 ms timeout, which the median hides (plant 2 back to back: median 5,208 ns, p99
    /// 102,094,250 ns).
    #[test]
    fn ten_thousand_pipe_writes_from_another_process_arrive_whole_and_in_order() {
        the_judge(NAME, 0);
    }

    /// The same judge with the server pausing 1 ms before every 100th answer's words (`AnswerPause`), set in this
    /// process, so the gates hold the order of an answer's stores: with the state stored after the words the client
    /// waits out the pause; with it stored before them, the client reads a stale result at each pause, and the bytes
    /// read differ from the bytes written. The median's figures are the unpaused test's.
    #[test]
    fn an_answer_paused_before_its_words_is_read_only_once_its_words_are_written() {
        the_judge(PAUSED, 100);
    }

    /// The two-process judge, the server pausing before every `pause_every`th answer (0: never); the second process
    /// is this binary run again with `--exact name`.
    fn the_judge(name: &str, pause_every: u64) {
        if let Ok(peer) = std::env::var(PEER) {
            return write_the_stream(&peer);
        }
        let kernel = Arc::new(KernelLock::default());
        let own_memory = hypervisor::GuestMemory::new(OWN_MEMORY).unwrap();
        let own: ConcurrentGuestMemory<'_> = crate::guest::memory_view(&own_memory);
        let own_maps = [(BUFFER_VA, READ_AT, (OWN_MEMORY as u64) - READ_AT)];
        // this process's KERNEL 33, as a vCPU's service takes it: the runtime word, then `kernel`
        let with_kernel = |serve: &mut dyn FnMut(&mut crate::run::Kernel)| {
            assert!(host_take_runtime_word(own, 0), "the runtime word, free");
            let mut held = kernel.take(own, crate::VcpuPages::FIRST);
            serve(&mut held);
            drop(held);
            host_release_runtime_word(own);
        };
        let own_request = |pid: u64, a: [u64; 4]| {
            let mut answer = None;
            with_kernel(&mut |k| {
                answer = Some(k.objects.request(pid, a, &mut MappedMemoryView::new(own, &own_maps)));
            });
            answer.unwrap().unwrap()
        };
        // the pipe, its write end the child's alone, so the child's end breaks it
        let (read_end, write_end) = {
            with_kernel(&mut |k| {
                k.objects.add(PARENT);
                k.objects.add(CHILD);
            });
            let p = own_request(PARENT, [PIPE_CREATE, 0, 0, 0]);
            own_request(CHILD, [REF, p.second, 0, 0]);
            own_request(PARENT, [CLOSE, p.second, 0, 0]);
            (p.result, p.second)
        };
        let mut server = Server::start(kernel.clone()).unwrap();
        server.pause_before_answers(pause_every);
        let memory = anonymous_file(CHILD_MEMORY as u64).unwrap();
        let memory_fd = inheritable(&memory).unwrap();
        let page_fd = server.page_descriptor().unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name, "--nocapture"])
            .env(PEER, format!("{page_fd} {memory_fd} 0 {write_end}"))
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut out = child.stdout.take().unwrap();
        server.adopt(0, CHILD, child, &memory).unwrap();
        let deadline = Instant::now() + Duration::from_secs(600);
        let mut read = Vec::new();
        loop {
            let seen = server.wakes().now();
            let a = own_request(PARENT, [PIPE_READ, read_end, BUFFER_VA, 4096]);
            if a.result > 0 {
                let at = read.len();
                read.resize(at + a.result as usize, 0);
                own.read(READ_AT, &mut read[at..]);
                continue;
            }
            if a.second == 2 {
                break;
            }
            if let Some(stop) = server.stopped() {
                panic!("the server stopped the run: {stop}");
            }
            assert!(Instant::now() < deadline, "the pipe not broken after 600 s");
            server.wakes().wait_past(seen, Duration::from_secs(1));
        }
        let mut printed = String::new();
        std::io::Read::read_to_string(&mut out, &mut printed).unwrap();
        server.end();
        assert_eq!(server.stopped(), None, "the server's stop");
        let Some(line) = printed.lines().find(|l| l.starts_with("cross-process peer: ")) else {
            panic!("the second process printed no result:\n{printed}");
        };
        println!("{line}");
        let number_after = |key: &str| -> u64 {
            line.split(key)
                .nth(1)
                .unwrap()
                .split(' ')
                .next()
                .unwrap()
                .parse()
                .unwrap()
        };
        let (written, requests, median, p99) = (
            number_after("wrote "),
            number_after("bytes in "),
            number_after("median "),
            number_after("p99 "),
        );
        assert_eq!(read.len() as u64, written, "the bytes read against the bytes written");
        if let Some(at) = (0..read.len()).find(|&i| read[i] != stream_byte(i as u64)) {
            panic!(
                "byte {at} of {written} read 0x{:02x}, written 0x{:02x}: out of order",
                read[at],
                stream_byte(at as u64)
            );
        }
        // a wake lost on a fraction of requests falls back on the server's 100 ms timeout, which the median hides
        assert!(
            p99 < P99_CEILING.as_nanos() as u64,
            "p99 round trip {p99} ns, median {median} ns"
        );
        assert!(median < 1_000_000, "median round trip {median} ns");
        if pause_every == 0 && std::env::var(ALONE).is_ok() {
            assert!(median < 10_000, "median round trip {median} ns, alone");
        }
        assert_eq!(requests, REQUESTS as u64, "the requests made");
    }

    /// The second process: the stream written by 10,000 PIPE_WRITE requests, each continuing where the last one's
    /// result left it, REQUEST_GAP apart, then the end posted; the round trip of each request timed. More than 1% of
    /// the 10,000 at P99_CEILING or over decides the p99's check, and the requests stop there.
    fn write_the_stream(peer: &str) {
        let v: Vec<u64> = peer.split(' ').map(|w| w.parse().unwrap()).collect();
        let &[page_fd, memory_fd, slot, write_end] = &v[..] else {
            panic!("{PEER}={peer}");
        };
        let client = Client::open(page_fd as i32, slot as usize).unwrap();
        let memory = hypervisor::GuestMemory::shared(memory_fd as i32, CHILD_MEMORY).unwrap();
        let view = crate::guest::memory_view(&memory);
        client
            .post_maps(&[(BUFFER_VA, guest_runtime::layout::IPA_BASE, CHILD_MEMORY as u64)])
            .unwrap();
        let (mut written, mut ns, mut slow) = (0u64, Vec::with_capacity(REQUESTS), 0);
        let mut chunk = [0u8; 100];
        for k in 0..REQUESTS {
            let n = request_length(k);
            for (i, b) in chunk[..n].iter_mut().enumerate() {
                *b = stream_byte(written + i as u64);
            }
            view.write(0, &chunk[..n]);
            let t = Instant::now();
            let (taken, _) = client.request([PIPE_WRITE, write_end, BUFFER_VA, n as u64]);
            let elapsed = t.elapsed().as_nanos() as u64;
            ns.push(elapsed);
            written += taken;
            slow += usize::from(elapsed >= P99_CEILING.as_nanos() as u64);
            if slow > REQUESTS / 100 {
                break;
            }
            std::thread::sleep(REQUEST_GAP);
        }
        client.end(0);
        let made = ns.len();
        ns.sort_unstable();
        println!(
            "cross-process peer: wrote {written} bytes in {made} PIPE_WRITE requests; round trip median {} ns, p99 {} ns",
            ns[made / 2],
            ns[made * 99 / 100]
        );
    }

    /// Sprint 363: a child gone without its end or stop is found at the server's look, at most 100 ms after its last,
    /// and stops the run naming it.
    #[test]
    fn a_child_gone_without_a_word_stops_the_run_by_name() {
        let server = Server::start(Arc::new(KernelLock::default())).unwrap();
        let memory = anonymous_file(CHILD_MEMORY as u64).unwrap();
        let gone = std::process::Command::new("/usr/bin/true").spawn().unwrap();
        server.adopt(0, CHILD, gone, &memory).unwrap();
        let stop = server.wait_for_stop(Duration::from_secs(2));
        let stop = stop.expect("no stop within 2 s");
        assert!(
            stop.starts_with("child process 4100: gone without a word (exit status: 0)"),
            "{stop}"
        );
    }
}
