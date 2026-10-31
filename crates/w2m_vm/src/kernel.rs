// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! What Windows processes share, for one process (Sprint 193): `src/w2m/kernel.py`'s objects (Sprint 055) in Rust, the
//! named events, the pipes and the process's own record, which the guest runtime reaches with host request KERNEL 33
//! (`tools/guest_runtime/sync_objects.c`, `processes.c`); the Windows rules stay in the guest.
//!
//! From Sprint 197 the tables are `Machine`'s for every process of a run: a record per process (its exit code, the
//! handles others hold on it, the handles it holds), PROCESS_CREATE's ids and inherited handles, PROCESS_STATE, and a
//! process's end, which closes every handle it held (`_release`). Starting the child's VM is the caller's: the answer
//! carries what `spawner` needs (`Spawn`).

use std::collections::BTreeMap;

pub const EVENT_OPEN: u64 = 1;
pub const EVENT_STATE: u64 = 2;
pub const EVENT_TAKE: u64 = 3;
pub const EVENT_SET: u64 = 4;
pub const EVENT_RESET: u64 = 5;
pub const CLOSE: u64 = 6;
pub const PIPE_CREATE: u64 = 7;
pub const PIPE_READ: u64 = 8;
pub const PIPE_WRITE: u64 = 9;
pub const PROCESS_CREATE: u64 = 10;
pub const PROCESS_STATE: u64 = 11;
pub const REF: u64 = 12;
/// kernel.py 61, 70-71
pub const STILL_ACTIVE: u64 = 259;
pub const OBJECT_IDS: u64 = 1 << 32;
pub const PIPE_CAPACITY: usize = 4096;
/// handle kinds in PROCESS_CREATE's inherited list (kernel.py 63-67, runtime.h 310-317)
pub use crate::process::KIND_STANDARD;
pub const KIND_DIRECTORY: u64 = 102;
/// `next_pid` (kernel.py 113): the first process's id less 4 is the floor of `allocate_ids`
const FIRST_PID: u64 = 4096;
/// `next_tid` (114) and its step per process (`allocate_ids`, 142-147)
const FIRST_TID: u64 = 4;
const TID_STEP: u64 = 0x10_0000;

/// A process's record (`Process`, kernel.py 89-97): its exit code (STILL_ACTIVE while it runs), the handles other
/// processes hold on it, and the handles it holds, by object or process id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    pub exit_code: u64,
    pub handles: i64,
    pub held: BTreeMap<u64, i64>,
}

impl Default for Process {
    fn default() -> Process {
        Process {
            exit_code: STILL_ACTIVE,
            handles: 0,
            held: BTreeMap::new(),
        }
    }
}

/// What a child's start needs (`spawner`'s arguments, kernel.py 453-454): its command line, ids, the inherited
/// entries but the directory (handle, kind, object id), and the current directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spawn {
    pub command_line: String,
    pub pid: u64,
    pub tid: u64,
    pub inherited: Vec<(u64, u64, u64)>,
    pub directory: String,
}

/// Guest memory as a request reads and writes it.
pub trait Memory {
    fn read(&self, va: u64, n: usize) -> Result<Vec<u8>, String>;
    fn write(&mut self, va: u64, data: &[u8]) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Object {
    Event {
        name: String,
        manual: bool,
        signaled: bool,
        handles: i64,
    },
    Pipe {
        data: Vec<u8>,
        readers: i64,
        writers: i64,
    },
}

/// The objects a run's processes share (kernel.py's `Machine` tables).
#[derive(Debug, Clone, Default)]
pub struct Objects {
    objects: BTreeMap<u64, Object>,
    /// an end's id -> the pipe's id, whether it reads
    pipe_ends: BTreeMap<u64, (u64, bool)>,
    names: BTreeMap<String, u64>,
    next_id: u64,
    next_tid: u64,
    /// process id -> its record
    pub processes: BTreeMap<u64, Process>,
}

/// A request's answer: the result, the second word, whether every blocked thread of every process checks its wait
/// again (`Machine.wake`), and for PROCESS_CREATE the child to start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub result: u64,
    pub second: u64,
    pub wake: bool,
    pub spawn: Option<Spawn>,
}

