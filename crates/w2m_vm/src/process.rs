// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The process area a program and the guest runtime read at start (Sprint 140): the TEB, the PEB, the process
//! parameters, the synthetic system images and the loader's lists, written as `src/w2m/vm_backend.py` writes them.
//! Each function cites the lines it carries. The areas are byte slices of the VM's memory: the monitor area (vCPU 0's
//! monitor data), the process area (at `PROCESS_VA`) and the heap's first pages (the parameters block).

use std::collections::{BTreeMap, BTreeSet};

use guest_runtime::layout::{
    COMMAND_LINE_OFF, CURRENT_DIRECTORY_OFF, ENVIRONMENT_OFF, ENVIRONMENT_SIZE, HEAP_VA, INHERIT_OFF,
    KERNEL32_IMAGE_OFF, KERNELBASE_IMAGE_OFF, LOADER_DATA_OFF, LOADER_DATA_SIZE, MONITOR_DATA, NTDLL_IMAGE_OFF,
    PEB_OFF, PROCESS_VA, RING_OFF, RING_SIZE, STATE_OFF, SYSTEM_IMAGES, SYSTEM_IMAGES_OFF, TEB_OFF, TRACE_OFF,
    TRACE_RECORD, TRACE_RECORDS_OFF,
};
use pe_loader::Loaded;

/// vm_backend 98-104: vCPU 0's monitor data words.
pub const START_MODE: u64 = 0x68;
pub const FOREGROUND_MODE: u64 = 0x80;
pub const CLOCK_MODE: u64 = 0x60;
pub const CLOCK_RATE: u64 = 0x90;
pub const CLOCK_EPOCH: u64 = 0xA0;
/// RUN_TABLE + this: the instruction clock's warp (vm_backend 104).
pub const CLOCK_WARP: u64 = 0xC8;
/// vm_backend 226-276, 345; modules.py 46; virtual_memory.py 36.
pub const RING_APIS: u64 = 512;
pub const RING_HEADER: u64 = 16 + 8 * RING_APIS;
pub const TRACE_SIZE: u64 = 0x80_0000;
pub const TRACE_CAPACITY: u64 = (TRACE_SIZE - TRACE_RECORDS_OFF) / TRACE_RECORD;
pub const LDR_ENTRY_SIZE: u64 = 0x120;
pub const PARAMETERS_VA: u64 = HEAP_VA + 0x1000;
pub const PARAMETERS_FIXED: usize = 0x448;
pub const PARAMETERS_FLAGS: u32 = 0x4001;
pub const CURRENT_DIRECTORY_MAX: u16 = 0x208;
pub const CURRENT_DIRECTORY_HANDLE: u64 = 0x10;
pub const DESKTOP: &str = "winsta0\\default";
pub const PROCESSORS: u32 = 4;
pub const START_GROUP: u32 = 1000;
pub const STDOUT_HANDLE: u64 = 0x74;
pub const PROCESS_ID: u64 = 4096;
pub const APP_DIRECTORY: &str = "C:\\w2m\\app\\";
pub const STACK_GUARD_PAGES: u64 = 3;
/// kernel.py 65-66: an inherited entry's kind that is a standard handle, or the parent's error mode.
pub const KIND_STANDARD: u64 = 100;
pub const KIND_ERROR_MODE: u64 = 101;
/// The bytes of the heap the parameters block may use (vm_backend 2900).
pub const PARAMETERS_ROOM: u64 = 0x1_0000;

/// Why the process area could not be written: the Python's `CompileError` and `Unimplemented`, in its words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessError(pub String);

