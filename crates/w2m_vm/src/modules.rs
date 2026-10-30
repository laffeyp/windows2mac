// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Which module a name means, for the guest runtime's module calls (Sprint 189): `src/w2m/modules.py` (Sprint 044) in
//! Rust, over what the guest already holds: the program and its DLLs as the loader mapped them (`Guest::images`), the
//! synthetic kernel32, kernelbase, ntdll and system images (`ProcessArea`), and the thunks (`Guest::thunks`,
//! `Guest::import_thunk`). The guest runtime (`tools/guest_runtime/module.c`) asks the host three things through
//! HOST_MODULE 15, HOST_PROC 16 and HOST_MODULE_PATH 17 (vm_backend 3889-3904); HOST_MODULE_AT 18 is
//! `host_request::containing`.
//!
//! A DLL file loaded after the start is mapped (`load_dll`, Sprint 207). GetProcAddress keeps its own thunks
//! (`Guest::proc_thunks`, the Python's `Modules._thunks`), apart from the import thunks (`Guest::imports`), so a
//! thunk's address is the frozen host's (Sprint 209; before, a GetProcAddress thunk was `import_thunk`'s, which reused
//! any thunk of the name).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::guest::Guest;

/// `APP_DIRECTORY` (modules.py 46)
pub const APP_DIRECTORY: &str = "C:\\w2m\\app\\";

/// `API_SET_HOSTS` (modules.py 49-53): API sets whose host is not kernelbase, as LoadLibraryExW answered on Server 2025
/// and 2022 (Sprint 055, rows load.api_sets) and the C runtime's thread start (Sprint 063).
const API_SET_HOSTS: [(&str, &str); 4] = [
    ("api-ms-win-core-winrt-l1-1-0.dll", "combase.dll"),
    ("api-ms-win-core-com-l1-1-1.dll", "combase.dll"),
    ("api-ms-win-core-winrt-string-l1-1-0.dll", "combase.dll"),
    ("api-ms-win-appmodel-runtime-l1-1-2.dll", "kernel.appcore.dll"),
];

/// `specs/windows_exports.json`: each module's export names, and System32's file names by their lowercase form.
struct Spec {
    exports: BTreeMap<String, BTreeSet<String>>,
    system32: BTreeMap<String, String>,
}

fn spec() -> &'static Spec {
    static SPEC: OnceLock<Spec> = OnceLock::new();
    SPEC.get_or_init(|| {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/windows_exports.json");
        let text = std::fs::read_to_string(path).expect("specs/windows_exports.json");
        let v: serde_json::Value = serde_json::from_str(&text).expect("specs/windows_exports.json parses");
        let exports = v["modules"]
            .as_object()
            .expect("specs/windows_exports.json's modules")
            .iter()
            .map(|(m, names)| {
                let names = names
                    .as_array()
                    .map(|a| a.iter().filter_map(|n| n.as_str()).map(str::to_string));
                (m.clone(), names.into_iter().flatten().collect())
            })
            .collect();
        let system32 = v["system32"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|n| n.as_str())
            .map(|n| (n.to_lowercase(), n.to_string()))
            .collect();
        Spec { exports, system32 }
    })
}

/// `_canonical` (56-59): lowercase, the last path component, ".dll" appended when there is no extension.
pub fn canonical(name: &str) -> String {
    let base = name.replace('/', "\\");
    let base = base.rsplit('\\').next().unwrap_or("").to_lowercase();
    if base.contains('.') { base } else { base + ".dll" }
}

/// The program's DLLs (`Modules.dlls`, vm_backend 2957-2958): canonical name -> (base, SizeOfImage, the image index).
fn dlls(guest: &Guest) -> impl Iterator<Item = (String, u64, u64, usize)> + '_ {
    guest
        .images
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, d)| (d.name.to_lowercase(), d.base, u64::from(d.pe.size_of_image), i))
}