fn answer(result: u64, second: u64) -> Answer {
    Answer {
        result,
        second,
        wake: false,
        spawn: None,
    }
}

fn waking(result: u64, second: u64) -> Answer {
    Answer {
        wake: true,
        ..answer(result, second)
    }
}

/// `read_text` (vm_backend 2298): `chars` UTF-16 units at `va`.
fn read_text(mem: &dyn Memory, va: u64, chars: u64) -> Result<String, String> {
    let raw = mem.read(va, 2 * chars as usize)?;
    let units: Vec<u16> = raw.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
    Ok(String::from_utf16_lossy(&units))
}

impl Objects {
    pub fn new() -> Objects {
        Objects {
            next_id: OBJECT_IDS,
            next_tid: FIRST_TID,
            ..Objects::default()
        }
    }

    /// A process joins the run (`add`, kernel.py 133-140) under the id it was built with.
    pub fn add(&mut self, pid: u64) {
        self.processes.entry(pid).or_default();
    }

    /// `allocate_ids` (142-147): a new process's id, the highest so far plus 4, and its first thread's.
    fn allocate_ids(&mut self) -> (u64, u64) {
        let pid = self.processes.keys().next_back().map_or(FIRST_PID, |p| p + 4);
        self.next_tid += TID_STEP;
        (pid, self.next_tid)
    }

    /// The handles process `pid` holds, by object or process id.
    fn held(&mut self, pid: u64) -> &mut BTreeMap<u64, i64> {
        &mut self.processes.entry(pid).or_default().held
    }

    /// A process ended (kernel.py 342-348, `_release` 319-324): its exit code kept, every handle it held closed; true:
    /// every blocked thread checks its wait again.
    pub fn end(&mut self, pid: u64, exit_code: u64) -> bool {
        let held = {
            let p = self.processes.entry(pid).or_default();
            p.exit_code = exit_code;
            std::mem::take(&mut p.held)
        };
        for (oid, n) in held {
            for _ in 0..n {
                self.close(oid);
            }
        }
        true
    }

    fn new_object(&mut self, obj: Object) -> u64 {
        let oid = self.next_id;
        self.next_id += 1;
        self.objects.insert(oid, obj);
        oid
    }

