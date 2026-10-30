// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The host side of the guest runtime's file system (Sprint 204): `src/w2m/file_bridge.py` (Sprint 042) in Rust. The
//! guest runtime (`tools/guest_runtime/file.c`) does the Windows rules; it reaches the Mac through these primitives on
//! the transport page (hvc #8), under a drive C: per run: a host directory holding the user's profile folders, the
//! folders `specs/recorded_folders.json` records from the runners, and C:\w2m\app, where `root` (the program's
//! directory) is mounted, read-only for the game's depot. Names compare ASCII case-folded, as NTFS compares ASCII names.
//! A request no Windows path can mean, another drive, a UNC name, a ".." the guest did not resolve, or a change the
//! run must not make stops the run by name (the Python's Unimplemented and BridgeRequestError), each by its kind,
//! `FileError` (Sprint 239).

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

use crate::kernel::Memory;

pub const FILE_LOOKUP: u64 = 6;
pub const FILE_OPEN: u64 = 7;
pub const FILE_CLOSE: u64 = 8;
pub const FILE_READ: u64 = 9;
pub const FILE_WRITE: u64 = 10;
pub const FILE_SIZE: u64 = 11;
pub const FILE_TRUNCATE: u64 = 12;
pub const FILE_UNLINK: u64 = 13;
pub const FILE_STAT: u64 = 24;
pub const FILE_MKDIR: u64 = 25;
pub const FILE_RMDIR: u64 = 26;
pub const FILE_FSTAT: u64 = 27;
pub const FILE_DUP: u64 = 28;
pub const FILE_LIST: u64 = 32;
/// 1970-01-01 in 100 ns units since 1601-01-01
pub const FILETIME_UNIX_EPOCH: u64 = 116_444_736_000_000_000;
pub const ABSENT: u64 = 0;
pub const IS_FILE: u64 = 1;
pub const IS_DIRECTORY: u64 = 2;
pub const PARENT_MISSING: u64 = 3;
/// file_bridge.py 64-65: the runners' user's AppData folders and C:\Users\w2m's Temp, fresh each run
const PROFILE_FOLDERS: [&str; 4] = [
    "Users/runneradmin/AppData/LocalLow",
    "Users/runneradmin/AppData/Local/Temp",
    "Users/runneradmin/AppData/Roaming",
    "Users/w2m/AppData/Local/Temp",
];

/// Why a request was not answered, by the kind the frozen bridge raises (Sprint 239; Deferred 2026-10-29, the helpers'
/// errors): `host_request` stops a refusal as a request not served and a failure as a fault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileError {
    /// a path or change w2m does not serve, the frozen bridge's `Unimplemented`: another drive, a rooted or UNC name,
    /// "..", a change under the read-only C:\w2m\app or in a recorded folder, a recorded file it holds by name only
    NotServed(String),
    /// a request that failed: one no Windows path or descriptor can mean (`BridgeRequestError`: a NUL in a path, a
    /// descriptor not open), an unknown primitive, a host call's or the guest memory's error
    Failed(String),
}

impl FileError {
    fn failed(e: impl std::fmt::Display) -> FileError {
        FileError::Failed(e.to_string())
    }
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileError::NotServed(why) | FileError::Failed(why) => f.write_str(why),
        }
    }
}

/// `-errno` as the request answers it.
fn neg(errno: i32) -> u64 {
    (-(i64::from(errno))) as u64
}

fn errno_of(e: &std::io::Error) -> u64 {
    neg(e.raw_os_error().unwrap_or(5)) // EIO
}

/// `fold` (191-193): ASCII upper case folded to lower.
pub fn fold(name: &str) -> String {
    name.chars().map(|c| c.to_ascii_lowercase()).collect()
}

/// A time in FILETIME units from a Unix time in seconds and nanoseconds.
fn filetime(secs: i64, nsec: i64) -> u64 {
    ((secs as i128 * 1_000_000_000 + nsec as i128) / 100) as u64 + FILETIME_UNIX_EPOCH
}