/// Why a module request was not answered (Sprint 218; the Deferred entry on the helpers' errors, reviewer-3's High 1 of
/// its review of Sprint 211): a case Windows serves that w2m does not serve yet, the Python's Unimplemented, which
/// ends a run as unserved; or a failure of the host's own (a file unreadable, a PE that does not parse, a mapping
/// error), which ends it as a fault. Each site names its kind; there is no conversion from a string, so a new error is
/// classed where it arises (reviewer-3's Medium 3 of its review of Sprint 218).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleError {
    NotServed(String),
    Failed(String),
}

/// The file bridge's kind kept (Sprint 239): LoadLibrary resolves its name through the bridge.
impl From<crate::file_bridge::FileError> for ModuleError {
    fn from(e: crate::file_bridge::FileError) -> ModuleError {
        match e {
            crate::file_bridge::FileError::NotServed(why) => ModuleError::NotServed(why),
            crate::file_bridge::FileError::Failed(why) => ModuleError::Failed(why),
        }
    }
}

/// `module` (94-132): the handle `name` means, 0 when it is found nowhere; `loading` is LoadLibrary, which would load
/// a DLL not yet in the process, where GetModuleHandle reports only loaded ones.
pub fn module(guest: &mut Guest, name: &str, loading: bool) -> Result<u64, ModuleError> {
    use guest_runtime::layout::{MONITOR_SIZE, PROCESS_VA, SYSTEM_IMAGES, SYSTEM_IMAGES_OFF};
    let p = &guest.process;
    let used = p.system.len() as u64;
    let next_system = (used < SYSTEM_IMAGES).then_some(PROCESS_VA + SYSTEM_IMAGES_OFF + 0x1000 * used);
    let handles = Handles {
        next_system,
        kernel32: p.kernel32,
        kernelbase: p.kernelbase,
        ntdll: p.ntdll,
        image: guest.images.first().map(|i| (i.name.to_lowercase(), i.base)),
        dlls: dlls(guest).map(|(n, base, ..)| (n, base)).collect(),
        system: p.system.iter().map(|(c, (h, _))| (c.clone(), *h)).collect(),
        app: AppDirectory {
            root: guest.app_root.clone(),
            program: guest.images.first().map_or(String::new(), |i| i.name.clone()),
            files: guest.files.as_ref(),
        },
    };
    let handle = module_in(&handles, name, loading)?;
    if handle == FILE_FOUND {
        return load_dll(guest, name);
    }
    // `load_system` (vm_backend `system_image`): the synthetic page written and kept under System32's spelling
    if Some(handle) == next_system {
        // the DLL loaded: an API set's host (`API_SET_HOSTS`), else the name itself
        let asked = canonical(name);
        let c = API_SET_HOSTS
            .iter()
            .find(|(set, _)| *set == asked)
            .map_or(asked.clone(), |(_, host)| canonical(host));
        let spelling = spec()
            .system32
            .get(&c)
            .ok_or_else(|| {
                ModuleError::Failed(format!("{c}: loaded as a system DLL, and System32 holds no such file"))
            })?
            .clone();
        let off = handle - PROCESS_VA;
        let process = &mut guest.memory.as_mut_slice()[MONITOR_SIZE as usize..];
        crate::process::synthetic_image(process, off);
        guest.process.system.insert(c, (handle, spelling));
    }
    Ok(handle)
}