    /// `_request` (kernel.py 194-281) for process `pid`; `a` is (sub-operation, a1, a2, a3). Err is the Python's
    /// Unimplemented: a stop by name.
    pub fn request(&mut self, pid: u64, a: [u64; 4], mem: &mut dyn Memory) -> Result<Answer, String> {
        let [sub, a1, a2, a3] = a;
        match sub {
            EVENT_OPEN => {
                let name = read_text(mem, a1, a2)?;
                if let Some(&oid) = self.names.get(&name) {
                    if let Some(Object::Event { handles, .. }) = self.objects.get_mut(&oid) {
                        *handles += 1;
                    }
                    *self.held(pid).entry(oid).or_default() += 1;
                    return Ok(answer(oid, 1));
                }
                let oid = self.new_object(Object::Event {
                    name: name.clone(),
                    manual: a3 & 1 != 0,
                    signaled: a3 & 2 != 0,
                    handles: 1,
                });
                self.names.insert(name, oid);
                *self.held(pid).entry(oid).or_default() += 1;
                Ok(answer(oid, 0))
            }
            EVENT_STATE | EVENT_TAKE | EVENT_SET | EVENT_RESET => {
                let Some(Object::Event { manual, signaled, .. }) = self.objects.get_mut(&a1) else {
                    return Err(format!("kernel request {sub} on {a1}, which is not an event"));
                };
                match sub {
                    EVENT_STATE => return Ok(answer(u64::from(*signaled), 0)),
                    EVENT_TAKE if !*manual => *signaled = false,
                    EVENT_SET => {
                        *signaled = true;
                        return Ok(waking(0, 0));
                    }
                    EVENT_RESET => *signaled = false,
                    _ => {}
                }
                Ok(answer(0, 0))
            }
            CLOSE => {
                let held = self.held(pid).entry(a1).or_default();
                if *held <= 0 {
                    return Err(format!(
                        "a close of host object {a1}, which process {pid} holds no handle on"
                    ));
                }
                *held -= 1;
                let wake = self.close(a1);
                Ok(Answer { wake, ..answer(0, 0) })
            }
            REF => {
                self.reference(a1)?;
                *self.held(pid).entry(a1).or_default() += 1;
                Ok(answer(0, 0))
            }
            PIPE_CREATE => {
                let pipe = self.new_object(Object::Pipe {
                    data: Vec::new(),
                    readers: 1,
                    writers: 1,
                });
                let (r, w) = (self.next_id, self.next_id + 1);
                self.next_id += 2;
                self.pipe_ends.insert(r, (pipe, true));
                self.pipe_ends.insert(w, (pipe, false));
                *self.held(pid).entry(r).or_default() += 1;
                *self.held(pid).entry(w).or_default() += 1;
                Ok(answer(r, w))
            }
            PIPE_READ | PIPE_WRITE => {
                let Some(&(pipe, _)) = self.pipe_ends.get(&a1) else {
                    return Err(format!("kernel request {sub} on {a1}, which is not a pipe end"));
                };
                let Some(Object::Pipe { data, readers, writers }) = self.objects.get_mut(&pipe) else {
                    return Err(format!("pipe {pipe} is gone"));
                };
                if sub == PIPE_WRITE {
                    if *readers == 0 {
                        return Err("a write to a pipe no process reads (ERROR_NO_DATA): not in the rows".into());
                    }
                    let n = (a3 as usize).min(PIPE_CAPACITY - data.len());
                    data.extend(mem.read(a2, n)?);
                    return Ok(waking(n as u64, 0));
                }
                if data.is_empty() {
                    return Ok(answer(0, if *writers == 0 { 2 } else { 1 }));
                }
                let n = (a3 as usize).min(data.len());
                let taken: Vec<u8> = data.drain(..n).collect();
                mem.write(a2, &taken)?;
                Ok(waking(n as u64, 0))
            }
            // kernel.py 274-276: a process's exit code, STILL_ACTIVE while it runs, ~0 for no such process
            PROCESS_STATE => Ok(answer(self.processes.get(&a1).map_or(u64::MAX, |p| p.exit_code), 0)),
            // kernel.py 258-273
            PROCESS_CREATE => {
                let command_line = read_text(mem, a1, a2 & 0xFFFF_FFFF)?;
                let count = (a2 >> 32) as usize;
                let raw = mem.read(a3, 24 * count)?;
                let listed: Vec<(u64, u64, u64)> = raw
                    .as_chunks::<24>()
                    .0
                    .iter()
                    .map(|e| {
                        let w = |i: usize| u64::from_le_bytes(e[8 * i..8 * i + 8].try_into().unwrap());
                        (w(0), w(1), w(2))
                    })
                    .collect();
                let directories: Vec<&(u64, u64, u64)> =
                    listed.iter().filter(|(_, k, _)| *k == KIND_DIRECTORY).collect();
                let &[&(chars, _, at)] = directories.as_slice() else {
                    return Err(format!(
                        "a process start that names {} current directories",
                        directories.len()
                    ));
                };
                let directory = read_text(mem, at, chars)?;
                let inherited: Vec<(u64, u64, u64)> =
                    listed.into_iter().filter(|(_, k, _)| *k != KIND_DIRECTORY).collect();
                let (child, tid) = self.allocate_ids();
                self.add(child);
                for &(_, kind, oid) in &inherited {
                    if kind < KIND_STANDARD {
                        self.reference(oid)?;
                        *self.held(child).entry(oid).or_default() += 1;
                    }
                }
                self.processes.get_mut(&child).unwrap().handles = 1;
                *self.held(pid).entry(child).or_default() += 1;
                Ok(Answer {
                    spawn: Some(Spawn {
                        command_line,
                        pid: child,
                        tid,
                        inherited,
                        directory,
                    }),
                    ..waking(child, tid)
                })
            }
            _ => Err(format!("kernel request {sub}")),
        }
    }

