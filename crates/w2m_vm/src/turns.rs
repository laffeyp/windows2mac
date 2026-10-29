// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The turns of a run's processes (Sprint 197; the Decision "Sequence after Sprint 196"): kernel.py's `Machine.run`
//! over processes that are `w2m-vm` processes of their own. The first `w2m-vm` is the supervisor: it holds the kernel
//! objects (`Guest::kernel`) and gives the turn. A child runs only in its turn and hands it back at a KERNEL 33 request
//! (served by the supervisor against the child's memory, which both map shared), when it is idle, at a slice's end
//! while another process is due, at its end, or at a stop. One page of an unlinked file carries the turn: word TURN
//! holds the pid whose turn it is, waited on with `os_sync_wait_on_address` (`shared::Page`); each process has a slot
//! of words it writes before it hands the turn back.

use std::fs::File;
use std::os::fd::AsRawFd;
use std::process::{Child, Command, Stdio};

use crate::kernel::{Memory, Objects, Spawn};
use crate::machine::{Next, Turn};
use crate::shared::{Page, anonymous_file, inheritable};

/// The page: 64 KB, the turn word, then a slot of SLOT words per process, slot 0 the supervisor's.
pub const PAGE_SIZE: usize = 64 * 1024;
pub const TURN: usize = 0;
const SLOT: usize = 1024;
pub const SLOTS: usize = PAGE_SIZE / 8 / SLOT - 1;
/// A slot's words.
const PID: usize = 0;
const MESSAGE: usize = 1;
/// the request's sub-operation and arguments, then its answer
const ARGS: usize = 2;
const RESULT: usize = 6;
const SECOND: usize = 7;
/// set by the supervisor: every blocked thread checks its wait again when the process runs next (`wake_later`)
const WAKE: usize = 8;
const READY: usize = 9;
const TIMED: usize = 10;
const EXIT_CODE: usize = 11;
/// the stop's text: its length in bytes, then its bytes, 1,024 at most
const STOP_LEN: usize = 12;
const STOP_TEXT: usize = 13;
const STOP_WORDS: usize = 128;
/// the layout's maps, (VA, IPA, size) each: their count, then the triples, 256 at most
const MAPS_LEN: usize = STOP_TEXT + STOP_WORDS;
const MAPS: usize = MAPS_LEN + 1;
const MAPS_MAX: usize = 256;
/// the process's clock at its turn's end (`_counter_now`), and the machine's clock when its turn comes back
/// (`_machine_now`, the latest of the processes'), for `_catch_up`
const CLOCK: usize = MAPS + 3 * MAPS_MAX;
const MACHINE: usize = CLOCK + 1;
/// the earliest deadline of a blocked thread (u64::MAX for none), and whether a thread is blocked, at the turn's end:
/// readiness is computed from them when a turn is chosen (`ready`, `_deadline_passed`, vm_backend 3653-3669; Sprint 199)
const DEADLINE: usize = MACHINE + 1;
const BLOCKED: usize = MACHINE + 2;

/// `VM.ready` (vm_backend 3653-3662) for slot `s` from what it posted: a thread ready, its earliest deadline at or
/// before the machine's clock `now`, or a wake pending with a thread blocked.
fn slot_ready(page: &Page, s: usize, now: u64) -> bool {
    page.load(word(s, READY)) != 0
        || page.load(word(s, DEADLINE)) <= now
        || (page.load(word(s, WAKE)) != 0 && page.load(word(s, BLOCKED)) != 0)
}

/// `_machine_now` over the posted clocks and the asking process's own `clock`.
fn machine_clock(page: &Page, clock: u64) -> u64 {
    (0..SLOTS)
        .filter(|&s| page.load(word(s, PID)) != 0)
        .map(|s| page.load(word(s, CLOCK)))
        .chain(std::iter::once(clock))
        .max()
        .unwrap_or(clock)
}

/// What a process hands the turn back with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum Message {
    Request = 1,
    Idle = 2,
    Slice = 3,
    Ended = 4,
    Stopped = 5,
}

fn word(slot: usize, w: usize) -> usize {
    (1 + slot) * SLOT + w
}