/// `_load_dll` (vm_backend 3906-3951; Sprint 205): the DLL file `name` finds through the process's file bridge, placed
/// in the VirtualAlloc arena at its preferred base or relocated (`place_image`), mapped, its imports bound (a loaded
/// DLL's export, else the import thunk), snapshotted, and added to the module table under the file's own name and full
/// path, PEB.Ldr and the trace ranges; its DllMain, if it has an entry, left to the init stub that the module request's
/// second answer gives the guest runtime. 0: no file. A DLL with TLS callbacks stops by name as the Python's does, a
/// refusal; one with static TLS gets its index and a block in every thread (`threads::late_static_tls`, Sprint 218).
fn load_dll(guest: &mut Guest, name: &str) -> Result<u64, ModuleError> {
    use crate::guest::PROCESS_SIZE;
    use guest_runtime::layout::MONITOR_SIZE;
    let Some(files) = guest.files.as_ref() else {
        return Ok(0);
    };
    let Some(host) = files.host_file(name)? else {
        return Ok(0);
    };
    let root = files.root.clone();
    use ModuleError::{Failed, NotServed};
    let raw = std::fs::read(&host).map_err(|e| Failed(format!("LoadLibrary {name}: {}: {e}", host.display())))?;
    let pe = pe_image::parse(&raw, false).map_err(|e| Failed(format!("LoadLibrary {name}: {e:?}")))?;
    if !pe.tls_callbacks.is_empty() {
        return Err(ModuleError::NotServed(format!(
            "LoadLibrary {name}: TLS callbacks in a DLL loaded after the start"
        )));
    }
    let file = host
        .file_name()
        .map_or(String::new(), |f| f.to_string_lossy().to_string());
    // the arena with no free range: its refusal (`Unimplemented`)
    let base = guest.arena.place_image(pe.clone()).map_err(|e| NotServed(e.0))?;
    let mut staging = pe_loader::Memory::default();
    pe_loader::map_image(&mut staging, &raw, &pe, base, &file).map_err(|e| Failed(format!("{file}: {e:?}")))?;
    let data = &staging.regions[0].data;
    let at = guest
        .layout
        .offset(base)
        .ok_or_else(|| Failed(format!("{file} placed at 0x{base:x}, which is not mapped")))? as usize;
    guest.memory.as_mut_slice()[at..at + data.len()].copy_from_slice(data);
    for (&slot, target) in &pe.iat {
        let (dll, function) = target.split_once('!').unwrap_or((target, ""));
        let leaf = canonical(dll);
        let found = dlls(guest).find(|(n, ..)| *n == leaf).map(|(_, b, _, i)| (b, i));
        let va = match found {
            Some((dll_base, i)) => guest.images[i]
                .pe
                .exports
                .iter()
                .find(|(n, _)| n == function)
                .map(|(_, rva)| dll_base + u64::from(*rva))
                .ok_or_else(|| {
                    // Windows fails the load (ERROR_PROC_NOT_FOUND), an answer w2m does not give yet
                    ModuleError::NotServed(format!("{file} imports {target}, which {leaf} does not export"))
                })?,
            None => guest.import_thunk(target).map_err(|e| Failed(format!("{e:?}")))?,
        };
        let s = at + slot as usize;
        guest.memory.as_mut_slice()[s..s + 8].copy_from_slice(&va.to_le_bytes());
    }
    // its static TLS in every thread before the snapshot and before it joins the images (vm_backend 3938-3940)
    crate::threads::late_static_tls(guest, &pe, base)?;
    let view = crate::guest::GuestView {
        mem: guest.memory.as_mut_slice(),
        layout: &guest.layout,
    };
    guest.arena.snapshot_image(&view, base, &pe);
    // `Modules.dll_paths` (vm_backend 3944-3945): below C:\w2m\app by its components, else the host path
    let path = match host.strip_prefix(&root) {
        Ok(rel) => {
            let parts: Vec<String> = rel.iter().map(|p| p.to_string_lossy().to_string()).collect();
            format!("{APP_DIRECTORY}{}", parts.join("\\"))
        }
        Err(_) => host.display().to_string(),
    };
    guest.dll_paths.insert(file.to_lowercase(), path);
    guest.dll_ranges.push((base, u64::from(pe.size_of_image)));
    let entry_rva = pe.entry_rva;
    guest.images.push(pe_loader::Image { name: file, pe, base });
    // PEB.Ldr lists it, as EnumProcessModules reads (`_write_loader_data`, Sprint 063)
    let list = crate::process::LoaderList {
        images: &guest.images,
        image_entry: guest.image_entry,
        image_name: &guest.start.image_name,
        dependency_order: &guest.dependency_order,
        paths: &guest.dll_paths,
    };
    let process = &mut guest.memory.as_mut_slice()[MONITOR_SIZE as usize..(MONITOR_SIZE + PROCESS_SIZE) as usize];
    // its only errors are the loader area full (process.rs 548, 569), a capacity, as the system slots are
    crate::process::write_loader_data(process, &list, &guest.process).map_err(|e| NotServed(e.0))?;
    guest.trace_ranges().map_err(|e| Failed(format!("{e:?}")))?;
    if entry_rva != 0 {
        guest.pending_init = guest
            .init_stub(base, base + u64::from(entry_rva))
            .map_err(|e| Failed(format!("{e:?}")))?;
    }
    Ok(base)
}

