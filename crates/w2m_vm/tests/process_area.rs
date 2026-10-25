// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 140's judge: the process area the Rust host writes for `tests/data/timed_waits/qpc.exe`, read back and held
//! to the rows Windows showed (`contracts/windows.process.parameters`, `contracts/windows.loader.static_import`,
//! `contracts/windows.memory.thread_stack`). Rows that need a running guest (GetStdHandle's answer, GetCommandLineW's
//! buffer, VirtualQuery) wait for the run loop.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use guest_runtime::layout::{
    CURRENT_DIRECTORY_OFF, ENVIRONMENT_OFF, HEAP_VA, LOADER_DATA_OFF, MONITOR_SIZE, PEB_OFF, PROCESS_VA, TEB_OFF,
    TRACE_OFF,
};
use pe_loader::{Loaded, load};
use w2m_vm::process::{
    APP_DIRECTORY, Areas, CURRENT_DIRECTORY_MAX, DESKTOP, PARAMETERS_ROOM, PARAMETERS_VA, PROCESS_ID, ProcessArea,
    Start, TRACE_SIZE, write,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The module names of `specs/windows_exports.json` (vm_backend 2929).
fn system_modules() -> BTreeSet<String> {
    let v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root().join("specs/windows_exports.json")).unwrap()).unwrap();
    v["modules"].as_object().unwrap().keys().cloned().collect()
}

/// The three areas, written for `qpc.exe`, readable by guest address.
struct Guest {
    monitor: Vec<u8>,
    process: Vec<u8>,
    heap: Vec<u8>,
    loaded: Loaded,
    area: ProcessArea,
}

impl Guest {
    fn new() -> Guest {
        let data = fs::read(root().join("tests/data/timed_waits/qpc.exe")).unwrap();
        Guest::of(load(&data, None, "probe.exe").unwrap())
    }

    /// The loader probe of `windows.loader.static_import` (family "loader", run 38047927781): loader.exe and the DLL
    /// it imports, loader_dll.dll (`tests/test_loader_images.py` 18-20).
    fn loader_probe() -> Guest {
        let bin = |n: &str| fs::read(root().join("store/binaries").join(n)).unwrap();
        let dll = bin("6a3dd26e2da2b4f25525c3f0940e7d03b6c3b1bf3ce325808d738ce2a0c62ff7.dll");
        let supply = move |n: &str| (n == "loader_dll.dll").then(|| dll.clone());
        let exe = bin("2bc372b71143f8a74323f8ecac7f5a73ec622d6a329e9000647f5e779d6d8e04.exe");
        Guest::of(load(&exe, Some(&supply), "probe.exe").unwrap())
    }

    fn of(loaded: Loaded) -> Guest {
        let mut monitor = vec![0u8; MONITOR_SIZE as usize];
        let mut process = vec![0u8; (TRACE_OFF + TRACE_SIZE) as usize];
        let mut heap = vec![0u8; PARAMETERS_ROOM as usize];
        let area = write(
            &mut Areas {
                monitor: &mut monitor,
                process: &mut process,
                heap: &mut heap,
            },
            &loaded,
            &Start::console("probe.exe"),
            &system_modules(),
        )
        .unwrap();
        Guest {
            monitor,
            process,
            heap,
            loaded,
            area,
        }
    }

    fn bytes(&self, va: u64, n: usize) -> &[u8] {
        let (mem, off) = if (PROCESS_VA..PROCESS_VA + self.process.len() as u64).contains(&va) {
            (&self.process, va - PROCESS_VA)
        } else if (HEAP_VA..HEAP_VA + self.heap.len() as u64).contains(&va) {
            (&self.heap, va - HEAP_VA)
        } else {
            panic!("0x{va:x} is in no area the test holds")
        };
        &mem[off as usize..off as usize + n]
    }

    fn q(&self, va: u64) -> u64 {
        u64::from_le_bytes(self.bytes(va, 8).try_into().unwrap())
    }

    fn d(&self, va: u64) -> u32 {
        u32::from_le_bytes(self.bytes(va, 4).try_into().unwrap())
    }

    fn h(&self, va: u64) -> u16 {
        u16::from_le_bytes(self.bytes(va, 2).try_into().unwrap())
    }

    /// A UNICODE_STRING at `va`: (Length, MaximumLength, Buffer).
    fn unicode(&self, va: u64) -> (u16, u16, u64) {
        (self.h(va), self.h(va + 2), self.q(va + 8))
    }

    fn text(&self, va: u64) -> String {
        let (n, _, buffer) = self.unicode(va);
        let units: Vec<u16> = self
            .bytes(buffer, usize::from(n))
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        String::from_utf16(&units).unwrap()
    }
}

const TEB: u64 = PROCESS_VA + TEB_OFF;
const PEB: u64 = PROCESS_VA + PEB_OFF;

