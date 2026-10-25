# Sprint 140 — The process area, as Windows lays it out

```yaml
---
id: 140
status: done
phase: 6
pass_kind: functional
---
```

## scope

The first link of `docs/design-host-compile-path-2026-10-24.md`'s order (the run loop to the first declined block),
split by technique #17 before any code: reading `src/w2m/vm_backend.py` showed the process area alone is four
functions and some 214 lines, with judges of its own from Windows. This sprint writes the process area the program
and the guest runtime read at start; Sprint 141 binds the bridges and runs `qpc` to its first declined block.

`crates/w2m_vm/src/process.rs` carries, from a `pe_loader::Loaded` into the guest memory the VM maps at `PROCESS_VA`
and at `PARAMETERS_VA`: `_init_process` (2803-2859: the clock, start and foreground modes, the command line, the
environment block, the inherited handles page, the TEB's stack, ids, TLS array and PEB fields, the PEB's image base,
heap and processor count, the thread-start defaults, the ring and trace headers), `_init_parameters` (2861-2903:
RTL_USER_PROCESS_PARAMETERS and the current directory), the synthetic system images of `_modules` (2905-2956: the
ntdll, KERNEL32 and KERNELBASE pages and one page per imported system DLL), and `_write_loader_data` (2970-3014: PEB.Ldr
and its three lists). The module table that `_modules` returns (`src/w2m/modules.py`) is not carried here; its
services come with the bridges.

## prerequisites

- 139 (closed)

## context_files

- `src/w2m/vm_backend.py` 2803-3014 (read in full), 2335-2511 (`VM.__init__`, read in full: the memory layout the
  areas sit in)
- `contracts/windows.process.parameters/0.1.0.json`, `contracts/windows.loader.static_import/0.1.0.json`,
  `contracts/windows.memory.thread_stack/0.1.0.json`
- `crates/guest_runtime/src/layout.rs` (the offsets), `crates/pe_loader`

## signal contract

### Emits

None yet: the record comes at the skeleton's gate.

### Consumes

`pe_loader::Loaded`; the layout.

### Invariants

- No Python file added or changed; no Python run or written.

## artifact contract

### Files created

- `crates/w2m_vm/src/process.rs`, `crates/w2m_vm/tests/process_area.rs`

### Files modified

- `crates/w2m_vm/` (`lib.rs`, `Cargo.toml`), `Cargo.lock`, `WORKING_AGREEMENT.md`, `BLACKBOARD.md` (Sprint 139's CI
  run, in this sprint's tail entry)

### Content assertions

- Each function cites the `vm_backend.py` lines it carries.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values**, read back from the guest memory the Rust host wrote for `qpc.exe` (console start, no parent):
  - the rows of `windows.process.parameters` that a started program's own block shows (`parent.header`: Flags 0x4001,
    MaximumLength equal to Length; `parent.console_handle`; `parent.standard_handles` for a console start;
    `parent.current_directory`: a final backslash, a 0x208-byte buffer outside the block; `parent.dll_path`;
    `parent.image_path`, `parent.command_line`, `parent.window` and `parent.strings` inside the block;
    `parent.environment` outside it), each row cited;
  - `windows.loader.static_import` rows `ldr.load_order`, `ldr.memory_order` and `ldr.init_order` for the names and
    order w2m writes (the program, ntdll.dll, KERNEL32.DLL, KERNELBASE.dll; init ntdll.dll, KERNELBASE.dll,
    KERNEL32.DLL), and `ldr.first_is_exe`; the lists walked forward and back from their heads;
  - `windows.memory.thread_stack`'s layout of the first thread's stack in the TEB (DeallocationStack, the guard,
    StackLimit, StackBase).
- **Planted fault:** the init-order list linked in load order: the `ldr.init_order` check fails and names the first
  wrong entry.

## done criteria

A program loaded from Rust finds its TEB, PEB, process parameters and loader lists where and as Windows puts them.

## self-review checklist

- [x] Scope names one verifiable property: the process area against the Windows rows.
- [x] context_files cite every Python line carried, read in full.
- [x] Emits: none yet, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values from Windows' rows, and a planted fault.
- [x] One concept: the process area.

## result

- `crates/w2m_vm/src/process.rs`: `write` carries `_init_process`, `_init_parameters`, the synthetic images of
  `_modules` and `_write_loader_data` into three byte areas (the monitor area, the process area, the heap's first
  64 KB), each function citing its lines; `Start` holds `VM.__init__`'s arguments the area reads. The module table
  `_modules` returns is not carried; its services come with the bridges.
- Expected values (`tests/process_area.rs`, 5 tests): the parameters block meets `windows.process.parameters`'s rows a
  started program's own block shows (`parent.present`, `.header`, `.console_handle`, `.current_directory`,
  `.dll_path`, `.image_path`, `.command_line`, `.environment`, `.strings`, `.drive_directories`, `.block`); the
  loader's three lists, walked forward and checked backward, meet `ldr.load_order`, `ldr.memory_order`,
  `ldr.init_order` and `ldr.first_is_exe` for `qpc` and for the loader probe the rows were observed on; the TEB's
  stack meets `teb`'s `limit_above_dealloc` and `aligned` and the layout `query.dealloc` gives; a console start's
  monitor modes are 0. Rows that need a running guest (GetStdHandle's answer, GetCommandLineW's buffer, VirtualQuery,
  `parent.window`) wait for Sprint 141.
- One row is not met, as in the Python host: `teb.reserve_is_header`. `qpc`, built by mingw, asks a 0x200000 stack
  reserve; `loader.py` reserves a fixed 0x100000, the MSVC probes' size the row was observed on. The test pins w2m's
  value and names the row. It is left until the stored records are made again, because the stack base moves with it.
- Planted fault: the init list linked in load order: "ldr.init_order", `["ntdll.dll", "KERNEL32.DLL",
  "KERNELBASE.dll"]` against `["ntdll.dll", "KERNELBASE.dll", "KERNEL32.DLL"]`; restored, the tests pass. A first
  planted test that reversed `qpc`'s dependency order planted nothing (`qpc` imports no DLL of its own) and was
  removed before it ran.
- Tests: w2m_vm 5 unit, 5 test files (one new).