/// The size, then birth, access and modification times as FILETIME (FILE_STAT's 32 bytes); with `change`, the change
/// time after them (FILE_FSTAT's 40). The birth time is `st_birthtimespec` exactly, as `birth_ns` (82-97) reads it.
fn stat_bytes(m: &std::fs::Metadata, change: bool) -> Vec<u8> {
    let birth = m
        .created()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(FILETIME_UNIX_EPOCH, |d| {
            filetime(d.as_secs() as i64, i64::from(d.subsec_nanos()))
        });
    let mut v = vec![
        m.size(),
        birth,
        filetime(m.atime(), m.atime_nsec()),
        filetime(m.mtime(), m.mtime_nsec()),
    ];
    if change {
        v.push(filetime(m.ctime(), m.ctime_nsec()));
    }
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// specs/recorded_folders.json's folders by their folded components from C:\.
fn recorded_folders() -> Result<BTreeMap<Vec<String>, Value>, String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/recorded_folders.json");
    let text = std::fs::read_to_string(path).map_err(|e| format!("specs/recorded_folders.json: {e}"))?;
    let spec: Value = serde_json::from_str(&text).map_err(|e| format!("specs/recorded_folders.json: {e}"))?;
    Ok(spec["folders"]
        .as_object()
        .ok_or("specs/recorded_folders.json holds no folders")?
        .iter()
        .map(|(name, row)| (name.split('\\').map(|p| p.to_lowercase()).collect(), row.clone()))
        .collect())
}

/// POSIX primitives under `root` for one guest process (`FileBridge`).
pub struct FileBridge {
    recorded: BTreeMap<Vec<String>, Value>,
    /// C:\w2m\app, the working directory
    pub root: PathBuf,
    /// the game's depot: nothing under it may change
    pub read_only: bool,
    /// C:\, holding w2m (app is the mount)
    pub drive: PathBuf,
    /// this bridge made the drive and removes it; a child's shares its parent's
    owns_drive: bool,
    fds: BTreeMap<u64, File>,
    read_only_fds: BTreeSet<u64>,
    recorded_fds: BTreeMap<u64, Value>,
    checked: BTreeSet<PathBuf>,
    next_fd: u64,
}