/// C:\w2m\app on the host (Sprint 196; reviewer-2's High 2 of its review of 193-196): the directory w2m-vm was given,
/// or for a probe run a directory holding only the program, as family_run.py's `run_in_vm` makes it.
struct AppDirectory<'a> {
    root: Option<PathBuf>,
    program: String,
    /// the process's file bridge, whose resolution LoadLibrary shares with the file requests (Sprint 204's departure,
    /// Sprint 205); None in a run without one
    files: Option<&'a crate::file_bridge::FileBridge>,
}

impl AppDirectory<'_> {
    /// `host_file` (file_bridge.py 202-240): whether a file is where the guest name points, a full C:\ path or a path
    /// relative to C:\w2m\app, each component matched case-folded; a path the bridge refuses is Err, as it raises.
    /// Without a bridge, a full path outside C:\w2m\app finds no file.
    fn host_file(&self, name: &str) -> Result<bool, ModuleError> {
        if let Some(files) = self.files {
            return Ok(files.host_file(name)?.is_some());
        }
        // without a bridge, `app_parts` refuses the names the bridge refuses, each a refusal
        match &self.root {
            Some(root) => Ok(host_file(root, name).map_err(ModuleError::NotServed)?.is_some()),
            None => {
                let parts = app_parts(name).map_err(ModuleError::NotServed)?;
                Ok(matches!(parts.as_deref(), Some([p]) if p.eq_ignore_ascii_case(&self.program)))
            }
        }
    }

    /// `in_app_directory` (vm_backend 2926-2927): a top-level entry of C:\w2m\app named `c` (canonical, lowercase).
    fn contains(&self, c: &str) -> bool {
        match &self.root {
            Some(root) => std::fs::read_dir(root).is_ok_and(|entries| {
                entries
                    .flatten()
                    .any(|e| e.file_name().to_string_lossy().to_lowercase() == c)
            }),
            None => self.program.to_lowercase() == c,
        }
    }
}

/// `_parts` (file_bridge.py 210-223) for a name under C:\w2m\app: its components below C:\w2m\app, a full C:\ path or
/// a relative one; None for a full path elsewhere on C:, where the bridge's drive holds only empty system folders; Err
/// for a path the bridge refuses.
fn app_parts(name: &str) -> Result<Option<Vec<&str>>, String> {
    let name = name.strip_prefix("\\\\?\\").unwrap_or(name);
    let b = name.as_bytes();
    let parts: Vec<&str> = if b.len() > 1 && b[1] == b':' {
        if !matches!(b[0], b'c' | b'C') || (b.len() > 2 && !matches!(b[2], b'\\' | b'/')) {
            return Err(format!(
                "file path on a drive other than C: or drive-relative: {name:?}"
            ));
        }
        let parts = split(name.get(3..).unwrap_or(""));
        match parts.as_slice() {
            [w, a, rest @ ..] if w.eq_ignore_ascii_case("w2m") && a.eq_ignore_ascii_case("app") => rest.to_vec(),
            _ => return Ok(None),
        }
    } else if name.starts_with(['\\', '/']) {
        return Err(format!("file path rooted without a drive, or UNC: {name:?}"));
    } else {
        split(name)
    };
    if parts.contains(&"..") {
        return Err(format!("file path with '..': {name:?}"));
    }
    Ok(Some(parts))
}

/// `host_file` (file_bridge.py 202-240): the host file a guest name means under C:\w2m\app, which is `root` on the
/// host, each component matched case-folded; None when no file is there.
pub fn host_file(root: &std::path::Path, name: &str) -> Result<Option<PathBuf>, String> {
    let Some(parts) = app_parts(name)? else {
        return Ok(None);
    };
    let mut here = root.to_path_buf();
    for part in parts {
        let Ok(entries) = std::fs::read_dir(&here) else {
            return Ok(None);
        };
        let found = entries
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .find(|e| e.eq_ignore_ascii_case(part));
        let Some(found) = found else {
            return Ok(None);
        };
        here.push(found);
    }
    Ok(here.is_file().then_some(here))
}

