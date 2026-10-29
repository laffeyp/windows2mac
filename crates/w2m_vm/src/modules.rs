// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Which module a name means, for the guest runtime's module calls (Sprint 189): `src/w2m/modules.py` (Sprint 044) in
//! Rust, over what the guest already holds: the program and its DLLs as the loader mapped them (`Guest::images`), the
//! synthetic kernel32, kernelbase, ntdll and system images (`ProcessArea`), and the thunks (`Guest::thunks`,
//! `Guest::import_thunk`). The guest runtime (`tools/guest_runtime/module.c`) asks the host three things through
//! HOST_MODULE 15, HOST_PROC 16 and HOST_MODULE_PATH 17 (vm_backend 3889-3904); HOST_MODULE_AT 18 is
//! `host_request::containing`.
//!
//! Two departures from the Python, each a stop rather than a guess: the Rust host has no file bridge yet, so a
//! LoadLibrary that would map a DLL file after the start (`_load_dll`) or a system DLL not imported (`load_system`)
//! stops by name; and a GetProcAddress thunk is `Guest::import_thunk`'s, which reuses a thunk of the same name, where
//! the Python keeps kernelbase's and ntdll's apart from the program's imports (they differ only for a program that
//! imports from KERNELBASE.dll or ntdll.dll by name, which the batch run's images do not).

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

/// `module` (94-132): the handle `name` means, 0 when it is found nowhere; `loading` is LoadLibrary, which would load
/// a DLL not yet in the process, where GetModuleHandle reports only loaded ones. Err is the Python's Unimplemented: a
/// stop by name.
pub fn module(guest: &Guest, name: &str, loading: bool) -> Result<u64, String> {
    let p = &guest.process;
    let handles = Handles {
        kernel32: p.kernel32,
        kernelbase: p.kernelbase,
        ntdll: p.ntdll,
        image: guest.images.first().map(|i| (i.name.to_lowercase(), i.base)),
        dlls: dlls(guest).map(|(n, base, ..)| (n, base)).collect(),
        system: p.system.iter().map(|(c, (h, _))| (c.clone(), *h)).collect(),
        app: AppDirectory {
            root: guest.app_root.clone(),
            program: guest.images.first().map_or(String::new(), |i| i.name.clone()),
        },
    };
    module_in(&handles, name, loading)
}

/// C:\w2m\app on the host (Sprint 196; reviewer-2's High 2 of its review of 193-196): the directory w2m-vm was given,
/// or for a probe run a directory holding only the program, as family_run.py's `run_in_vm` makes it.
struct AppDirectory {
    root: Option<PathBuf>,
    program: String,
}