/// How the process starts: `VM.__init__`'s arguments the process area reads (vm_backend 2335-2366).
#[derive(Debug, Clone)]
pub struct Start {
    pub image_name: String,
    pub command_line: Option<String>,
    pub environment: Vec<String>,
    pub gui: bool,
    pub foreground_held: bool,
    pub process_id: u64,
    pub tid: u64,
    /// a child's handles from its parent: (value, kind, handle); `None`: none
    pub inherited: Option<Vec<(u64, u64, u64)>>,
    pub directory: Option<String>,
    pub process_group: u32,
    /// the host serves the clock
    pub clock_from_host: bool,
    /// (instructions a second, the wall clock's start as a FILETIME)
    pub instruction_clock: Option<(u64, u64)>,
    /// the images whose calls `call_trace.c` records, at most 2 (vm_backend 2346-2348; Sprint 191: a bool until then)
    pub call_trace: Vec<String>,
    /// "DLL!export": the trace opens at the first compile of its block, as tools/jit_capture hooks the images before
    /// UnityMain (vm_backend 2481-2487); None: from the start
    pub call_trace_from: Option<String>,
    /// the runtime word's takings timed in the monitor code (Sprint 164, step 3's measure); false writes the Python's
    /// words
    pub time_runtime_word: bool,
    /// the miss stub's call of the in-VM compiler timed in the monitor code (Sprint 247); false writes the Python's
    /// words
    pub time_compile: bool,
    /// the instructions retired at each in-VM compile and install written by the monitor code (Sprint 266); false
    /// writes the Python's words
    pub instructions_at_compile: bool,
    /// `[lo, hi)`: the miss stub sends a target in it to the host before the in-VM compiler, which installs it from the
    /// cache (Sprint 257); None writes the Python's words
    pub host_range: Option<(u64, u64)>,
    /// with a host range, the range's misses go to the installer inside the VM first (Sprint 263, card 5)
    pub install_in_vm: bool,
    /// the host directory that is C:\w2m\app, the Python's file bridge root (`host_file`, `in_app_directory`;
    /// Sprint 196): the batch run's depot; None for a probe run, whose directory holds only the program (family_run.py
    /// `run_in_vm` writes probe.exe alone)
    pub app_root: Option<std::path::PathBuf>,
    /// a child's memory: the descriptor of the unlinked file the supervisor made, which the guest sizes and maps
    /// shared so the supervisor serves its requests against it (Sprint 197); None: anonymous memory
    pub memory_fd: Option<i32>,
    /// the program's file on the host: a child started by its own name finds it here when the run has no program
    /// directory (a probe run's C:\w2m\app holds only the program; Sprint 197)
    pub program_path: Option<std::path::PathBuf>,
}

impl Start {
    /// A console start of `image_name` with the Python's defaults.
    pub fn console(image_name: &str) -> Start {
        Start {
            image_name: image_name.to_string(),
            command_line: None,
            environment: Vec::new(),
            gui: false,
            foreground_held: false,
            process_id: 0,
            tid: 4,
            inherited: None,
            directory: None,
            process_group: START_GROUP,
            clock_from_host: false,
            instruction_clock: None,
            call_trace: Vec::new(),
            call_trace_from: None,
            time_runtime_word: false,
            time_compile: false,
            instructions_at_compile: false,
            host_range: None,
            install_in_vm: false,
            app_root: None,
            memory_fd: None,
            program_path: None,
        }
    }

    /// vm_backend 2360: the default command line, the program's path in quotes.
    pub fn command_line(&self) -> String {
        self.command_line
            .clone()
            .unwrap_or_else(|| format!("\"{APP_DIRECTORY}{}\"", self.image_name))
    }
}

/// The three areas the process area is written into.
pub struct Areas<'a> {
    /// the monitor area, from `MONITOR_VA`
    pub monitor: &'a mut [u8],
    /// the process area, from `PROCESS_VA`
    pub process: &'a mut [u8],
    /// the heap's first `PARAMETERS_ROOM` bytes, from `HEAP_VA`
    pub heap: &'a mut [u8],
}

fn put(mem: &mut [u8], off: u64, bytes: &[u8]) {
    mem[off as usize..off as usize + bytes.len()].copy_from_slice(bytes);
}

fn q(mem: &mut [u8], off: u64, v: u64) {
    put(mem, off, &v.to_le_bytes());
}

fn d(mem: &mut [u8], off: u64, v: u32) {
    put(mem, off, &v.to_le_bytes());
}

fn h(mem: &mut [u8], off: u64, v: u16) {
    put(mem, off, &v.to_le_bytes());
}

fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

/// A UNICODE_STRING: Length, MaximumLength, the pad, Buffer.
fn unicode_string(mem: &mut [u8], off: u64, length: u16, maximum: u16, buffer: u64) {
    h(mem, off, length);
    h(mem, off + 2, maximum);
    d(mem, off + 4, 0);
    q(mem, off + 8, buffer);
}

/// What the process area wrote that the run needs later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessArea {
    /// (DeallocationStack, StackLimit, StackBase)
    pub stack: (u64, u64, u64),
    pub environment_bytes: u64,
    /// the synthetic system images: canonical name -> (address, the spelling the program imported)
    pub system: BTreeMap<String, (u64, String)>,
    pub kernel32: u64,
    pub kernelbase: u64,
    pub ntdll: u64,
    /// TEB.ClientId.UniqueProcess (vm_backend 2362), which a created thread's TEB takes too
    pub process_id: u64,
}

/// vm_backend 2318-2325: a synthetic module's PE32+ optional header, SizeOfOptionalHeader 0xF0, SizeOfImage 0x1000.
fn synthetic_optional_header(page: &mut [u8]) {
    h(page, 0x80 + 0x14, 0xF0);
    h(page, 0x80 + 0x18, 0x20B);
    d(page, 0x80 + 0x18 + 0x38, 0x1000);
}

/// A synthetic module page: DOS and PE headers, x64, no sections (vm_backend 2909-2915, 2946-2951); `process` is the
/// process area, `off` the page's offset in it (Sprint 202: a LoadLibrary after the start writes one too).
pub fn synthetic_image(process: &mut [u8], off: u64) {
    let page = &mut process[off as usize..off as usize + 0x1000];
    page[..2].copy_from_slice(b"MZ");
    d(page, 0x3C, 0x80);
    page[0x80..0x84].copy_from_slice(b"PE\0\0");
    h(page, 0x84, 0x8664);
    h(page, 0x86, 0);
    synthetic_optional_header(page);
}