/// A path's components, "\" or "/" separated, empty ones and "." dropped (`_parts`).
fn split(path: &str) -> Vec<&str> {
    path.split(['\\', '/']).filter(|p| !p.is_empty() && *p != ".").collect()
}

/// `module_in`'s answer for a DLL file it found and `module` maps (Sprint 205): no handle w2m hands out is odd, so 1
/// is never one.
const FILE_FOUND: u64 = 1;

/// What `module` reads: the handles w2m hands out, by canonical name.
struct Handles<'a> {
    kernel32: u64,
    kernelbase: u64,
    ntdll: u64,
    image: Option<(String, u64)>,
    dlls: Vec<(String, u64)>,
    system: BTreeMap<String, u64>,
    app: AppDirectory<'a>,
    /// the next free system slot's address, None when the process area's slots are full (Sprint 202)
    next_system: Option<u64>,
}

/// `module` over the handles alone, in modules.py 97-132's order.
fn module_in(h: &Handles, name: &str, loading: bool) -> Result<u64, ModuleError> {
    use ModuleError::NotServed;
    let c = canonical(name);
    if c == "kernel32.dll" {
        return Ok(h.kernel32);
    }
    if c == "kernelbase.dll" {
        return Ok(h.kernelbase);
    }
    if let Some((image, base)) = &h.image
        && c == *image
    {
        return Ok(*base);
    }
    if let Some((_, host)) = API_SET_HOSTS.iter().find(|(set, _)| *set == c) {
        return module_in(h, host, loading);
    }
    if c.starts_with("api-ms-win-core-") {
        return Ok(h.kernelbase);
    }
    if c.starts_with("api-ms-") || c.starts_with("ext-ms-") {
        return Err(NotServed(format!(
            "API set {name}: its host module on Windows is not observed"
        )));
    }
    if c == "ntdll.dll" {
        if h.ntdll == 0 {
            return Err(NotServed(
                "ntdll.dll: loaded in every Windows process, not modeled".into(),
            ));
        }
        return Ok(h.ntdll);
    }
    if let Some((_, base)) = h.dlls.iter().find(|(n, _)| *n == c) {
        return Ok(*base);
    }
    if let Some(handle) = h.system.get(&c) {
        return Ok(*handle);
    }
    if !loading {
        return Ok(0);
    }
    // `_load_dll` (vm_backend 3906-3918): a file the name finds, full or relative to C:\w2m\app, is mapped (`module`
    // does it, Sprint 205); FILE_FOUND marks it here
    if h.app.host_file(name)? {
        return Ok(FILE_FOUND);
    }
    // modules.py 124-129: a System32 DLL whose exports the spec lists is a synthetic system image in the next free
    // slot, as one imported at the start has (Sprint 202: combase.dll at the batch run's row 49)
    if spec().system32.contains_key(&c) && spec().exports.contains_key(&c[..c.len() - 4]) {
        return h.next_system.ok_or_else(|| {
            NotServed(format!(
                "{c}: the process area holds {} system DLLs",
                guest_runtime::layout::SYSTEM_IMAGES
            ))
        });
    }
    // modules.py 130-131
    if spec().system32.contains_key(&c) || h.app.contains(&c) {
        return Err(NotServed(format!(
            "LoadLibrary {name}: a Windows DLL w2m does not model"
        )));
    }
    // modules.py 132: a name found nowhere (Sprint 196; reviewer-2's Medium 1 of its review of 185-189)
    Ok(0)
}