/// What a process reports of itself at a hand-back: whether a thread is ready, whether one waits with a deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct State {
    pub ready: bool,
    pub timed: bool,
    /// the process's clock (`_counter_now`); 0 when not given
    pub clock: u64,
    /// the earliest deadline of a blocked, unsuspended thread, u64::MAX for none
    pub deadline: u64,
    /// a thread is blocked
    pub blocked: bool,
}

/// A process's state at its turn's end, written to its slot.
fn post_state(page: &Page, s: usize, state: State) {
    page.store(word(s, READY), u64::from(state.ready));
    page.store(word(s, TIMED), u64::from(state.timed));
    page.store(word(s, DEADLINE), state.deadline);
    page.store(word(s, BLOCKED), u64::from(state.blocked));
    if state.clock != 0 {
        page.store(word(s, CLOCK), state.clock);
    }
}

/// The guest's maps written to its slot, so the supervisor can find a VA in the shared memory.
fn post_maps(page: &Page, slot: usize, maps: &[(u64, u64, u64)]) -> Result<(), String> {
    if maps.len() > MAPS_MAX {
        return Err(format!("{} maps, more than a slot holds ({MAPS_MAX})", maps.len()));
    }
    page.store(word(slot, MAPS_LEN), maps.len() as u64);
    for (i, (va, ipa, n)) in maps.iter().enumerate() {
        page.store(word(slot, MAPS + 3 * i), *va);
        page.store(word(slot, MAPS + 3 * i + 1), *ipa);
        page.store(word(slot, MAPS + 3 * i + 2), *n);
    }
    Ok(())
}

/// A child's side: its slot and pid.
pub struct ChildLink {
    pub page: Page,
    pub slot: usize,
    pub pid: u64,
    pub supervisor: u64,
}

/// `sys/event.h` 86-93
#[repr(C)]
struct Kevent {
    ident: usize,
    filter: i16,
    flags: u16,
    fflags: u32,
    data: isize,
    udata: *mut std::ffi::c_void,
}

unsafe extern "C" {
    /// `sys/event.h` 371-375
    fn kqueue() -> i32;
    fn kevent(
        kq: i32,
        changes: *const Kevent,
        nchanges: i32,
        events: *mut Kevent,
        nevents: i32,
        timeout: *const u8,
    ) -> i32;
}

/// `sys/event.h` 72, 136, 142, 260
const EVFILT_PROC: i16 = -5;
const EV_ADD: u16 = 0x0001;
const EV_ONESHOT: u16 = 0x0010;
const NOTE_EXIT: u32 = 0x8000_0000;