impl FileBridge {
    /// `__init__` (108-128): with no `drive`, a fresh one in the temporary directory, its profile and recorded folders
    /// made.
    pub fn new(root: &Path, read_only: bool, drive: Option<&Path>) -> Result<FileBridge, String> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let recorded = recorded_folders()?;
        let root = root.canonicalize().map_err(|e| format!("{}: {e}", root.display()))?;
        let (drive, owns_drive) = match drive {
            Some(d) => (d.to_path_buf(), false),
            None => {
                // the next name when one is taken: a process killed before its bridge removed its drive leaves the
                // name, and a later process given the same id meets it (Sprint 247's gates met w2m-drive-c-8915-0,
                // left at 05:33, in a family run)
                let d = loop {
                    let n = NEXT.fetch_add(1, Ordering::Relaxed);
                    let d = std::env::temp_dir().join(format!("w2m-drive-c-{}-{n}", std::process::id()));
                    match std::fs::create_dir(&d) {
                        Ok(()) => break d,
                        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                        Err(e) => return Err(format!("{}: {e}", d.display())),
                    }
                };
                let d = d.canonicalize().map_err(|e| e.to_string())?;
                std::fs::create_dir(d.join("w2m")).map_err(|e| e.to_string())?;
                for folder in PROFILE_FOLDERS {
                    std::fs::create_dir_all(d.join(folder)).map_err(|e| e.to_string())?;
                }
                for parts in recorded.keys() {
                    let mut p = d.clone();
                    for part in parts {
                        let mut c = part.chars();
                        p.push(
                            c.next()
                                .map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str()),
                        );
                    }
                    std::fs::create_dir_all(&p).map_err(|e| e.to_string())?;
                }
                (d, true)
            }
        };
        Ok(FileBridge {
            recorded,
            root,
            read_only,
            drive,
            owns_drive,
            fds: BTreeMap::new(),
            read_only_fds: BTreeSet::new(),
            recorded_fds: BTreeMap::new(),
            checked: BTreeSet::new(),
            next_fd: 3,
        })
    }

    fn under_mount(&self, path: &Path) -> bool {
        path == self.root || path.starts_with(&self.root)
    }

    fn writable(&self, path: &Path) -> Result<(), FileError> {
        if self.read_only && self.under_mount(path) {
            return Err(FileError::NotServed(format!(
                "a change under the read-only C:\\w2m\\app: {}",
                path.display()
            )));
        }
        if self.recorded_parent(path).is_some() {
            return Err(FileError::NotServed(format!(
                "a change in a folder recorded from the runners: {}",
                path.display()
            )));
        }
        Ok(())
    }

    /// `_components` (139-143): a host path's folded components from C:\, None outside the drive or under the mount.
    fn components(&self, path: &Path) -> Option<Vec<String>> {
        if self.under_mount(path) || !path.starts_with(&self.drive) {
            return None;
        }
        Some(
            path.strip_prefix(&self.drive)
                .ok()?
                .components()
                .map(|c| fold(&c.as_os_str().to_string_lossy()))
                .collect(),
        )
    }

    fn recorded_parent(&self, path: &Path) -> Option<&Value> {
        let parts = self.components(path)?;
        if parts.is_empty() {
            return None;
        }
        self.recorded.get(&parts[..parts.len() - 1])
    }

    /// `_open_recorded` (150-170): a recorded file's bytes, the folder's copy, read-only, its sha256 the row's.
    fn open_recorded(&mut self, path: &Path, row: &Value, name: &str, create: bool) -> Result<(u64, u64), FileError> {
        let parts = self
            .components(path)
            .ok_or_else(|| FileError::failed("a recorded file outside the drive"))?;
        let folder = &self.recorded[&parts[..parts.len() - 1]];
        let by_name_only = || {
            FileError::NotServed(format!(
                "the contents of a file recorded from the runners by name only: {}",
                path.display()
            ))
        };
        let (Some(contents), Some(sha)) = (folder.get("contents").filter(|c| !c.is_null()), row["sha256"].as_str())
        else {
            return Err(by_name_only());
        };
        if create {
            return Err(by_name_only());
        }
        let root = contents["root"].as_str().unwrap_or("");
        let root = match root.strip_prefix("~/") {
            Some(rest) => PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(rest),
            None => PathBuf::from(root),
        };
        let copy = root.join(name);
        if !self.checked.contains(&copy) {
            let not_the_runners = || {
                FileError::NotServed(format!(
                    "the copy of {name} at {} is missing or not the runner's (sha256 {sha})",
                    copy.display()
                ))
            };
            let bytes = std::fs::read(&copy).map_err(|_| not_the_runners())?;
            if guest_runtime::sha256::hex(&bytes) != sha {
                return Err(not_the_runners());
            }
            self.checked.insert(copy.clone());
        }
        let file = File::open(&copy).map_err(|e| FileError::Failed(format!("{}: {e}", copy.display())))?;
        let fd = self.next_fd;
        self.next_fd += 1;
        self.fds.insert(fd, file);
        self.recorded_fds.insert(fd, row.clone());
        Ok((fd, name_id(name)))
    }

    /// `_recorded_row` (172-189): (kind, the recording's row and the file's recorded name) for a path in or of a
    /// recorded folder; (kind, None) elsewhere.
    fn recorded_row(&self, kind: u64, path: &Path) -> Result<(u64, Option<(Value, String)>), FileError> {
        let Some(parts) = self.components(path) else {
            return Ok((kind, None));
        };
        for i in 0..parts.len() {
            if let Some(above) = self.recorded.get(&parts[..i])
                && above["files"].is_null()
                && !self.recorded.contains_key(&parts[..=i])
            {
                return Err(FileError::NotServed(format!(
                    "a name in a folder whose listing is not recorded: {}",
                    path.display()
                )));
            }
        }
        if let Some(folder) = self.recorded.get(&parts) {
            return Ok((IS_DIRECTORY, Some((folder["entry"].clone(), String::new()))));
        }
        if parts.is_empty() {
            return Ok((kind, None));
        }
        let Some(parent) = self.recorded.get(&parts[..parts.len() - 1]) else {
            return Ok((kind, None));
        };
        if kind == PARENT_MISSING {
            return Ok((kind, None));
        }
        let last = &parts[parts.len() - 1];
        let row = parent["files"]
            .as_object()
            .and_then(|files| files.iter().find(|(n, _)| fold(n) == *last))
            .map(|(n, r)| (r.clone(), n.clone()));
        Ok(match row {
            Some(r) => (IS_FILE, Some(r)),
            None => (ABSENT, None),
        })
    }

    /// `_path` (195-200): the components a request's path names.
    fn path(&self, mem: &dyn Memory, va: u64, chars: u64) -> Result<Vec<String>, FileError> {
        let raw = mem.read(va, 2 * chars as usize).map_err(FileError::Failed)?;
        let units: Vec<u16> = raw.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
        let name = String::from_utf16_lossy(&units);
        if let Some(at) = units.iter().position(|&u| u == 0) {
            return Err(FileError::Failed(format!(
                "path of {chars} characters has a NUL at character {at}: {name:?}"
            )));
        }
        parts(&name)
    }

    /// `host_file` (202-206): the host file a guest name means, None when no file is there.
    pub fn host_file(&self, name: &str) -> Result<Option<PathBuf>, FileError> {
        let (kind, path) = self.resolve(&parts(name)?);
        Ok((kind == IS_FILE).then_some(path))
    }

    /// A file view's bytes (vm_backend 3859-3862, Sprint 211): `os.pread(fd, size, 0)`, the file's first `size` bytes
    /// or fewer at its end; None when `fd` is not open.
    pub fn view(&self, fd: u64, size: u64) -> Option<Result<Vec<u8>, FileError>> {
        let file = self.fds.get(&fd)?;
        let mut data = vec![0u8; size as usize];
        Some(match file.read_at(&mut data, 0) {
            Ok(n) => {
                data.truncate(n);
                Ok(data)
            }
            Err(e) => Err(FileError::failed(e)),
        })
    }

    /// `_resolve` (225-239): (kind, host path), each existing component matched ASCII case-folded; C:\w2m\app is root.
    pub fn resolve(&self, parts: &[String]) -> (u64, PathBuf) {
        let mut here = self.drive.clone();
        for (i, part) in parts.iter().enumerate() {
            if here == self.drive.join("w2m") && fold(part) == "app" {
                here = self.root.clone();
                continue;
            }
            if !here.is_dir() {
                return (PARENT_MISSING, here.join(part));
            }
            let folded = fold(part);
            let found = std::fs::read_dir(&here).ok().and_then(|entries| {
                entries
                    .flatten()
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .find(|e| fold(e) == folded)
            });
            let Some(found) = found else {
                let kind = if i == parts.len() - 1 { ABSENT } else { PARENT_MISSING };
                return (kind, here.join(part));
            };
            here.push(found);
        }
        (if here.is_dir() { IS_DIRECTORY } else { IS_FILE }, here)
    }

    fn new_fd(&mut self, file: File) -> u64 {
        let fd = self.next_fd;
        self.next_fd += 1;
        self.fds.insert(fd, file);
        fd
    }

    /// `request` (241-360): one primitive; (result, arg[5]). A refusal stops the run as unserved, a failure as a fault.
    pub fn request(&mut self, mem: &mut dyn Memory, op: u64, arg: [u64; 4]) -> Result<(u64, u64), FileError> {
        match op {
            FILE_LOOKUP => {
                let (kind, path) = self.resolve(&self.path(mem, arg[0], arg[1])?);
                let (kind, row) = self.recorded_row(kind, &path)?;
                if row.is_some() && kind == IS_FILE {
                    let name = path
                        .file_name()
                        .map_or(String::new(), |n| n.to_string_lossy().to_string());
                    return Ok((kind, name_id(&name)));
                }
                let ino = if kind == IS_FILE || kind == IS_DIRECTORY {
                    std::fs::metadata(&path).map_err(FileError::failed)?.ino()
                } else {
                    0
                };
                Ok((kind, ino))
            }
            FILE_OPEN => {
                let (kind, path) = self.resolve(&self.path(mem, arg[0], arg[1])?);
                let (_, row) = self.recorded_row(kind, &path)?;
                if let Some((row, name)) = row {
                    return self.open_recorded(&path, &row, &name, arg[2] != 0);
                }
                if arg[2] != 0 {
                    self.writable(&path)?;
                }
                let mut options = OpenOptions::new();
                options.read(true).mode(0o644);
                if !(self.under_mount(&path) && self.read_only) {
                    options.write(true);
                }
                if arg[2] != 0 {
                    options.create_new(true);
                }
                let file = match options.open(&path) {
                    Ok(f) => f,
                    Err(e) => return Ok((errno_of(&e), 0)),
                };
                let ino = file.metadata().map_err(FileError::failed)?.ino();
                let under = self.under_mount(&path);
                let fd = self.new_fd(file);
                if under {
                    self.read_only_fds.insert(fd);
                }
                Ok((fd, ino))
            }
            FILE_STAT => {
                let (kind, path) = self.resolve(&self.path(mem, arg[0], arg[1])?);
                let (kind, row) = self.recorded_row(kind, &path)?;
                if let Some((row, _)) = row {
                    let bytes: Vec<u8> = ["size", "creation", "access", "write"]
                        .iter()
                        .flat_map(|k| row[*k].as_u64().unwrap_or(0).to_le_bytes())
                        .collect();
                    mem.write(arg[2], &bytes).map_err(FileError::Failed)?;
                    return Ok((kind, row["attributes"].as_u64().unwrap_or(0)));
                }
                if kind == IS_FILE || kind == IS_DIRECTORY {
                    let m = std::fs::metadata(&path).map_err(FileError::failed)?;
                    mem.write(arg[2], &stat_bytes(&m, false)).map_err(FileError::Failed)?;
                }
                Ok((kind, 0))
            }
            FILE_LIST => {
                let (kind, path) = self.resolve(&self.path(mem, arg[0], arg[1])?);
                if kind != IS_DIRECTORY {
                    return Ok((0, kind));
                }
                let folder = self.components(&path).and_then(|p| self.recorded.get(&p));
                let mut entries: BTreeSet<String> = match folder {
                    Some(f) if f["files"].is_null() => {
                        return Err(FileError::NotServed(format!(
                            "the listing of a folder recorded without one: {}",
                            path.display()
                        )));
                    }
                    Some(f) => f["files"]
                        .as_object()
                        .map(|m| m.keys().cloned().collect())
                        .unwrap_or_default(),
                    None => std::fs::read_dir(&path)
                        .map_err(FileError::failed)?
                        .flatten()
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .collect(),
                };
                if path == self.drive.join("w2m") {
                    entries.retain(|e| fold(e) != "app");
                    entries.insert("app".into());
                }
                let mut names: Vec<u8> = Vec::new();
                for e in &entries {
                    names.extend(e.encode_utf16().flat_map(u16::to_le_bytes));
                    names.extend([0, 0]);
                }
                names.extend([0, 0]);
                if names.len() as u64 <= arg[3] {
                    mem.write(arg[2], &names).map_err(FileError::Failed)?;
                }
                Ok((names.len() as u64, kind))
            }
            FILE_MKDIR | FILE_RMDIR => {
                let (kind, path) = self.resolve(&self.path(mem, arg[0], arg[1])?);
                if kind == PARENT_MISSING {
                    return Ok((neg(2), 0)); // ENOENT
                }
                if op == FILE_MKDIR && (kind == IS_FILE || kind == IS_DIRECTORY) {
                    return Ok((neg(17), 0)); // EEXIST
                }
                self.writable(&path)?;
                let done = if op == FILE_MKDIR {
                    std::fs::create_dir(&path)
                } else {
                    std::fs::remove_dir(&path)
                };
                Ok(match done {
                    Ok(()) => (0, 0),
                    Err(e) => (errno_of(&e), 0),
                })
            }
            FILE_UNLINK => {
                let (_, path) = self.resolve(&self.path(mem, arg[0], arg[1])?);
                self.writable(&path)?;
                Ok(match std::fs::remove_file(&path) {
                    Ok(()) => (0, 0),
                    Err(e) => (errno_of(&e), 0),
                })
            }
            _ => self.descriptor_request(mem, op, arg),
        }
    }

    /// The primitives on an open descriptor (313-360).
    fn descriptor_request(&mut self, mem: &mut dyn Memory, op: u64, arg: [u64; 4]) -> Result<(u64, u64), FileError> {
        let fd = arg[0];
        let Some(file) = self.fds.get(&fd) else {
            return Err(FileError::Failed(format!(
                "file primitive {op} on descriptor {fd}, which is not open"
            )));
        };
        match op {
            FILE_CLOSE => {
                self.recorded_fds.remove(&fd);
                self.read_only_fds.remove(&fd);
                self.fds.remove(&fd);
                Ok((0, 0))
            }
            FILE_READ => {
                let mut data = vec![0u8; arg[2] as usize];
                let n = file.read_at(&mut data, arg[3]).map_err(FileError::failed)?;
                mem.write(arg[1], &data[..n]).map_err(FileError::Failed)?;
                Ok((n as u64, 0))
            }
            FILE_WRITE => {
                if self.recorded_fds.contains_key(&fd) {
                    return Err(FileError::NotServed(
                        "a write to a file recorded from the runners".into(),
                    ));
                }
                if self.read_only && self.read_only_fds.contains(&fd) {
                    return Err(FileError::NotServed(
                        "a write to a file under the read-only C:\\w2m\\app".into(),
                    ));
                }
                let data = mem.read(arg[1], arg[2] as usize).map_err(FileError::Failed)?;
                let n = file.write_at(&data, arg[3]).map_err(FileError::failed)?;
                Ok((n as u64, 0))
            }
            FILE_DUP => {
                let copy = file.try_clone().map_err(FileError::failed)?;
                let new = self.new_fd(copy);
                if let Some(row) = self.recorded_fds.get(&fd).cloned() {
                    self.recorded_fds.insert(new, row);
                }
                if self.read_only_fds.contains(&fd) {
                    self.read_only_fds.insert(new);
                }
                Ok((new, 0))
            }
            FILE_FSTAT => {
                if let Some(row) = self.recorded_fds.get(&fd) {
                    let bytes: Vec<u8> = ["size", "creation", "access", "write", "write"]
                        .iter()
                        .flat_map(|k| row[*k].as_u64().unwrap_or(0).to_le_bytes())
                        .collect();
                    mem.write(arg[1], &bytes).map_err(FileError::Failed)?;
                    return Ok((0, 0));
                }
                let m = file.metadata().map_err(FileError::failed)?;
                mem.write(arg[1], &stat_bytes(&m, true)).map_err(FileError::Failed)?;
                Ok((0, 0))
            }
            FILE_SIZE => {
                let m = file.metadata().map_err(FileError::failed)?;
                if !m.is_file() {
                    return Err(FileError::Failed(format!(
                        "FILE_SIZE on descriptor {fd}, which is not a regular file"
                    )));
                }
                Ok((m.size(), 0))
            }
            FILE_TRUNCATE => {
                if self.recorded_fds.contains_key(&fd) {
                    return Err(FileError::NotServed(
                        "a truncation of a file recorded from the runners".into(),
                    ));
                }
                if self.read_only && self.read_only_fds.contains(&fd) {
                    return Err(FileError::NotServed(
                        "a truncation of a file under the read-only C:\\w2m\\app".into(),
                    ));
                }
                file.set_len(arg[1]).map_err(FileError::failed)?;
                Ok((0, 0))
            }
            _ => Err(FileError::Failed(format!("unknown file primitive {op}"))),
        }
    }
}