/// `file_name` (134-147): the module's path, None for a handle w2m did not hand out.
pub fn file_name(guest: &Guest, handle: u64) -> Option<String> {
    let p = &guest.process;
    if let Some(image) = guest.images.first()
        && handle == image.base
    {
        return Some(format!("{APP_DIRECTORY}{}", image.name));
    }
    if handle == p.kernel32 {
        return Some("C:\\Windows\\System32\\KERNEL32.DLL".into());
    }
    if handle == p.kernelbase {
        return Some("C:\\Windows\\System32\\KERNELBASE.dll".into());
    }
    if let Some((c, _, _, i)) = dlls(guest).find(|(_, base, ..)| *base == handle) {
        // a DLL loaded after the start by its full path (`dll_paths`, modules.py 143; Sprint 205)
        return Some(
            guest
                .dll_paths
                .get(&c)
                .cloned()
                .unwrap_or_else(|| format!("{APP_DIRECTORY}{}", guest.images[i].name)),
        );
    }
    p.system
        .values()
        .find(|(h, _)| *h == handle)
        .map(|(_, spelling)| format!("C:\\Windows\\System32\\{spelling}"))
}

/// `proc` (160-190): GetProcAddress, a thunk or an export's address, 0 when the module does not export `name`.
pub fn proc(guest: &mut Guest, handle: u64, name: &str) -> Result<u64, ModuleError> {
    let export = |i: usize| {
        let image = &guest.images[i];
        image
            .pe
            .exports
            .iter()
            .find(|(n, _)| n == name)
            .map_or(0, |(_, rva)| image.base + u64::from(*rva))
    };
    if guest.images.first().is_some_and(|image| image.base == handle) {
        return Ok(export(0));
    }
    if let Some((_, _, _, i)) = dlls(guest).find(|(_, base, ..)| *base == handle) {
        return Ok(export(i));
    }
    let system = guest
        .process
        .system
        .iter()
        .find(|(_, (h, _))| *h == handle)
        .map(|(c, (_, spelling))| (c.clone(), spelling.clone()));
    if let Some((c, spelling)) = system {
        let stem = &c[..c.len() - 4];
        if !spec().exports.get(stem).is_some_and(|names| names.contains(name)) {
            return Ok(0);
        }
        // the program's own import thunk when it imports the name (Windows answers the import's address,
        // proc.imported), otherwise GetProcAddress's own thunk per module and name (modules.py 172-177)
        if let Some(t) = guest.imports.get(&format!("{c}!{}", name.to_lowercase())) {
            return Ok(*t);
        }
        return proc_thunk(guest, format!("{spelling}!{name}"));
    }
    let p = &guest.process;
    let module = if handle == p.kernel32 {
        "kernel32"
    } else if handle == p.kernelbase {
        "kernelbase"
    } else if handle != 0 && handle == p.ntdll {
        "ntdll"
    } else {
        return Err(ModuleError::NotServed(format!(
            "GetProcAddress on module 0x{handle:x}, which w2m did not hand out"
        )));
    };
    if !spec().exports.get(module).is_some_and(|names| names.contains(name)) {
        return Ok(0);
    }
    // modules.py 185-190: kernel32's imported names answer with the import thunk
    if module == "kernel32"
        && let Some(t) = guest.imports.get(&format!("kernel32.dll!{}", name.to_lowercase()))
    {
        return Ok(*t);
    }
    let key = if module == "ntdll" {
        format!("ntdll.dll!{name}")
    } else {
        format!("{}.dll!{name}", module.to_uppercase())
    };
    proc_thunk(guest, key)
}