/// A child ends the moment its supervisor does, mid-turn included, so no child outlives the run (Sprint 199;
/// reviewer-2's Medium 3 of its review of 196-198: a child checked its parent only while it waited for a turn): a
/// thread waits on the parent's exit with kqueue's EVFILT_PROC and NOTE_EXIT and ends the process.
pub fn end_with_parent() -> Result<(), String> {
    let parent = std::os::unix::process::parent_id();
    let kq = unsafe { kqueue() };
    if kq == -1 {
        return Err(format!("kqueue: {}", std::io::Error::last_os_error()));
    }
    let change = Kevent {
        ident: parent as usize,
        filter: EVFILT_PROC,
        flags: EV_ADD | EV_ONESHOT,
        fflags: NOTE_EXIT,
        data: 0,
        udata: std::ptr::null_mut(),
    };
    if unsafe { kevent(kq, &change, 1, std::ptr::null_mut(), 0, std::ptr::null()) } == -1 {
        return Err(format!(
            "kevent on the parent {parent}: {}",
            std::io::Error::last_os_error()
        ));
    }
    std::thread::spawn(move || {
        let mut event = Kevent {
            ident: 0,
            filter: 0,
            flags: 0,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        unsafe { kevent(kq, std::ptr::null(), 0, &mut event, 1, std::ptr::null()) };
        std::process::exit(1);
    });
    Ok(())
}

impl ChildLink {
    /// A child's side of the page its supervisor passed by descriptor; the supervisor's pid is in slot 0.
    pub fn open(page_fd: i32, slot: usize, pid: u64) -> Result<ChildLink, String> {
        if slot == 0 || slot >= SLOTS {
            return Err(format!("slot {slot} of a turn page of {SLOTS}"));
        }
        let page = Page::map(page_fd, PAGE_SIZE).map_err(|e| format!("the turn page: {e}"))?;
        let supervisor = page.load(word(0, PID));
        Ok(ChildLink {
            page,
            slot,
            pid,
            supervisor,
        })
    }

    /// Hands the turn to the supervisor with `message` and waits for it back; true when a wake is pending.
    fn hand_back(&self, message: Message, state: State) -> bool {
        let s = self.slot;
        // a request's turn goes on after its answer: the state posted at the last turn's end stands
        if message != Message::Request {
            post_state(&self.page, s, state);
        }
        self.page.store(word(s, MESSAGE), message as u64);
        self.page.post(TURN, self.supervisor);
        self.wait_for_turn();
        let wake = self.page.load(word(s, WAKE)) != 0;
        self.page.store(word(s, WAKE), 0);
        wake
    }

    /// Waits for this process's turn; a child whose supervisor is gone ends, so none outlives the run.
    fn wait_for_turn(&self) {
        let parent = std::os::unix::process::parent_id();
        if self
            .page
            .wait_until_alive(
                TURN,
                |t| t == self.pid,
                || std::os::unix::process::parent_id() == parent,
            )
            .is_none()
        {
            std::process::exit(1);
        }
    }

    /// Waits for the first turn, before the child loads anything: every stop it posts after is read (each request
    /// posts the guest's maps).
    pub fn first_turn(&self) {
        self.wait_for_turn();
    }

    /// The machine's clock as the supervisor wrote it when it gave this turn (`_machine_now`).
    pub fn machine(&self) -> u64 {
        self.page.load(word(self.slot, MACHINE))
    }

    /// A KERNEL 33 request served by the supervisor: (result, second, a wake pending here).
    pub fn request(&self, a: [u64; 4], maps: &[(u64, u64, u64)]) -> Result<(u64, u64, bool), String> {
        post_maps(&self.page, self.slot, maps)?;
        for (i, v) in a.iter().enumerate() {
            self.page.store(word(self.slot, ARGS + i), *v);
        }
        let wake = self.hand_back(Message::Request, State::default());
        if self.page.load(word(self.slot, STOP_LEN)) != 0 {
            return Err(self.stop_text());
        }
        Ok((
            self.page.load(word(self.slot, RESULT)),
            self.page.load(word(self.slot, SECOND)),
            wake,
        ))
    }

    /// Idle or at a slice's end: the turn back; true when a wake is pending.
    pub fn yield_turn(&self, message: Message, state: State) -> (bool, u64) {
        let wake = self.hand_back(message, state);
        (wake, self.page.load(word(self.slot, MACHINE)))
    }

    /// The process ended with `code`, or stopped: the supervisor told, and the turn not taken back.
    pub fn end(&self, code: u64) {
        self.page.store(word(self.slot, EXIT_CODE), code);
        self.page.store(word(self.slot, MESSAGE), Message::Ended as u64);
        self.page.post(TURN, self.supervisor);
    }

    pub fn stop(&self, why: &str) {
        write_text(&self.page, self.slot, why);
        self.page.store(word(self.slot, MESSAGE), Message::Stopped as u64);
        self.page.post(TURN, self.supervisor);
    }

    fn stop_text(&self) -> String {
        read_text(&self.page, self.slot)
    }
}

fn write_text(page: &Page, slot: usize, text: &str) {
    let bytes = &text.as_bytes()[..text.len().min(8 * STOP_WORDS)];
    page.store(word(slot, STOP_LEN), bytes.len() as u64);
    for (i, chunk) in bytes.chunks(8).enumerate() {
        let mut w = [0u8; 8];
        w[..chunk.len()].copy_from_slice(chunk);
        page.store(word(slot, STOP_TEXT + i), u64::from_le_bytes(w));
    }
}

fn read_text(page: &Page, slot: usize) -> String {
    let n = page.load(word(slot, STOP_LEN)) as usize;
    let bytes: Vec<u8> = (0..n.div_ceil(8))
        .flat_map(|i| page.load(word(slot, STOP_TEXT + i)).to_le_bytes())
        .take(n)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// A process's place in the machine: the supervisor (the first `w2m-vm`, alone until a PROCESS_CREATE) or a child.
pub enum Link {
    Supervisor(Supervisor),
    Child(ChildLink),
}

impl Link {
    /// The page and this process's slot; true too when a child has not yet started, which is ready (`not threads`).
    fn page_and_slot(&self) -> (&Page, usize, bool) {
        match self {
            Link::Supervisor(s) => (&s.page, 0, s.children.iter().any(|c| !c.started)),
            Link::Child(c) => (&c.page, c.slot, false),
        }
    }

    /// `Machine.others_ready` (kernel.py 326-327): another process has a thread to run now, `VM.ready` computed from
    /// what each posted against the machine's clock with this process's own `clock` (Sprint 199: before, the posted
    /// bit alone, so a child's passed deadline waited for its parent to idle).
    pub fn others_ready(&self, clock: u64) -> bool {
        let (page, me, unstarted) = self.page_and_slot();
        let now = machine_clock(page, clock);
        unstarted
            || (0..SLOTS)
                .filter(|&s| s != me && page.load(word(s, PID)) != 0)
                .any(|s| slot_ready(page, s, now))
    }

    /// `_other_process_due` (vm_backend 2750-2754): another process is ready or waits with a deadline.
    pub fn others_due(&self, clock: u64) -> bool {
        let (page, me, unstarted) = self.page_and_slot();
        let now = machine_clock(page, clock);
        unstarted
            || (0..SLOTS)
                .filter(|&s| s != me && page.load(word(s, PID)) != 0)
                .any(|s| slot_ready(page, s, now) || page.load(word(s, DEADLINE)) != u64::MAX)
    }
}

/// A child as the supervisor holds it.
pub struct ChildProcess {
    pub pid: u64,
    slot: usize,
    process: Child,
    memory_file: File,
    memory: Option<hypervisor::GuestMemory>,
    started: bool,
}

/// The supervisor's side.
pub struct Supervisor {
    pub page: Page,
    page_file: File,
    pub pid: u64,
    pub children: Vec<ChildProcess>,
    /// the process whose turn ended last (`Machine.run`'s `current`)
    current: u64,
    /// the run is on the instruction clock: a timed waiter runs at once, its clock warped (`_next`)
    instruction_clock: bool,
}

/// What the supervisor's own process does after a hand-over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Back {
    /// its turn again; true when a wake is pending for it; the machine's clock
    Run { wake: bool, machine: u64 },
    /// no process is ready and this one waits with a deadline: its timed waiter runs
    Timed { wake: bool, machine: u64 },
}

/// A child's memory as `kernel.rs` reads and writes it: the shared mapping, found through the maps the child posted.
struct ChildMemory<'a> {
    memory: &'a mut hypervisor::GuestMemory,
    maps: Vec<(u64, u64, u64)>,
}