/// Writes the process area for `loaded` (vm_backend 2803-2859, 2861-2903, 2905-2956, 2970-3014). `system_modules` is
/// `specs/windows_exports.json`'s module names, the DLLs w2m gives a synthetic image when a program imports them.
pub fn write(
    areas: &mut Areas<'_>,
    loaded: &Loaded,
    start: &Start,
    system_modules: &BTreeSet<String>,
) -> Result<ProcessArea, ProcessError> {
    let err = |m: String| Err(ProcessError(m));
    let md = MONITOR_DATA;
    let mode = if start.clock_from_host {
        1
    } else if start.instruction_clock.is_some() {
        2
    } else {
        0
    };
    q(areas.monitor, md + CLOCK_MODE, mode);
    if let Some((rate, epoch)) = start.instruction_clock {
        q(areas.monitor, md + CLOCK_RATE, rate);
        q(areas.monitor, md + CLOCK_EPOCH, epoch);
        q(areas.monitor, guest_runtime::layout::RUN_TABLE + CLOCK_WARP, 0);
    }
    q(areas.monitor, md + START_MODE, u64::from(start.gui));
    q(areas.monitor, md + FOREGROUND_MODE, u64::from(start.foreground_held));

    let mut line = utf16(&start.command_line());
    line.extend([0, 0]);
    if line.len() > 0x1000 {
        return err(format!("command line of {} bytes: the page holds 4096", line.len()));
    }
    put(areas.process, COMMAND_LINE_OFF, &line);
    if start.environment.iter().any(|e| e.is_empty() || e.contains('\0')) {
        return err("an environment entry is empty or holds a NUL".into());
    }
    let mut block: Vec<u8> = start
        .environment
        .iter()
        .flat_map(|e| utf16(&format!("{e}\0")))
        .collect();
    block.extend([0, 0]);
    if start.environment.is_empty() {
        block = vec![0; 4]; // an empty block is two NULs (row empty.heap)
    }
    if block.len() as u64 > ENVIRONMENT_SIZE / 2 {
        return err(format!(
            "environment of {} bytes: the area holds {}",
            block.len(),
            ENVIRONMENT_SIZE / 2
        ));
    }
    put(areas.process, ENVIRONMENT_OFF, &block);
    let environment_bytes = block.len() as u64;

    let (teb, peb) = (PROCESS_VA + TEB_OFF, PROCESS_VA + PEB_OFF);
    let stack = loaded
        .memory
        .regions
        .iter()
        .find(|r| r.name == "stack")
        .ok_or_else(|| ProcessError("the loaded program has no stack region".into()))?;
    // the stack as Windows lays it out (contracts/windows.memory.thread_stack): DeallocationStack at the reservation's
    // bottom, one reserved page, the 3-page guard, then StackLimit; w2m commits the rest
    let limit = stack.base + (1 + STACK_GUARD_PAGES) * 0x1000;
    if let Some(inherited) = &start.inherited {
        // objects, then the standard handles (KIND_STANDARD), then the mark
        let entries: Vec<&(u64, u64, u64)> = inherited.iter().filter(|e| e.1 < KIND_STANDARD).collect();
        let standard: BTreeMap<u64, u64> = inherited
            .iter()
            .filter(|e| e.1 == KIND_STANDARD)
            .map(|&(w, _, h)| (w, h))
            .collect();
        let error_mode = inherited.iter().find(|e| e.1 == KIND_ERROR_MODE).map(|e| e.2);
        if 32 + 24 * entries.len() as u64 > 0xFE8 {
            return err(format!(
                "{} inherited handles: the page holds {}",
                entries.len(),
                (0xFE8 - 32) / 24
            ));
        }
        let page = INHERIT_OFF;
        q(areas.process, page, entries.len() as u64);
        for w in 0..3 {
            q(areas.process, page + 8 + 8 * w, standard.get(&w).copied().unwrap_or(0));
        }
        for (i, e) in entries.iter().enumerate() {
            let at = page + 32 + 24 * i as u64;
            q(areas.process, at, e.0);
            q(areas.process, at + 8, e.1);
            q(areas.process, at + 16, e.2);
        }
        // [509] standard handles given, [510] the error mode + 1 (0: none), [511] a child's page
        q(areas.process, page + 0xFE8, u64::from(!standard.is_empty()));
        q(areas.process, page + 0xFF0, error_mode.map_or(0, |m| m + 1));
        q(areas.process, page + 0xFF8, 1);
    }
    write_parameters(areas, start, environment_bytes)?;
    let process_id = if start.process_id != 0 {
        start.process_id
    } else {
        PROCESS_ID
    };
    for (field, value) in [
        (0x08, loaded.stack_top),
        (0x10, limit),
        (0x30, teb),
        (0x40, process_id),
        (0x48, start.tid),
        (0x58, loaded.tls_array),
        (0x60, peb),
        (0x1478, stack.base),
    ] {
        q(areas.process, TEB_OFF + field, value);
    }
    if let Some(image) = &loaded.image {
        q(areas.process, PEB_OFF + 0x10, image.image_base); // PEB.ImageBaseAddress (2850-2851)
    }
    q(areas.process, PEB_OFF + 0x30, HEAP_VA); // PEB.ProcessHeap (heap.c)
    d(areas.process, PEB_OFF + 0xB8, PROCESSORS); // PEB.NumberOfProcessors (Chappell)
    let st = STATE_OFF; // thread-start defaults (spec, observed on Windows)
    d(areas.process, st + 152, 0x1F80);
    h(areas.process, st + 156, 0x27F);
    q(areas.process, st + 176, teb);
    q(areas.process, st + 184, teb);
    q(areas.process, st + 192, 0);
    q(areas.process, RING_OFF, 0);
    q(areas.process, RING_OFF + 8, (RING_SIZE - RING_HEADER) / 24);
    q(areas.process, TRACE_OFF, u64::from(!start.call_trace.is_empty()));
    q(areas.process, TRACE_OFF + 8, 0);
    q(areas.process, TRACE_OFF + 16, 0);
    q(areas.process, TRACE_OFF + 24, TRACE_CAPACITY);

    let system = system_images(areas, loaded, system_modules)?;
    let area = ProcessArea {
        stack: (stack.base, limit, loaded.stack_top),
        environment_bytes,
        system,
        kernel32: PROCESS_VA + KERNEL32_IMAGE_OFF,
        kernelbase: PROCESS_VA + KERNELBASE_IMAGE_OFF,
        ntdll: PROCESS_VA + NTDLL_IMAGE_OFF,
        process_id,
    };
    if !loaded.images.is_empty() {
        // a program from the loader (vm_backend 2966-2967: tests build bare ones)
        let list = LoaderList {
            images: &loaded.images,
            image_entry: loaded.image_entry,
            image_name: &start.image_name,
            dependency_order: &loaded.dependency_order,
            paths: &BTreeMap::new(),
        };
        write_loader_data(areas.process, &list, &area)?;
    }
    Ok(area)
}