impl AppDirectory {
    /// `host_file` (file_bridge.py 202-240): whether a file is where the guest name points, a full C:\ path or a path
    /// relative to C:\w2m\app, each component matched case-folded; a path the bridge refuses is Err, as it raises. A
    /// full path outside C:\w2m\app finds no file: the bridge's drive holds only empty system folders.
    fn host_file(&self, name: &str) -> Result<bool, String> {
        match &self.root {
            Some(root) => Ok(host_file(root, name)?.is_some()),
            None => Ok(matches!(app_parts(name)?.as_deref(), Some([p]) if p.eq_ignore_ascii_case(&self.program))),
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

/// What `module` reads: the handles w2m hands out, by canonical name.
struct Handles {
    kernel32: u64,
    kernelbase: u64,
    ntdll: u64,
    image: Option<(String, u64)>,
    dlls: Vec<(String, u64)>,
    system: BTreeMap<String, u64>,
    app: AppDirectory,
}

/// `module` over the handles alone, in modules.py 97-132's order.
fn module_in(h: &Handles, name: &str, loading: bool) -> Result<u64, String> {
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
        return Err(format!("API set {name}: its host module on Windows is not observed"));
    }
    if c == "ntdll.dll" {
        if h.ntdll == 0 {
            return Err("ntdll.dll: loaded in every Windows process, not modeled".into());
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
    // `_load_dll` (vm_backend 3906-3918): a file the name finds, full or relative to C:\w2m\app, would be mapped
    if h.app.host_file(name)? {
        return Err(format!(
            "LoadLibrary {name}: a DLL file after the start, not served by the Rust host yet (it has no file bridge)"
        ));
    }
    if spec().system32.contains_key(&c) {
        return Err(format!(
            "LoadLibrary {name}: a system DLL not imported at the start, not served by the Rust host yet"
        ));
    }
    if h.app.contains(&c) {
        return Err(format!("LoadLibrary {name}: a Windows DLL w2m does not model"));
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
    if let Some((_, _, _, i)) = dlls(guest).find(|(_, base, ..)| *base == handle) {
        return Some(format!("{APP_DIRECTORY}{}", guest.images[i].name));
    }
    p.system
        .values()
        .find(|(h, _)| *h == handle)
        .map(|(_, spelling)| format!("C:\\Windows\\System32\\{spelling}"))
}

/// `proc` (160-190): GetProcAddress, a thunk or an export's address, 0 when the module does not export `name`.
pub fn proc(guest: &mut Guest, handle: u64, name: &str) -> Result<u64, String> {
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
        // the program's own import thunk when it imports the name (Windows answers the import's address), otherwise a
        // thunk per module and name
        return guest
            .import_thunk(&format!("{spelling}!{name}"))
            .map_err(|e| format!("{e:?}"));
    }
    let p = &guest.process;
    let module = if handle == p.kernel32 {
        "kernel32"
    } else if handle == p.kernelbase {
        "kernelbase"
    } else if handle != 0 && handle == p.ntdll {
        "ntdll"
    } else {
        return Err(format!(
            "GetProcAddress on module 0x{handle:x}, which w2m did not hand out"
        ));
    };
    if !spec().exports.get(module).is_some_and(|names| names.contains(name)) {
        return Ok(0);
    }
    let key = if module == "ntdll" {
        format!("ntdll.dll!{name}")
    } else {
        format!("{}.dll!{name}", module.to_uppercase())
    };
    guest.import_thunk(&key).map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::{AppDirectory, Handles, canonical, module_in};

    /// A process of the program, one DLL beside it and one system image; the synthetic pages at 0x1000 steps.
    fn handles() -> Handles {
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
            },
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
    /// beside the program or below it by a full or relative path, stops by name (no file bridge yet); a system DLL not
    /// imported stops by name; and a name found nowhere is 0 (Sprint 196; reviewer-2's High 2 of its review of 193-196).
    #[test]
    fn a_load_of_a_name_found_nowhere_is_0() {
        let h = handles();
        for found in [
            "C:\\w2m\\app\\MonoBleedingEdge\\EmbedRuntime\\mono-2.0-bdwgc.dll",
            "monobleedingedge/embedruntime/MONO-2.0-BDWGC.DLL",
        ] {
            assert!(
                module_in(&h, found, true)
                    .unwrap_err()
                    .contains("a DLL file after the start"),
                "{found}"
            );
        }
        assert_eq!(module_in(&h, "C:\\Windows\\System32\\w2m-none.dll", true), Ok(0));
        assert!(module_in(&h, "D:\\x.dll", true).unwrap_err().contains("other than C:"));
        assert_eq!(module_in(&h, "w2m-no-such-module.dll", true), Ok(0));
        assert!(
            module_in(&h, "Plugin.dll", true)
                .unwrap_err()
                .contains("a DLL file after the start")
        );
        assert!(
            module_in(&h, "dbghelp.dll", true)
                .unwrap_err()
                .contains("a system DLL not imported")
        );
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
            Err("API set api-ms-win-shcore-scaling-l1-1-1: its host module on Windows is not observed".into())
        );
        assert_eq!(module_in(&h, "nothing.dll", false), Ok(0));
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