impl ChildMemory<'_> {
    fn offset(&self, va: u64, n: usize) -> Result<usize, String> {
        self.maps
            .iter()
            .find(|(v, _, size)| (*v..v + size).contains(&va))
            .map(|(v, ipa, _)| (ipa - guest_runtime::layout::IPA_BASE + (va - v)) as usize)
            .filter(|at| at + n <= self.memory.len())
            .ok_or_else(|| format!("a child's kernel request at unmapped 0x{va:x}"))
    }
}

impl Memory for ChildMemory<'_> {
    fn read(&self, va: u64, n: usize) -> Result<Vec<u8>, String> {
        let at = self.offset(va, n)?;
        Ok(self.memory.as_slice()[at..at + n].to_vec())
    }
    fn write(&mut self, va: u64, data: &[u8]) -> Result<(), String> {
        let at = self.offset(va, data.len())?;
        self.memory.as_mut_slice()[at..at + data.len()].copy_from_slice(data);
        Ok(())
    }
}

impl Supervisor {
    pub fn new(pid: u64, instruction_clock: bool) -> Result<Supervisor, String> {
        let page_file = anonymous_file(PAGE_SIZE as u64).map_err(|e| format!("the turn page: {e}"))?;
        let page = Page::map(page_file.as_raw_fd(), PAGE_SIZE).map_err(|e| format!("the turn page: {e}"))?;
        page.store(TURN, pid);
        page.store(word(0, PID), pid);
        Ok(Supervisor {
            page,
            page_file,
            pid,
            children: Vec::new(),
            current: pid,
            instruction_clock,
        })
    }