/// `windows.process.parameters`, the rows a started program's own block shows.
#[test]
fn the_parameters_block_is_windows() {
    let g = Guest::new();
    let p = g.q(PEB + 0x20);
    assert_eq!(p, PARAMETERS_VA, "parent.present");
    let length = g.d(p + 4);
    let inside = |va: u64| (p..p + u64::from(length)).contains(&va);
    // parent.header: Flags 0x4001, DebugFlags 0, ConsoleFlags 0, MaximumLength equal to Length, a group neither 0 nor
    // the process's own id
    assert_eq!(
        (g.d(p), g.d(p + 8), g.d(p + 0xC), g.q(p + 0x18)),
        (length, 0x4001, 0, 0),
        "parent.header"
    );
    let group = u64::from(g.d(p + 0x408));
    assert!(group != 0 && group != PROCESS_ID, "parent.header: the group {group}");
    assert_eq!(g.q(p + 0x10), 0, "parent.console_handle");
    // parent.current_directory: MaximumLength 0x208, the directory and its slash, a NUL after, a handle, outside the
    // block
    let (n, max, buffer) = g.unicode(p + 0x38);
    let dir = "C:\\w2m\\app\\";
    assert_eq!(
        (max, buffer),
        (CURRENT_DIRECTORY_MAX, PROCESS_VA + CURRENT_DIRECTORY_OFF)
    );
    assert_eq!(
        (usize::from(n), g.text(p + 0x38)),
        (2 * dir.len(), dir.to_string()),
        "parent.current_directory"
    );
    assert_eq!(g.h(buffer + u64::from(n)), 0);
    assert!(g.q(p + 0x48) != 0 && !inside(buffer));
    assert_eq!(g.unicode(p + 0x50), (0, 0, 0), "parent.dll_path");
    // parent.image_path and parent.command_line: inside the block, MaximumLength two more than Length, a NUL after
    let module = format!("{APP_DIRECTORY}probe.exe");
    let line = format!("\"{module}\"");
    for (off, text, row) in [
        (0x60, module.as_str(), "parent.image_path"),
        (0x70, line.as_str(), "parent.command_line"),
        (0xB0, module.as_str(), "parent.strings: the title"),
        (0xC0, DESKTOP, "parent.strings: the desktop"),
    ] {
        let (n, max, buffer) = g.unicode(p + off);
        assert_eq!(g.text(p + off), text, "{row}");
        assert_eq!((max - n, g.h(buffer + u64::from(n))), (2, 0), "{row}");
        assert!(inside(buffer), "{row}: the buffer inside the block");
    }
    // parent.strings: no shell info, no runtime data; parent.drive_directories: none set
    assert_eq!((g.unicode(p + 0xD0).0, g.unicode(p + 0xE0).0), (0, 0), "parent.strings");
    assert!(
        (0..32).all(|i| g.unicode(p + 0xF0 + 0x18 * i).0 == 0),
        "parent.drive_directories"
    );
    // parent.environment: outside the block, its size the bytes in use (an empty block: two NULs)
    assert_eq!(g.q(p + 0x80), PROCESS_VA + ENVIRONMENT_OFF);
    assert!(!inside(g.q(p + 0x80)), "parent.environment");
    assert_eq!(g.q(p + 0x3F0), g.area.environment_bytes);
    assert_eq!(g.area.environment_bytes, 4);
    // parent.block: in the process heap
    assert!((HEAP_VA..HEAP_VA + 0x1000_0000).contains(&p), "parent.block");
}

/// The names of one of PEB.Ldr's lists, walked forward from its head and checked backward.
fn ldr_list(g: &Guest, head_off: u64, link_off: u64) -> Vec<(String, u64)> {
    let ldr = g.q(PEB + 0x18);
    assert_eq!(ldr, PROCESS_VA + LOADER_DATA_OFF);
    let head = ldr + head_off;
    let (mut out, mut node, mut prev) = (Vec::new(), g.q(head), head);
    while node != head {
        assert_eq!(g.q(node + 8), prev, "Blink of 0x{node:x}");
        let entry = node - link_off;
        out.push((g.text(entry + 0x58), g.q(entry + 0x30)));
        prev = node;
        node = g.q(node);
        assert!(out.len() < 64, "the list does not close");
    }
    assert_eq!(g.q(head + 8), prev, "the head's Blink");
    out
}