    /// `_ref` (283-296)
    fn reference(&mut self, oid: u64) -> Result<(), String> {
        if let Some(&(pipe, reads)) = self.pipe_ends.get(&oid) {
            if let Some(Object::Pipe { readers, writers, .. }) = self.objects.get_mut(&pipe) {
                *if reads { readers } else { writers } += 1;
            }
            return Ok(());
        }
        if let Some(p) = self.processes.get_mut(&oid) {
            p.handles += 1;
            return Ok(());
        }
        match self.objects.get_mut(&oid) {
            Some(Object::Event { handles, .. }) => {
                *handles += 1;
                Ok(())
            }
            _ => Err(format!("a reference to host object {oid}, which no process holds")),
        }
    }

    /// `_close` (298-317): true when a pipe lost its last writer, which wakes a reader to see it broken.
    fn close(&mut self, oid: u64) -> bool {
        if let Some(&(pipe, reads)) = self.pipe_ends.get(&oid) {
            if let Some(Object::Pipe { readers, writers, .. }) = self.objects.get_mut(&pipe) {
                if reads {
                    *readers -= 1;
                } else {
                    *writers -= 1;
                    return *writers == 0;
                }
            }
            return false;
        }
        if let Some(p) = self.processes.get_mut(&oid) {
            p.handles -= 1;
            return false;
        }
        if let Some(Object::Event { name, handles, .. }) = self.objects.get_mut(&oid) {
            *handles -= 1;
            if *handles == 0 {
                let name = name.clone();
                self.objects.remove(&oid);
                self.names.remove(&name);
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A guest memory of one region at 0x1000.
    struct Region(Vec<u8>);

    impl Memory for Region {
        fn read(&self, va: u64, n: usize) -> Result<Vec<u8>, String> {
            let at = (va - 0x1000) as usize;
            Ok(self.0[at..at + n].to_vec())
        }
        fn write(&mut self, va: u64, data: &[u8]) -> Result<(), String> {
            let at = (va - 0x1000) as usize;
            self.0[at..at + data.len()].copy_from_slice(data);
            Ok(())
        }
    }

    fn with_name(name: &str) -> Region {
        let mut mem = Region(vec![0; 0x2000]);
        let bytes: Vec<u8> = name.encode_utf16().flat_map(u16::to_le_bytes).collect();
        mem.0[..bytes.len()].copy_from_slice(&bytes);
        mem
    }

    /// kernel.py 197-208: a new name is a new object, 0 second; the same name again is the same id with 1, a handle
    /// more; SET signals and wakes, TAKE clears an auto-reset event, STATE reads it.
    #[test]
    fn a_named_event_is_found_again_and_signaled() {
        let (mut k, mut mem) = (Objects::new(), with_name("w2m"));
        let a = k.request(4096, [EVENT_OPEN, 0x1000, 3, 0], &mut mem).unwrap();
        assert_eq!((a.result, a.second), (OBJECT_IDS, 0));
        let b = k.request(4096, [EVENT_OPEN, 0x1000, 3, 0], &mut mem).unwrap();
        assert_eq!((b.result, b.second), (OBJECT_IDS, 1));
        assert!(k.request(4096, [EVENT_SET, OBJECT_IDS, 0, 0], &mut mem).unwrap().wake);
        assert_eq!(
            k.request(4096, [EVENT_STATE, OBJECT_IDS, 0, 0], &mut mem)
                .unwrap()
                .result,
            1
        );
        k.request(4096, [EVENT_TAKE, OBJECT_IDS, 0, 0], &mut mem).unwrap();
        assert_eq!(
            k.request(4096, [EVENT_STATE, OBJECT_IDS, 0, 0], &mut mem)
                .unwrap()
                .result,
            0
        );
    }

    /// kernel.py 232-257: a pipe's ends after the pipe's id; a write takes what fits in 4,096 bytes; a read takes what
    /// is there, 1 when empty, 2 once the last writer has closed.
    #[test]
    fn a_pipe_holds_four_kilobytes_and_breaks_when_its_writer_closes() {
        let (mut k, mut mem) = (Objects::new(), Region(vec![7; 0x2000]));
        let p = k.request(4096, [PIPE_CREATE, 0, 0, 0], &mut mem).unwrap();
        let (r, w) = (p.result, p.second);
        assert_eq!((r, w), (OBJECT_IDS + 1, OBJECT_IDS + 2));
        assert_eq!(
            k.request(4096, [PIPE_WRITE, w, 0x1000, 5000], &mut mem).unwrap().result,
            4096
        );
        assert_eq!(
            k.request(4096, [PIPE_READ, r, 0x1000, 100], &mut mem).unwrap().result,
            100
        );
        assert_eq!(
            k.request(4096, [PIPE_READ, r, 0x1000, 8000], &mut mem).unwrap().result,
            3996
        );
        assert_eq!(k.request(4096, [PIPE_READ, r, 0x1000, 8], &mut mem).unwrap().second, 1);
        assert!(k.request(4096, [CLOSE, w, 0, 0], &mut mem).unwrap().wake);
        assert_eq!(k.request(4096, [PIPE_READ, r, 0x1000, 8], &mut mem).unwrap().second, 2);
    }

    /// kernel.py 142-147, 258-276, 319-348: a child's ids after the parent's (4100, 0x100004); an inherited event gains
    /// the child's handle; PROCESS_STATE answers STILL_ACTIVE until the child ends, then its exit code, ~0 for no such
    /// process; the child's end closes what it held.
    #[test]
    fn a_child_starts_holds_what_it_inherits_and_ends_with_its_code() {
        let mut mem = Region(vec![0; 0x2000]);
        let put_text = |mem: &mut Region, at: usize, s: &str| {
            let b: Vec<u8> = s.encode_utf16().flat_map(u16::to_le_bytes).collect();
            mem.0[at..at + b.len()].copy_from_slice(&b);
        };
        put_text(&mut mem, 0, "ev");
        put_text(&mut mem, 0x100, "probe.exe -c 1");
        put_text(&mut mem, 0x400, "C:\\w2m\\app");
        let mut k = Objects::new();
        k.add(4096);
        let ev = k.request(4096, [EVENT_OPEN, 0x1000, 2, 0], &mut mem).unwrap().result;
        // the list: the event (kind 9), the error mode (101), the directory (102, 10 characters at 0x1400)
        for (i, e) in [(0, 9, ev), (0, 101, 0), (10, KIND_DIRECTORY, 0x1400)]
            .iter()
            .enumerate()
        {
            for (j, w) in [e.0, e.1, e.2].iter().enumerate() {
                let at = 0x300 + 24 * i + 8 * j;
                mem.0[at..at + 8].copy_from_slice(&u64::to_le_bytes(*w));
            }
        }
        let a = k
            .request(4096, [PROCESS_CREATE, 0x1100, 14 | 3 << 32, 0x1300], &mut mem)
            .unwrap();
        assert_eq!((a.result, a.second, a.wake), (4100, 0x10_0004, true));
        assert_eq!(
            a.spawn,
            Some(Spawn {
                command_line: "probe.exe -c 1".into(),
                pid: 4100,
                tid: 0x10_0004,
                inherited: vec![(0, 9, ev), (0, 101, 0)],
                directory: "C:\\w2m\\app".into(),
            })
        );
        assert_eq!(k.processes[&4100].held.get(&ev), Some(&1));
        assert_eq!(
            k.request(4096, [PROCESS_STATE, 4100, 0, 0], &mut mem).unwrap().result,
            STILL_ACTIVE
        );
        assert_eq!(
            k.request(4096, [PROCESS_STATE, 4104, 0, 0], &mut mem).unwrap().result,
            u64::MAX
        );
        assert!(k.end(4100, 7));
        assert_eq!(
            k.request(4096, [PROCESS_STATE, 4100, 0, 0], &mut mem).unwrap().result,
            7
        );
        assert!(k.processes[&4100].held.is_empty());
        // the parent's handle on the event is the last: its close removes the name
        k.request(4096, [CLOSE, ev, 0, 0], &mut mem).unwrap();
        assert_eq!(k.request(4096, [EVENT_OPEN, 0x1000, 2, 0], &mut mem).unwrap().second, 0);
    }
}