/// GetProcAddress's own thunk for `key` ("DLL!Name" as asked), made once (`Modules._thunks`, modules.py 176, 189;
/// Sprint 209: before, `import_thunk` reused any thunk of the name, the trace's own among them).
fn proc_thunk(guest: &mut Guest, key: String) -> Result<u64, ModuleError> {
    if let Some(t) = guest.proc_thunks.get(&key) {
        return Ok(*t);
    }
    let t = guest
        .make_thunk(&key)
        .map_err(|e| ModuleError::Failed(format!("{e:?}")))?;
    guest.proc_thunks.insert(key, t);
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::{AppDirectory, FILE_FOUND, Handles, ModuleError, canonical, module_in};

    /// A process of the program, one DLL beside it and one system image; the synthetic pages at 0x1000 steps.
    fn handles() -> Handles<'static> {
        Handles {
            kernel32: 0x1000,
            kernelbase: 0x2000,
            ntdll: 0x3000,
            image: Some(("game.exe".into(), 0x1_4000_0000)),
            dlls: vec![("unityplayer.dll".into(), 0x1_8000_0000)],
            system: [("user32.dll".to_string(), 0x4000)].into_iter().collect(),
            app: AppDirectory {
                root: Some(app_root()),
                program: "Game.exe".into(),
                files: None,
            },
            next_system: Some(0x5000),
        }
    }

    /// C:\w2m\app on the host: the program, a DLL beside it, and mono's DLL two directories down, as the batch run's
    /// depot holds MonoBleedingEdge/EmbedRuntime/mono-2.0-bdwgc.dll.
    fn app_root() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("w2m-modules-{}", std::process::id()));
        let deep = root.join("MonoBleedingEdge").join("EmbedRuntime");
        std::fs::create_dir_all(&deep).unwrap();
        for file in [
            root.join("Game.exe"),
            root.join("Plugin.dll"),
            deep.join("mono-2.0-bdwgc.dll"),
        ] {
            std::fs::write(file, b"MZ").unwrap();
        }
        root
    }

    /// LoadLibrary of a name not in the process (modules.py 117-132, file_bridge.py 202-240): a DLL file the name finds,
    /// beside the program or below it by a full or relative path, is a file to map (Sprint 205); a system DLL not
    /// imported stops by name; and a name found nowhere is 0 (Sprint 196; reviewer-2's High 2 of its review of 193-196).
    #[test]
    fn a_load_of_a_name_found_nowhere_is_0() {
        let h = handles();
        for found in [
            "C:\\w2m\\app\\MonoBleedingEdge\\EmbedRuntime\\mono-2.0-bdwgc.dll",
            "monobleedingedge/embedruntime/MONO-2.0-BDWGC.DLL",
        ] {
            assert_eq!(module_in(&h, found, true), Ok(FILE_FOUND), "{found}");
        }
        assert_eq!(module_in(&h, "C:\\Windows\\System32\\w2m-none.dll", true), Ok(0));
        // each a refusal, which ends a run as unserved, not a failure (Sprint 218)
        let refused = |r: Result<u64, ModuleError>, why: &str| {
            assert!(
                matches!(&r, Err(ModuleError::NotServed(m)) if m.contains(why)),
                "{why}: {r:?}"
            )
        };
        refused(module_in(&h, "D:\\x.dll", true), "other than C:");
        assert_eq!(module_in(&h, "w2m-no-such-module.dll", true), Ok(0));
        assert_eq!(module_in(&h, "Plugin.dll", true), Ok(FILE_FOUND));
        // modules.py 124-131 (Sprint 202): a System32 DLL whose exports the spec lists loads in the next system slot,
        // the winrt API set's host combase.dll too; one the spec lacks is a DLL w2m does not model; full slots stop
        assert_eq!(module_in(&h, "dbghelp.dll", true), Ok(0x5000));
        assert_eq!(module_in(&h, "api-ms-win-core-winrt-l1-1-0.dll", true), Ok(0x5000));
        refused(
            module_in(&h, "aphostclient.dll", true),
            "a Windows DLL w2m does not model",
        );
        let full = Handles {
            next_system: None,
            ..handles()
        };
        refused(module_in(&full, "dbghelp.dll", true), "system DLLs");
    }

    /// modules.py's docstring, row by row: kernel32, kernelbase, the program, its DLLs, the system images, ntdll, an
    /// `api-ms-win-core-*` set (kernelbase, as Windows maps api-ms-win-core-synch-l1-2-0), the winrt sets (combase,
    /// not imported here: GetModuleHandle's 0), another API set a stop, and GetModuleHandle's 0 for a name not loaded.
    #[test]
    fn each_name_means_the_module_modules_py_gives() {
        let h = handles();
        assert_eq!(module_in(&h, "KERNEL32.DLL", false), Ok(0x1000));
        assert_eq!(module_in(&h, "kernelbase", false), Ok(0x2000));
        assert_eq!(module_in(&h, "C:\\w2m\\app\\Game.exe", false), Ok(0x1_4000_0000));
        assert_eq!(module_in(&h, "UnityPlayer.dll", true), Ok(0x1_8000_0000));
        assert_eq!(module_in(&h, "USER32", false), Ok(0x4000));
        assert_eq!(module_in(&h, "ntdll.dll", false), Ok(0x3000));
        assert_eq!(module_in(&h, "api-ms-win-core-synch-l1-2-0", true), Ok(0x2000));
        assert_eq!(module_in(&h, "api-ms-win-core-winrt-l1-1-0.dll", false), Ok(0));
        assert_eq!(
            module_in(&h, "api-ms-win-shcore-scaling-l1-1-1", true),
            Err(ModuleError::NotServed(
                "API set api-ms-win-shcore-scaling-l1-1-1: its host module on Windows is not observed".into()
            ))
        );
        assert_eq!(module_in(&h, "nothing.dll", false), Ok(0));
    }

    /// GetProcAddress's thunks apart from the import thunks (modules.py 172-190, `Modules._thunks`; Sprint 210, owed by
    /// Sprint 209's tail): kernel32's imported name answers with its import thunk; a name not imported gets a thunk of
    /// its own, the same one when asked again; and `import_thunk` of that name later makes another, not GetProcAddress's.
    /// The guest needs the runtime image clang builds, so the test runs on the Mac, as `tests/bridges.rs` does.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn getprocaddress_keeps_its_thunks_apart_from_the_imports() {
        use super::proc;
        use crate::guest::{Guest, vm_compiler_symbols};
        use crate::process::Start;
        use guest_runtime::layout::{RUNTIME_IMAGE_VA, runtime_defines};
        use std::path::Path;

        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let data = std::fs::read(root.join("tests/data/timed_waits/qpc.exe")).unwrap();
        let loaded = pe_loader::load(&data, None, "probe.exe").unwrap();
        let (_, image) = guest_runtime::build(&root, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
        let services = guest_runtime::services::services(&guest_runtime::texts(&root).unwrap()).unwrap();
        let table = std::fs::read_to_string(root.join("tools/vm_compiler/stencil_table.h")).unwrap();
        let symbols = vm_compiler_symbols(&table).unwrap();
        let exports: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join("specs/windows_exports.json")).unwrap()).unwrap();
        let system = exports["modules"].as_object().unwrap().keys().cloned().collect();
        let mut g = Guest::new(
            &loaded,
            &image,
            &services,
            &Start::console("probe.exe"),
            &system,
            &symbols,
        )
        .unwrap();
        let kernel32 = g.process.kernel32;
        let imported = g.imports["kernel32.dll!queryperformancecounter"];
        assert_eq!(proc(&mut g, kernel32, "QueryPerformanceCounter"), Ok(imported));
        let own = proc(&mut g, kernel32, "Sleep").unwrap();
        assert_ne!(own, 0, "kernel32 exports Sleep");
        assert!(
            !g.imports.values().any(|&t| t == own),
            "Sleep's thunk is GetProcAddress's own"
        );
        assert_eq!(proc(&mut g, kernel32, "Sleep"), Ok(own), "made once");
        let import = g.import_thunk("KERNEL32.dll!Sleep").unwrap();
        assert_ne!(import, own, "an import of Sleep makes its own thunk");
        assert_eq!(
            proc(&mut g, kernel32, "Sleep"),
            Ok(import),
            "and GetProcAddress then answers the import's"
        );
    }

    /// `_canonical`: the last component, lowercase, ".dll" when the name has no extension (LoadLibrary's rule).
    #[test]
    fn a_name_is_its_last_component_lowercased_with_dll_added() {
        assert_eq!(
            canonical("api-ms-win-core-synch-l1-2-0"),
            "api-ms-win-core-synch-l1-2-0.dll"
        );
        assert_eq!(canonical("C:\\Windows\\System32\\USER32.dll"), "user32.dll");
        assert_eq!(canonical("a/b/Mono.DLL"), "mono.dll");
        assert_eq!(canonical("kernel.appcore.dll"), "kernel.appcore.dll");
    }
}