/// What PEB.Ldr lists (vm_backend 2978-2983): the images in load order, the program first; its entry and name; every
/// DLL dependencies first; and a DLL's full path where it is not C:\w2m\app\<name>, by its lowercase name
/// (`Modules.dll_paths`: a DLL loaded after the start from a folder; Sprint 205).
pub struct LoaderList<'a> {
    pub images: &'a [pe_loader::Image],
    pub image_entry: u64,
    pub image_name: &'a str,
    pub dependency_order: &'a [usize],
    pub paths: &'a BTreeMap<String, String>,
}

/// vm_backend 2861-2903: PEB.ProcessParameters as the rows of `contracts/windows.process.parameters` show Windows
/// filling it, and the current directory.
fn write_parameters(areas: &mut Areas<'_>, start: &Start, environment_bytes: u64) -> Result<(), ProcessError> {
    let mut std = [0, if start.gui { 0 } else { STDOUT_HANDLE }, 0]; // console.c's starts: the GUI start has none
    let given: BTreeMap<u64, u64> = start
        .inherited
        .iter()
        .flatten()
        .filter(|e| e.1 == KIND_STANDARD)
        .map(|&(w, _, h)| (w, h))
        .collect();
    if !given.is_empty() {
        std = [0, 1, 2].map(|i| given.get(&i).copied().unwrap_or(0));
    }
    let directory = start
        .directory
        .clone()
        .unwrap_or_else(|| APP_DIRECTORY.trim_end_matches('\\').to_string());
    let dos_path = if directory.ends_with('\\') {
        directory
    } else {
        format!("{directory}\\")
    };
    let dos_units = dos_path.encode_utf16().count();
    if 2 * dos_units + 2 > usize::from(CURRENT_DIRECTORY_MAX) {
        return Err(ProcessError(format!(
            "a current directory of {dos_units} characters: the buffer holds {}",
            CURRENT_DIRECTORY_MAX / 2 - 1
        )));
    }
    let mut cwd = utf16(&dos_path);
    cwd.extend([0, 0]);
    put(areas.process, CURRENT_DIRECTORY_OFF, &cwd);

    let image = format!("{APP_DIRECTORY}{}", start.image_name);
    let command_line = start.command_line();
    // ImagePathName, CommandLine, WindowTitle, DesktopInfo
    let strings = [image.as_str(), command_line.as_str(), image.as_str(), DESKTOP];
    let mut data = vec![0u8; PARAMETERS_FIXED];
    let mut laid = Vec::new();
    for text in strings {
        let mut raw = utf16(text);
        raw.extend([0, 0]);
        laid.push((
            (raw.len() - 2) as u16,
            raw.len() as u16,
            PARAMETERS_VA + data.len() as u64,
        ));
        data.extend(&raw);
        data.resize(data.len().next_multiple_of(8), 0);
    }
    let length = data.len() as u32;
    d(&mut data, 0, length);
    d(&mut data, 4, length);
    d(&mut data, 8, PARAMETERS_FLAGS);
    d(&mut data, 12, 0);
    q(&mut data, 0x10, 0); // ConsoleHandle
    q(&mut data, 0x18, 0); // ConsoleFlags
    q(&mut data, 0x20, std[0]);
    q(&mut data, 0x28, std[1]);
    q(&mut data, 0x30, std[2]);
    unicode_string(
        &mut data,
        0x38,
        (2 * dos_units) as u16,
        CURRENT_DIRECTORY_MAX,
        PROCESS_VA + CURRENT_DIRECTORY_OFF,
    );
    q(&mut data, 0x48, CURRENT_DIRECTORY_HANDLE);
    for (off, (n, maximum, buffer)) in [0x60, 0x70, 0xB0, 0xC0].into_iter().zip(laid) {
        unicode_string(&mut data, off, n, maximum, buffer);
    }
    q(&mut data, 0x80, PROCESS_VA + ENVIRONMENT_OFF);
    q(&mut data, 0x3F0, environment_bytes); // EnvironmentSize
    q(&mut data, 0x3F8, 3); // EnvironmentVersion
    d(&mut data, 0x408, start.process_group);
    if PARAMETERS_VA + u64::from(length) > HEAP_VA + PARAMETERS_ROOM {
        return Err(ProcessError(format!(
            "process parameters of {length} bytes: the heap's first pages hold {}",
            0xF000
        )));
    }
    put(areas.heap, PARAMETERS_VA - HEAP_VA, &data);
    q(areas.process, PEB_OFF + 0x20, PARAMETERS_VA); // PEB.ProcessParameters
    Ok(())
}