impl Drop for FileBridge {
    /// `close_all` (362-368): the descriptors closed; the drive's owner removes it.
    fn drop(&mut self) {
        self.fds.clear();
        if self.owns_drive {
            let _ = std::fs::remove_dir_all(&self.drive);
        }
    }
}

/// A recorded file's id, from its name, stable within the run (file_bridge.py 170, 247).
fn name_id(name: &str) -> u64 {
    let lower = name.to_lowercase();
    let bytes = &lower.as_bytes()[..lower.len().min(7)];
    let mut w = [0u8; 8];
    w[..bytes.len()].copy_from_slice(bytes);
    u64::from_le_bytes(w) | 1 << 60
}

/// `_parts` (208-223): the components from C:\ (a full path) or from C:\w2m\app (a relative name); another drive, a
/// rooted or UNC name and ".." refused, as the frozen bridge refuses them (`Unimplemented`, 213-222).
pub fn parts(name: &str) -> Result<Vec<String>, FileError> {
    let name = name.strip_prefix("\\\\?\\").unwrap_or(name);
    let b = name.as_bytes();
    let (mut out, rest): (Vec<String>, &str) = if b.len() > 1 && b[1] == b':' {
        if !matches!(b[0], b'c' | b'C') || (b.len() > 2 && !matches!(b[2], b'\\' | b'/')) {
            return Err(FileError::NotServed(format!(
                "file path on a drive other than C: or drive-relative: {name:?}"
            )));
        }
        (Vec::new(), name.get(3..).unwrap_or(""))
    } else if name.starts_with(['\\', '/']) {
        return Err(FileError::NotServed(format!(
            "file path rooted without a drive, or UNC: {name:?}"
        )));
    } else {
        (vec!["w2m".into(), "app".into()], name)
    };
    for p in rest.split(['\\', '/']) {
        if p.is_empty() || p == "." {
            continue;
        }
        if p == ".." {
            return Err(FileError::NotServed(format!("file path with '..': {name:?}")));
        }
        out.push(p.to_string());
    }
    Ok(out)
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

    fn with_path(mem: &mut Region, path: &str) -> u64 {
        let b: Vec<u8> = path.encode_utf16().flat_map(u16::to_le_bytes).collect();
        mem.0[..b.len()].copy_from_slice(&b);
        path.encode_utf16().count() as u64
    }

    fn root(name: &str) -> PathBuf {
        let r = std::env::temp_dir().join(format!("w2m-bridge-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&r);
        std::fs::create_dir_all(r.join("Data")).unwrap();
        std::fs::write(r.join("Data").join("Level0.bin"), b"0123456789").unwrap();
        r
    }

    /// A drive's name left by a killed process of the same id is passed over, not an error (Sprint 247).
    #[test]
    fn a_drive_left_behind_is_passed_over() {
        let r = root("left");
        let name = |k: u64| std::env::temp_dir().join(format!("w2m-drive-c-{}-{k}", std::process::id()));
        // the names it leaves are removed when the test ends, a panic included (Sprint 252: a plant's panic at 247
        // left 1,024 of them in the temporary directory)
        struct Left(Vec<PathBuf>, PathBuf);
        impl Drop for Left {
            fn drop(&mut self) {
                for d in &self.0 {
                    let _ = std::fs::remove_dir(d);
                }
                let _ = std::fs::remove_dir_all(&self.1);
            }
        }
        let left = Left(
            (0..1024)
                .map(name)
                .filter_map(|d| std::fs::create_dir(&d).ok().map(|()| d.canonicalize().unwrap()))
                .collect(),
            r.clone(),
        );
        let b = FileBridge::new(&r, true, None).unwrap();
        assert!(!left.0.contains(&b.drive), "{}", b.drive.display());
    }

    /// file_bridge.py 225-239, 268-278, 320-323: a name found case-folded under C:\w2m\app, its size and FILETIMEs
    /// by FILE_STAT, its bytes by FILE_OPEN and FILE_READ at an offset; a missing name absent, a missing parent so.
    #[test]
    fn a_file_is_found_case_folded_stated_opened_and_read() {
        let r = root("read");
        let mut b = FileBridge::new(&r, true, None).unwrap();
        let mut mem = Region(vec![0; 0x2000]);
        let n = with_path(&mut mem, "C:\\W2M\\APP\\data\\LEVEL0.BIN");
        let (kind, _) = b.request(&mut mem, FILE_STAT, [0x1000, n, 0x1800, 0]).unwrap();
        assert_eq!(kind, IS_FILE);
        assert_eq!(u64::from_le_bytes(mem.0[0x800..0x808].try_into().unwrap()), 10);
        let (fd, _) = b.request(&mut mem, FILE_OPEN, [0x1000, n, 0, 0]).unwrap();
        assert_eq!(fd, 3);
        let (read, _) = b.request(&mut mem, FILE_READ, [fd, 0x1900, 4, 3]).unwrap();
        assert_eq!((read, &mem.0[0x900..0x904]), (4, &b"3456"[..]));
        let n = with_path(&mut mem, "Data\\none.bin");
        assert_eq!(b.request(&mut mem, FILE_LOOKUP, [0x1000, n, 0, 0]).unwrap().0, ABSENT);
        let n = with_path(&mut mem, "none\\x.bin");
        assert_eq!(
            b.request(&mut mem, FILE_LOOKUP, [0x1000, n, 0, 0]).unwrap().0,
            PARENT_MISSING
        );
        drop(b);
        let _ = std::fs::remove_dir_all(&r);
    }

    /// file_bridge.py 133-137, 293-304: the read-only mount refuses a change by name; the profile's Temp takes a new
    /// directory, and EEXIST answers it again.
    #[test]
    fn the_read_only_mount_refuses_a_change_and_the_profile_takes_one() {
        let r = root("write");
        let mut b = FileBridge::new(&r, true, None).unwrap();
        let mut mem = Region(vec![0; 0x2000]);
        let n = with_path(&mut mem, "C:\\w2m\\app\\Data\\new");
        let refused = b.request(&mut mem, FILE_MKDIR, [0x1000, n, 0, 0]).unwrap_err();
        assert!(refused.to_string().contains("read-only"), "{refused}");
        let n = with_path(&mut mem, "C:\\Users\\w2m\\AppData\\Local\\Temp\\x");
        assert_eq!(b.request(&mut mem, FILE_MKDIR, [0x1000, n, 0, 0]).unwrap().0, 0);
        assert_eq!(b.request(&mut mem, FILE_MKDIR, [0x1000, n, 0, 0]).unwrap().0, neg(17));
        drop(b);
        let _ = std::fs::remove_dir_all(&r);
    }

    /// Each error the kind the frozen bridge raises (Sprint 239): `Unimplemented` (213-222, 135, 326-328, 355-357) a
    /// refusal; `BridgeRequestError` (199, 314), the unknown primitive's `ValueError` (360) and FILE_SIZE's `assert`
    /// (351) failures.
    #[test]
    fn each_error_is_the_kind_the_frozen_bridge_raises() {
        let r = root("kinds");
        let mut b = FileBridge::new(&r, true, None).unwrap();
        let mut mem = Region(vec![0; 0x2000]);
        let mut refusals = Vec::new();
        for (path, op) in [
            ("D:\\x", FILE_LOOKUP),
            ("\\\\server\\x", FILE_LOOKUP),
            ("a\\..\\b", FILE_LOOKUP),
            ("C:\\w2m\\app\\Data\\new", FILE_MKDIR),
            ("C:\\w2m\\app\\Data\\Level0.bin", FILE_UNLINK),
        ] {
            let n = with_path(&mut mem, path);
            refusals.push((path, b.request(&mut mem, op, [0x1000, n, 0, 0]).unwrap_err()));
        }
        let n = with_path(&mut mem, "Data\\Level0.bin");
        let (fd, _) = b.request(&mut mem, FILE_OPEN, [0x1000, n, 0, 0]).unwrap();
        refusals.push((
            "write",
            b.request(&mut mem, FILE_WRITE, [fd, 0x1000, 1, 0]).unwrap_err(),
        ));
        refusals.push((
            "truncate",
            b.request(&mut mem, FILE_TRUNCATE, [fd, 0, 0, 0]).unwrap_err(),
        ));
        for (what, e) in &refusals {
            assert!(
                matches!(e, FileError::NotServed(_)),
                "{what}: the frozen bridge refuses it, and here {e:?}"
            );
        }

        let mut failures = Vec::new();
        let n = with_path(&mut mem, "a\0b");
        failures.push(("NUL", b.request(&mut mem, FILE_LOOKUP, [0x1000, n, 0, 0]).unwrap_err()));
        failures.push((
            "not open",
            b.request(&mut mem, FILE_READ, [fd + 9, 0x1900, 1, 0]).unwrap_err(),
        ));
        failures.push(("unknown", b.request(&mut mem, 99, [fd, 0, 0, 0]).unwrap_err()));
        let n = with_path(&mut mem, "Data");
        let (dir, _) = b.request(&mut mem, FILE_OPEN, [0x1000, n, 0, 0]).unwrap();
        failures.push(("size", b.request(&mut mem, FILE_SIZE, [dir, 0, 0, 0]).unwrap_err()));
        for (what, e) in &failures {
            assert!(
                matches!(e, FileError::Failed(_)),
                "{what}: the frozen bridge fails it, and here {e:?}"
            );
        }
        drop(b);
        let _ = std::fs::remove_dir_all(&r);
    }

    /// `_parts`: \\?\ dropped, C:\ full, relative under C:\w2m\app; another drive, a UNC name and ".." refused.
    #[test]
    fn paths_split_as_the_python_splits_them() {
        assert_eq!(parts("\\\\?\\C:\\a\\.\\b").unwrap(), ["a", "b"]);
        assert_eq!(parts("x/y").unwrap(), ["w2m", "app", "x", "y"]);
        assert!(parts("D:\\x").is_err());
        assert!(parts("\\\\server\\x").is_err());
        assert!(parts("a\\..\\b").is_err());
    }
}
