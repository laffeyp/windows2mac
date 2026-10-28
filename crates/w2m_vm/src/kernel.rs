// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! What Windows processes share, for one process (Sprint 193): `src/w2m/kernel.py`'s objects (Sprint 055) in Rust, the
//! named events, the pipes and the process's own record, which the guest runtime reaches with host request KERNEL 33
//! (`tools/guest_runtime/sync_objects.c`, `processes.c`); the Windows rules stay in the guest. A second process
//! (PROCESS_CREATE) is the next card's: it stops by name here.

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

/// The objects one process shares (kernel.py's `Machine` tables for a machine of one process).
#[derive(Debug, Clone, Default)]
pub struct Objects {
    objects: BTreeMap<u64, Object>,
    /// an end's id -> the pipe's id, whether it reads
    pipe_ends: BTreeMap<u64, (u64, bool)>,
    names: BTreeMap<String, u64>,
    next_id: u64,
    /// object or process id -> the handles this process holds
    held: BTreeMap<u64, i64>,
    /// handles on this process itself (OpenProcess, REF)
    own_handles: i64,
}

/// A request's answer: the result, the second word, and whether every blocked thread checks its wait again
/// (`Machine.wake`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Answer {
    pub result: u64,
    pub second: u64,
    pub wake: bool,
}

fn answer(result: u64, second: u64) -> Answer {
    Answer {
        result,
        second,
        wake: false,
    }
}

impl Objects {
    pub fn new() -> Objects {
        Objects {
            next_id: OBJECT_IDS,
            ..Objects::default()
        }
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
                // `read_text` (vm_backend 2298): a2 UTF-16 units
                let raw = mem.read(a1, 2 * a2 as usize)?;
                let units: Vec<u16> = raw.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
                let name = String::from_utf16_lossy(&units);
                if let Some(&oid) = self.names.get(&name) {
                    if let Some(Object::Event { handles, .. }) = self.objects.get_mut(&oid) {
                        *handles += 1;
                    }
                    *self.held.entry(oid).or_default() += 1;
                    return Ok(answer(oid, 1));
                }
                let oid = self.new_object(Object::Event {
                    name: name.clone(),
                    manual: a3 & 1 != 0,
                    signaled: a3 & 2 != 0,
                    handles: 1,
                });
                self.names.insert(name, oid);
                *self.held.entry(oid).or_default() += 1;
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
                        return Ok(Answer {
                            result: 0,
                            second: 0,
                            wake: true,
                        });
                    }
                    EVENT_RESET => *signaled = false,
                    _ => {}
                }
                Ok(answer(0, 0))
            }
            CLOSE => {
                let held = self.held.entry(a1).or_default();
                if *held <= 0 {
                    return Err(format!(
                        "a close of host object {a1}, which process {pid} holds no handle on"
                    ));
                }
                *held -= 1;
                let wake = self.close(pid, a1);
                Ok(Answer {
                    result: 0,
                    second: 0,
                    wake,
                })
            }
            REF => {
                self.reference(pid, a1)?;
                *self.held.entry(a1).or_default() += 1;
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
                *self.held.entry(r).or_default() += 1;
                *self.held.entry(w).or_default() += 1;
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
                    return Ok(Answer {
                        result: n as u64,
                        second: 0,
                        wake: true,
                    });
                }
                if data.is_empty() {
                    return Ok(answer(0, if *writers == 0 { 2 } else { 1 }));
                }
                let n = (a3 as usize).min(data.len());
                let taken: Vec<u8> = data.drain(..n).collect();
                mem.write(a2, &taken)?;
                Ok(Answer {
                    result: n as u64,
                    second: 0,
                    wake: true,
                })
            }
            PROCESS_STATE => Ok(answer(if a1 == pid { STILL_ACTIVE } else { u64::MAX }, 0)),
            PROCESS_CREATE => Err("a second process (CreateProcessW): the next card's".into()),
            _ => Err(format!("kernel request {sub}")),
        }
    }

    /// `_ref` (283-296)
    fn reference(&mut self, pid: u64, oid: u64) -> Result<(), String> {
        if let Some(&(pipe, reads)) = self.pipe_ends.get(&oid) {
            if let Some(Object::Pipe { readers, writers, .. }) = self.objects.get_mut(&pipe) {
                *if reads { readers } else { writers } += 1;
            }
            return Ok(());
        }
        if oid == pid {
            self.own_handles += 1;
            return Ok(());
        }
        match self.objects.get_mut(&oid) {
            Some(Object::Event { handles, .. }) => {
                *handles += 1;
                Ok(())
            }
            _ => Err(format!(
                "a reference to host object {oid}, which this process does not hold"
            )),
        }
    }

    /// `_close` (298-317): true when a pipe lost its last writer, which wakes a reader to see it broken.
    fn close(&mut self, pid: u64, oid: u64) -> bool {
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
        if oid == pid {
            self.own_handles -= 1;
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
}