/// vm_backend 2905-2956: the ntdll, KERNEL32 and KERNELBASE pages, and a page for each system DLL the program
/// imports, in name order.
fn system_images(
    areas: &mut Areas<'_>,
    loaded: &Loaded,
    system_modules: &BTreeSet<String>,
) -> Result<BTreeMap<String, (u64, String)>, ProcessError> {
    for off in [KERNEL32_IMAGE_OFF, KERNELBASE_IMAGE_OFF, NTDLL_IMAGE_OFF] {
        synthetic_image(areas.process, off);
    }
    let own: BTreeSet<String> = loaded.images.iter().skip(1).map(|d| d.name.to_lowercase()).collect();
    let mut spellings: BTreeMap<String, String> = BTreeMap::new();
    for name in loaded.thunks.values() {
        let dll = name.split_once('!').map_or(name.as_str(), |(d, _)| d);
        let c = if dll.contains('.') {
            dll.to_lowercase()
        } else {
            format!("{}.dll", dll.to_lowercase())
        };
        let stem = &c[..c.len().saturating_sub(4)];
        if !own.contains(&c)
            && !["kernel32.dll", "kernelbase.dll", "ntdll.dll"].contains(&c.as_str())
            && system_modules.contains(stem)
        {
            spellings.entry(c).or_insert_with(|| dll.to_string());
        }
    }
    if spellings.len() as u64 > SYSTEM_IMAGES {
        return Err(ProcessError(format!(
            "{} system DLLs imported; the process area holds {SYSTEM_IMAGES}",
            spellings.len()
        )));
    }
    let mut system = BTreeMap::new();
    for (i, (c, spelling)) in spellings.into_iter().enumerate() {
        let off = SYSTEM_IMAGES_OFF + 0x1000 * i as u64;
        synthetic_image(areas.process, off);
        system.insert(c, (PROCESS_VA + off, spelling));
    }
    Ok(system)
}