/// `windows.loader.static_import` rows `ldr.load_order`, `ldr.memory_order`, `ldr.init_order`, `ldr.first_is_exe`,
/// for the modules w2m keeps (the rows also hold apphelp's absence; w2m has none).
#[test]
fn the_loader_lists_are_windows() {
    let g = Guest::new();
    let names = |list: &[(String, u64)]| list.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>();
    let load = ldr_list(&g, 0x10, 0x00);
    let memory = ldr_list(&g, 0x20, 0x10);
    let init = ldr_list(&g, 0x30, 0x20);
    assert_eq!(
        names(&load),
        ["probe.exe", "ntdll.dll", "KERNEL32.DLL", "KERNELBASE.dll"],
        "ldr.load_order"
    );
    assert_eq!(names(&memory), names(&load), "ldr.memory_order");
    assert_eq!(
        names(&init),
        ["ntdll.dll", "KERNELBASE.dll", "KERNEL32.DLL"],
        "ldr.init_order"
    );
    assert_eq!(
        load[0].1,
        g.loaded.image.as_ref().unwrap().image_base,
        "ldr.first_is_exe"
    );
    assert_eq!(
        [load[1].1, load[2].1, load[3].1],
        [g.area.ntdll, g.area.kernel32, g.area.kernelbase]
    );
}

/// `windows.memory.thread_stack` rows `teb` and the layout `query.dealloc` and `query.below_limit` give: one reserved
/// page from DeallocationStack, the three guard pages below StackLimit, StackBase the stack's top.
#[test]
fn the_first_threads_stack_is_windows() {
    let g = Guest::new();
    let (base, limit, dealloc) = (g.q(TEB + 0x08), g.q(TEB + 0x10), g.q(TEB + 0x1478));
    assert_eq!((dealloc, limit, base), g.area.stack);
    assert_eq!(
        limit - dealloc,
        4 * 0x1000,
        "query.dealloc ends at limit - 3 pages, one page above dealloc"
    );
    assert!(
        limit > dealloc && base.is_multiple_of(0x1000),
        "teb: limit_above_dealloc, aligned"
    );
    assert_eq!(
        (g.q(TEB + 0x30), g.q(TEB + 0x60)),
        (TEB, PEB),
        "TEB.Self, TEB.ProcessEnvironmentBlock"
    );
    assert_eq!((g.q(TEB + 0x40), g.q(TEB + 0x48)), (PROCESS_ID, 4), "TEB.ClientId");
    assert_eq!(g.q(TEB + 0x58), g.loaded.tls_array, "TEB.ThreadLocalStoragePointer");
    // teb.reserve_is_header is NOT met, here as in the Python host: Windows reserves the header's SizeOfStackReserve
    // (qpc, built by mingw, asks 0x200000), and loader.py reserves a fixed STACK_SIZE (0x100000), which equals the
    // MSVC-built probes' header that the row was observed on. Kept until the stored records are made again
    // (BLACKBOARD, Sprint 140's tail entry); the assertion pins what w2m does so the change is seen.
    assert_eq!(g.loaded.image.as_ref().unwrap().stack_reserve, 0x20_0000);
    assert_eq!(
        base - dealloc,
        pe_loader::STACK_SIZE,
        "w2m's fixed reserve, not the header's"
    );
}

/// vCPU 0's monitor data for a console start on the guest counter: START_MODE 0, FOREGROUND_MODE 0, CLOCK_MODE 0
/// (vm_backend 2807-2813).
#[test]
fn a_console_start_sets_the_monitor_modes() {
    let g = Guest::new();
    let word = |off: u64| u64::from_le_bytes(g.monitor[off as usize..off as usize + 8].try_into().unwrap());
    use w2m_vm::process::{CLOCK_MODE, FOREGROUND_MODE, START_MODE};
    assert_eq!((word(START_MODE), word(FOREGROUND_MODE), word(CLOCK_MODE)), (0, 0, 0));
}

/// `windows.loader.static_import` rows `ldr.load_order`, `ldr.memory_order` and `ldr.init_order` on the probe they
/// were observed on, whose DLL joins each list last (the rows' apphelp.dll is left out by their derivation).
#[test]
fn the_loader_probes_lists_are_windows() {
    let g = Guest::loader_probe();
    let names = |head: u64, link: u64| ldr_list(&g, head, link).into_iter().map(|(n, _)| n).collect::<Vec<_>>();
    let load_order = [
        "probe.exe",
        "ntdll.dll",
        "KERNEL32.DLL",
        "KERNELBASE.dll",
        "loader_dll.dll",
    ];
    assert_eq!(names(0x10, 0x00), load_order, "ldr.load_order");
    assert_eq!(names(0x20, 0x10), load_order, "ldr.memory_order");
    assert_eq!(
        names(0x30, 0x20),
        ["ntdll.dll", "KERNELBASE.dll", "KERNEL32.DLL", "loader_dll.dll"],
        "ldr.init_order"
    );
    let dll = ldr_list(&g, 0x10, 0x00)[4].1;
    assert_eq!(dll, g.loaded.images[1].base, "bases_equal_module_handles");
}