    /// `spawner` (kernel.py 437-468): a child `w2m-vm` started with the turn page's and its memory's descriptors and
    /// its start as one JSON line; it builds its guest and waits for its turn.
    pub fn spawn(&mut self, child: &Spawn, start: serde_json::Value) -> Result<(), String> {
        let slot = 1 + self.children.len();
        if slot >= SLOTS {
            return Err(format!("a process past the turn page's {SLOTS} slots"));
        }
        let memory_file = anonymous_file(0).map_err(|e| format!("a child's memory: {e}"))?;
        let (page_fd, memory_fd) = (
            inheritable(&self.page_file).map_err(|e| e.to_string())?,
            inheritable(&memory_file).map_err(|e| e.to_string())?,
        );
        self.page.store(word(slot, PID), child.pid);
        let exe = std::env::current_exe().map_err(|e| format!("w2m-vm's own path: {e}"))?;
        let mut process = Command::new(exe)
            .args([
                "--child".to_string(),
                page_fd.to_string(),
                memory_fd.to_string(),
                slot.to_string(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .map_err(|e| format!("a child w2m-vm: {e}"))?;
        use std::io::Write;
        let mut line = start.to_string();
        line.push('\n');
        process
            .stdin
            .take()
            .ok_or("a child w2m-vm's standard input")?
            .write_all(line.as_bytes())
            .map_err(|e| format!("a child w2m-vm's start: {e}"))?;
        self.children.push(ChildProcess {
            pid: child.pid,
            slot,
            process,
            memory_file,
            memory: None,
            started: false,
        });
        Ok(())
    }

    /// Every child's next run sees a wake (`Machine.wake`, `wake_later`).
    pub fn wake_children(&self) {
        for c in &self.children {
            self.page.store(word(c.slot, WAKE), 1);
        }
    }

    /// `Machine.run` from the supervisor's turn's end (`message` Idle or Slice, `own` its state): the next turn, given
    /// and served until it comes back to the supervisor's own process. `objects` are the kernel's tables.
    pub fn hand_over(&mut self, objects: &mut Objects, own: State) -> Result<Back, String> {
        let mut own = own;
        let mut own_wake = false;
        loop {
            post_state(
                &self.page,
                0,
                State {
                    ready: own.ready || own_wake,
                    ..own
                },
            );
            // `_machine_now` (vm_backend 3671-3678): the latest clock of the processes that have run
            let clocks: Vec<u64> = std::iter::once(own.clock)
                .chain(self.children.iter().map(|c| self.page.load(word(c.slot, CLOCK))))
                .collect();
            let machine = crate::machine::machine_now(&clocks);
            // readiness when the turn is chosen (`VM.ready`, 3653-3669; Sprint 199): a thread ready, a deadline passed
            // on the machine's clock, a wake pending with a thread blocked
            let mut live = vec![Turn {
                pid: self.pid,
                started: true,
                ready: own.ready || own.deadline <= machine || (own_wake && own.blocked),
                timed: own.timed,
            }];
            for c in &self.children {
                let s = c.slot;
                live.push(Turn {
                    pid: c.pid,
                    started: c.started,
                    ready: slot_ready(&self.page, s, machine),
                    timed: self.page.load(word(s, TIMED)) != 0,
                });
            }
            let chosen = crate::machine::next(&live, self.current);
            // `_next` (kernel.py 394-397): a timed waiter runs after a millisecond on the wall clock
            if matches!(chosen, Next::Timed(_)) && !self.instruction_clock {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            let pid = match chosen {
                Next::Run(p) if p == self.pid => {
                    self.current = self.pid;
                    return Ok(Back::Run {
                        wake: own_wake,
                        machine,
                    });
                }
                Next::Timed(p) if p == self.pid => {
                    self.current = self.pid;
                    return Ok(Back::Timed {
                        wake: own_wake,
                        machine,
                    });
                }
                Next::Run(p) | Next::Timed(p) => p,
                Next::Deadlock => {
                    return Err("every thread of every process waits with no timeout (a deadlock)".into());
                }
            };
            self.current = pid;
            let i = self.children.iter().position(|c| c.pid == pid).unwrap();
            self.children[i].started = true;
            self.page.store(word(self.children[i].slot, MACHINE), machine);
            // the child's turn, served until it hands the turn back for good (idle, slice, end, stop)
            loop {
                self.page.post(TURN, pid);
                let me = self.pid;
                let child = &mut self.children[i].process;
                if self
                    .page
                    .wait_until_alive(TURN, |t| t == me, || matches!(child.try_wait(), Ok(None)))
                    .is_none()
                {
                    let status = child.try_wait().ok().flatten();
                    return Err(format!(
                        "child process {pid}: its w2m-vm ended without a word to the supervisor ({status:?})"
                    ));
                }
                let slot = self.children[i].slot;
                let message = self.page.load(word(slot, MESSAGE));
                if message == Message::Request as u64 {
                    let a: [u64; 4] = std::array::from_fn(|k| self.page.load(word(slot, ARGS + k)));
                    match self.serve(objects, i, a) {
                        Ok((result, second, wake, spawn)) => {
                            self.page.store(word(slot, RESULT), result);
                            self.page.store(word(slot, SECOND), second);
                            if wake {
                                own_wake = true;
                                self.wake_children();
                            }
                            if let Some((child, start)) = spawn {
                                self.spawn(&child, start)?;
                            }
                        }
                        Err(why) => {
                            write_text(&self.page, slot, &why);
                            return Err(format!("child process {pid}: {why}"));
                        }
                    }
                    continue;
                }
                if message == Message::Idle as u64 || message == Message::Slice as u64 {
                    break;
                }
                if message == Message::Ended as u64 {
                    let code = self.page.load(word(slot, EXIT_CODE));
                    objects.end(pid, code);
                    own_wake = true;
                    self.wake_children();
                    let mut done = self.children.remove(i);
                    let _ = done.process.wait();
                    break;
                }
                // `_child_stopped` (kernel.py 461-466)
                return Err(format!("child process {pid}: {}", read_text(&self.page, slot)));
            }
            own.ready = own.ready || own_wake;
        }
    }

    /// A child's KERNEL 33 request against its shared memory: (result, second, wake, a child to start).
    #[allow(clippy::type_complexity)]
    fn serve(
        &mut self,
        objects: &mut Objects,
        i: usize,
        a: [u64; 4],
    ) -> Result<(u64, u64, bool, Option<(Spawn, serde_json::Value)>), String> {
        let c = &mut self.children[i];
        let maps: Vec<(u64, u64, u64)> = (0..self.page.load(word(c.slot, MAPS_LEN)) as usize)
            .map(|k| {
                (
                    self.page.load(word(c.slot, MAPS + 3 * k)),
                    self.page.load(word(c.slot, MAPS + 3 * k + 1)),
                    self.page.load(word(c.slot, MAPS + 3 * k + 2)),
                )
            })
            .collect();
        if c.memory.is_none() {
            let len = c.memory_file.metadata().map_err(|e| e.to_string())?.len() as usize;
            c.memory =
                Some(hypervisor::GuestMemory::shared(c.memory_file.as_raw_fd(), len).map_err(|e| e.to_string())?);
        }
        let mut memory = ChildMemory {
            memory: c.memory.as_mut().unwrap(),
            maps,
        };
        let answer = objects.request(c.pid, a, &mut memory)?;
        if answer.spawn.is_some() {
            return Err("a grandchild process: not served yet".into());
        }
        Ok((answer.result, answer.second, answer.wake, None))
    }

    /// The run ended: every child process is stopped and waited for.
    pub fn end(&mut self) {
        for c in &mut self.children {
            let _ = c.process.kill();
            let _ = c.process.wait();
        }
        self.children.clear();
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        self.end();
    }
}