/// vm_backend 2970-3014: PEB.Ldr as the loader probe's rows have it (`contracts/windows.loader.static_import`): load
/// and memory order the program, ntdll.dll, KERNEL32.DLL, KERNELBASE.dll, then the program's DLLs as loaded;
/// initialization order ntdll.dll, KERNELBASE.dll, KERNEL32.DLL, then the DLLs dependencies first. PEB.Ldr 0x18;
/// PEB_LDR_DATA Length 0, Initialized 4, the list heads at 0x10, 0x20, 0x30; LDR_DATA_TABLE_ENTRY links at 0x00, 0x10,
/// 0x20, DllBase 0x30, EntryPoint 0x38, SizeOfImage 0x40, FullDllName 0x48, BaseDllName 0x58.
pub fn write_loader_data(process: &mut [u8], list: &LoaderList<'_>, area: &ProcessArea) -> Result<(), ProcessError> {
    let (base, va0) = (LOADER_DATA_OFF, PROCESS_VA + LOADER_DATA_OFF);
    let system32 = "C:\\Windows\\System32\\";
    type Entry = (u64, u64, u64, String, String);
    let program: Entry = (
        list.images[0].base,
        list.image_entry,
        u64::from(list.images[0].pe.size_of_image),
        format!("{APP_DIRECTORY}{}", list.image_name),
        list.image_name.to_string(),
    );
    let k32: Entry = (
        area.kernel32,
        0,
        0x1000,
        format!("{system32}KERNEL32.DLL"),
        "KERNEL32.DLL".into(),
    );
    let kb: Entry = (
        area.kernelbase,
        0,
        0x1000,
        format!("{system32}KERNELBASE.dll"),
        "KERNELBASE.dll".into(),
    );
    let nt: Entry = (
        area.ntdll,
        0,
        0x1000,
        format!("{system32}ntdll.dll"),
        "ntdll.dll".into(),
    );
    let dll = |i: usize| -> Entry {
        let d = &list.images[i];
        (
            d.base,
            d.entry().unwrap_or(0),
            u64::from(d.pe.size_of_image),
            list.paths
                .get(&d.name.to_lowercase())
                .cloned()
                .unwrap_or_else(|| format!("{APP_DIRECTORY}{}", d.name)),
            d.name.clone(),
        )
    };
    let mut entries = vec![program, nt.clone(), k32.clone(), kb.clone()];
    entries.extend((1..list.images.len()).map(dll));
    let mut init = vec![nt, kb, k32];
    init.extend(list.dependency_order.iter().map(|&i| dll(i)));
    if 0x58 + LDR_ENTRY_SIZE * entries.len() as u64 > LOADER_DATA_SIZE / 2 {
        return Err(ProcessError(format!(
            "{} loader entries: the area holds fewer",
            entries.len()
        )));
    }
    let at: BTreeMap<u64, u64> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.0, va0 + 0x58 + LDR_ENTRY_SIZE * i as u64))
        .collect();
    // the names from the area's middle: the list is written again when LoadLibrary maps a DLL (Sprint 063), and an
    // entry or a name the program already holds keeps its address, the order being load order
    let mut strings = LOADER_DATA_SIZE / 2;
    for (i, (dll_base, entry, size, full, leaf)) in entries.iter().enumerate() {
        let e = base + 0x58 + LDR_ENTRY_SIZE * i as u64;
        q(process, e + 0x30, *dll_base);
        q(process, e + 0x38, *entry);
        q(process, e + 0x40, *size);
        for (field, text) in [(0x48, full), (0x58, leaf)] {
            let raw = utf16(text);
            if strings + raw.len() as u64 + 2 > LOADER_DATA_SIZE {
                return Err(ProcessError("loader names: the area is full".into()));
            }
            put(process, base + strings, &raw);
            unicode_string(
                process,
                e + field,
                raw.len() as u16,
                raw.len() as u16 + 2,
                va0 + strings,
            );
            strings += (raw.len() as u64 + 2 + 7) & !7;
        }
    }
    let mut link = |head: u64, link: u64, order: &[Entry]| {
        let nodes: Vec<u64> = std::iter::once(va0 + head)
            .chain(order.iter().map(|o| at[&o.0] + link))
            .collect();
        for (i, node) in nodes.iter().enumerate() {
            let off = base + (node - va0);
            q(process, off, nodes[(i + 1) % nodes.len()]);
            q(process, off + 8, nodes[(i + nodes.len() - 1) % nodes.len()]);
        }
    };
    link(0x10, 0x00, &entries);
    link(0x20, 0x10, &entries);
    link(0x30, 0x20, &init);
    d(process, base, 0x58);
    d(process, base + 4, 1);
    q(process, PEB_OFF + 0x18, va0);
    Ok(())
}
