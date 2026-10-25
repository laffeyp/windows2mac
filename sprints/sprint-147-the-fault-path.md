# Sprint 147 — The fault path: cases Windows ended in a fault, judged through the guest's exception records

```yaml
---
id: 147
status: active
phase: 6
pass_kind: functional
---
```

## scope

The case runner's second judge (`docs/design-host-compile-path-2026-10-24.md`, link 3): the cases Windows ended in a
fault, run in the Rust VM and judged as `scripts/vm_oracle.py` `judge_fault` (96-123) judges them. A fault goes the way
a Windows fault goes in w2m. The EL1 vector saves the registers. The guest runtime (`runtime.c` `w2m_exception`) maps
the host PC to the instruction through the reconstruction table, builds the EXCEPTION_RECORD and CONTEXT, and dispatches
them (`exceptions.c`). With no handler, the walk ends the process, leaving both records in the exception area.

The walk asks the host three things, through `hvc #8` and the transport page. The Rust host gains them:

- `HOST_MODULE_AT`: `modules.py` `containing` (149-158): the image, the synthetic system images (one page each), and
  the program's DLLs;
- `HOST_EXCEPTION`: `EXCEPTION_STUBS`, the x86 stubs' page (`guest_stubs.py`, read in full), written at first use on an
  arena page (`_code_page`, `_guest_stubs`) with the thunk of `KERNEL32.dll!W2mGuestReturn` made then
  (`_import_thunk`, `make_thunk` 2920-2924) and its bridge; and `EXCEPTION_CAPTURE_XMM`, XMM0-15 to a guest address;
- `HOST_EXIT`: the process's end; `HOST_RUNTIME_FAULT` stops the run by name.

`Guest` gains the arena (`VirtualMemory`, Sprint 146, set up as `VM.__init__` 2498-2505 sets it), keeps the services and
the runtime's symbols for bridges made after construction, and reads the exception area (`exception`, 4173-4195) and
clears it (`clear_exception`, 4197-4200). `run_cases` serves `hvc #8` until the process ends, then judges the fault and
goes on to the next case, as `on_exception` (243-248) does.

Judged at a fault: the exception code, RIP, ExceptionAddress equal to RIP, the 16 GPRs, EFlags under
`EFLAGS_FAULT_MASK` (CF, bit 1, PF, AF, ZF, SF, IF, DF, OF), MXCSR, XMM0-15, and the three windows. A case one side
completed and the other faulted is its own class ("vm fault, cpu completed", "vm completed, cpu fault").

Out of scope: the YMM upper halves and the x87 state at a fault, with the vector judging of completed cases next; the
other host requests.

## prerequisites

- 146 (closed)

## context_files

- `scripts/vm_oracle.py` 52, 96-123, 126-128, 236-258; `src/w2m/vm_backend.py` 2492, 2498-2505, 2920-2924, 3087-3092,
  3796-3900 (`_serve_host_request`), 4173-4200, 5236-5256; `src/w2m/guest_stubs.py`; `src/w2m/modules.py` 149-158
- `tools/guest_runtime/runtime.c` 155-300, `exceptions.c` 100-150, 275-392, 505-520 (read)
- `crates/w2m_vm` (`guest.rs`, `cases.rs`, `process.rs`), `crates/virtual_memory`, `crates/cpu_cases`

## signal contract

### Emits

None yet: the record comes at the skeleton's gate.

### Consumes

The case files and the downloaded Windows results (Sprint 144).

### Invariants

- No Python file added or changed.

## artifact contract

### Files created

- `crates/w2m_vm/src/guest_stubs.rs`, `crates/w2m_vm/src/host_request.rs`, `crates/w2m_vm/tests/guest_stubs.rs`

### Files modified

- `crates/w2m_vm/` (`guest.rs`, `cases.rs`, `lib.rs`, `Cargo.toml`, `tests/case_runner.rs`), `Cargo.lock`,
  `WORKING_AGREEMENT.md`, `BLACKBOARD.md` (Sprint 146's CI run, in the tail entry)

### Content assertions

- Each function cites the Python lines it carries.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values, from Windows:** in the 22 Intel runs of Sprint 145, every case Windows ended in a fault whose
  instruction the given compiler compiles is judged on the fields above against the CONTEXT Windows gave its vectored
  handler, with zero difference classes, and the completed cases stay at zero. The run reports faults judged and the
  host requests served by kind.
- **Expected values, carried:** `guest_stubs.page` for a thunk address equals, byte for byte, the page the frozen Python
  writes for the same address (run once, its bytes in the test).
- **Planted fault:** the CONTEXT's RIP read 8 bytes off (from +240 in place of +248): the run names "fault rip".

## done criteria

Every case Windows ended in a fault runs to the same fault in the Rust VM, with the same exception code, RIP,
registers, flags and memory.

## self-review checklist

- [x] Scope names one verifiable property: faulting cases agree with Windows in the Rust VM.
- [x] context_files cite the Python carried.
- [x] Emits: none yet, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values from Windows, and a planted fault.
- [x] One concept: the fault path.

## result

- Amended during the sprint, by measurement. The card named three requests a fault's walk makes. The first run found
  two more before them. Every fault first stopped at `hvc #14`, `HVC_FAULT_GIVE_BACK`, whose answer gives the block's
  unrun instructions back to the budget and resumes (vm_backend 5205-5206, 5294-5296); a case's block holds one
  instruction, so one goes back. Then `w2m_exception` asks `THREAD_STACK_GROW` (`HOST_THREAD`, runtime.h 341), which for
  the main thread, whose stack the arena does not hold, answers 0 (`_stack_grow` 3161-3170). Both are served; a created
  thread's stack stops by name. Per fault the host then served two `HOST_MODULE_AT` and one `HOST_EXIT`;
  `EXCEPTION_STUBS` once per VM, at the first fault.
- `crates/w2m_vm/src/guest_stubs.rs` (`guest_stubs.py`), `host_request.rs` (`containing`, `serve`); `Guest` gains
  the arena, `import_thunk`, `code_page`, `guest_stubs`, `exception` and `clear_exception`; `run_cases` serves
  `hvc #8` and `hvc #14` until the process ends, and `judge_fault` judges the records.
- Expected values (`tests/case_runner.rs`, the 22 Intel runs): 173,776 cases judged with zero difference classes:
  the 148,614 completed cases of Sprint 145 and 25,162 faults, on exception code, RIP, ExceptionAddress, the GPRs,
  EFlags under the mask, MXCSR, XMM0-15 and the three windows. Of 33,168 cases Windows ended in a fault, the other
  8,006 ran in the model (`hvc #10` or `#11`) and are counted, not judged. 66 s in release.
- Carried: `guest_stubs::page(0x7ffe00001230)` equals the frozen Python's 512 bytes (`tests/guest_stubs.rs`).
- Planted fault: the CONTEXT's RIP read from +240. Set 039 showed 168 difference classes, the first "ADD fault rip:
  case 4: vm 0xdafa, cpu 0x20000000". Restored, the test passes.
