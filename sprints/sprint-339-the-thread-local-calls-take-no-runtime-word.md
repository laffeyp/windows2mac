# Sprint 339 — The thread-local calls and QueryPerformanceCounter take no runtime word

```yaml
---
id: 339
status: closed-partial
phase: 6
pass_kind: functional
---
```

## scope

Change 1 of the host design: "a runtime call takes the lock of what it touches, or none"
(`docs/design-rust-host-2026-10-24.md` 95-140). Its first lock class is **None**, "for what one thread owns or what
no one writes". The design names those calls, which include the three hottest of the game's import mix:

- GetCurrentThreadId, GetCurrentProcessId;
- TlsGetValue, TlsSetValue, FlsGetValue, FlsSetValue;
- GetLastError, SetLastError;
- QueryPerformanceCounter, which reads `CNTVCT_EL0`.

Today every bridge takes the runtime word around its call into the runtime's C: `guest::bridge` in
`crates/w2m_vm/src/guest.rs` (545-603) does `take_runtime`, `blr`, `release_runtime`. So one word serializes every
kernel32 call on every vCPU. Sprint 338 put the host's own take behind a guard; this card changes the guest's bridge
for the None class.

- **The work:**
  - **The lock class.** Each service in the runtime's service table (`crates/guest_runtime/src/services.rs`,
    `Service`) carries its lock class. This card gives it two values, the runtime word, which every service takes
    today, and none. Only the services named above are set to none. Change 1's other classes (a heap's lock, the TLS
    slots' lock, the handle table's, a sync object's, the modules') stay on the word until their own cards.
  - **The bridge.** For a service of class none, `guest::bridge` emits no `take_runtime` and no `release_runtime`.
    Everything else stays as it is, the budget's count and the register save and restore included. A traced call
    (`traced` is `Some`, through `w2m_traced`) still takes the word, because `w2m_traced` writes the process-wide trace
    area. The design moves that to an atomic sequence number in its own later card.
  - **The proof, before any change.** For each of the nine services, read every store the bridge's path makes between
    its take and its release, and every store the service's C makes (`tools/guest_runtime/thread_storage.c`,
    `process.c`, `runtime.c`). Show each one lands in the calling thread's own state (its TEB, its TLS or FLS slots, its
    last-error) or in the calling vCPU's own pages (its state block, its runtime stack; Sprint 328). Quote the lines.
    A store to shared state keeps that service on the word, with the reason recorded. Any shared read the service makes
    is named too, with why it needs no lock.
- **The measurement.** `run_measurements`' `--runtime-word` run of `qpc` takes the word at the `bridge` site once per
  served bridge call (`Site::Bridge`). Before and after, quote its count of `bridge` takings and its
  `runtime_word_hold_ns` rows. After, the takings fall by the number of QueryPerformanceCounter calls `qpc` makes. A
  new record, `store/runs/w2m-vm-measurements-<ns>`, holds the after figures, and `tests/measurements_record.rs` keeps
  passing on it.
- **Not here:**
  - The other lock classes.
  - The trace's atomic sequence.
  - G6, the game's hot-import mix from six threads, which measures the whole of Change 1 once its classes are built.

## prerequisites

- 338 (closed)

## context_files

- `docs/design-rust-host-2026-10-24.md` 95-140, Change 1 whole, its lock classes and order.
- `crates/w2m_vm/src/guest.rs`: `bridge` (545-603), `service_for` (199-215), and its two callers (917, 1346).
- `crates/w2m_vm/src/monitor_code.rs`: `take_runtime` (283), `release_runtime`, `lock_runtime` (116),
  `unlock_runtime` (219), `Site` (243-263).
- `crates/guest_runtime/src/services.rs` (`Service` and the table), and the C of the nine services in
  `tools/guest_runtime/` (`thread_storage.c`, `process.c`, `runtime.c`; find each by `grep -n`).
- `crates/w2m_vm/src/bin/run_measurements.rs` and `crates/w2m_vm/tests/measurements_record.rs` (Sprint 337), and
  `harness/w2m-vm-measurements.toml`.
- `crates/w2m_vm/tests/run_loop.rs` 1015-1250, the two-vCPU tests that must keep passing.
- The Decision: `grep -n "Step 6 restored to the user's fix (b)" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands", with "A sprint closes in its work commit"
  and "no gate row is dropped".

## signal contract

### Emits

- The new measurements record: MEASUREMENT_RECORDED `runtime_word_hold_ns_p50` and `runtime_word_hold_ns_p99` for
  `qpc` as before, their `variant` naming the takings, and the rest of Sprint 337's record unchanged in form.

### Invariants

- Every test passes as before: the two-vCPU tests, the batch run, the traced (`--call-trace`) runs and the family runs.
- A traced call takes the word as before.

## artifact contract

### Files modified

- `crates/guest_runtime/src/services.rs` (the lock class) and `crates/w2m_vm/src/guest.rs` (the bridge); a test.

### Files created

- `store/runs/w2m-vm-measurements-<ns>/`

### Command exit codes

- `scripts/gates.sh` returns 0
- `uv run substrate replay <the new record> --level 2` returns 0, and `check_records` over it prints no violation
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The nine services' proof quoted in the result, line by line.
  - `qpc`'s `bridge` takings before and after, and their difference against `qpc`'s QueryPerformanceCounter calls.
  - A test that fails if a service of class none takes the word. For example: a text test over the service table
    naming the nine, and a bridge test that assembles a none-class bridge and finds no `lock_runtime` in its code.
- **Planted faults:**
  - (1) One service outside the nine, CreateEventW, set to none. The test, holding the class set to the nine, fails
    by name.
  - (2) The bridge emitting `take_runtime` for a none-class service. The bridge test fails, and `qpc`'s takings return
    to the before count.

  Before the work, read each judge's reach: the bridge test assembles the very path the plant changes.

## done criteria

The nine thread-local services and QueryPerformanceCounter take no runtime word, each with a written proof that it
touches only its thread's or its vCPU's own state. `qpc`'s word takings fall by its QueryPerformanceCounter calls, on
a record, and two plants fail by name.

## self-review checklist

- [x] Scope names one verifiable property: the None class's services take no word, on a record.
- [x] context_files cite Change 1, the bridge, the word's code, the service table, the C, the measurement and the tests.
- [x] Emits: tags and values in 0.13 only.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the proof, the counts, the test; two plants, their reach read.
- [x] Sweet spot: `services.rs` and `guest.rs` plus a test and a record; one concept, the None class.

## result

Not done: the proof, run before any change, fails for all nine services, so by the card's rule ("A store to shared
state keeps that service on the word, with the reason recorded") every one stays on the word. No source file was
changed, no plant was run and no record was made; the gates ran on the tree with this result as its only change.

- **The bridge's own path passes.** Between `take_runtime` and `release_runtime` (`guest.rs` 577-581) the path is
  `mov_imm(16, target)`, `blr(16)` and `budget_from_count`, `ldp x16, x17, [x28]` (`monitor_code.rs` 353): registers
  and one load, no store. x28 is `MONITOR_VA + MONITOR_DATA` (`run.rs` 1256), the vCPU's own monitor page
  (`lib.rs` 108). Around it every store lands in the vCPU's own pages: the GPRs' `stp` into `PROCESS_VA + STATE_OFF`
  (`guest.rs` 558-561) and RAX's `str` (584), `count_from_budget`'s `str x17, [x28]` (`monitor_code.rs` 347),
  `str xzr` to `STATE_REDIRECT` (`guest.rs` 597); SP is the vCPU's runtime stack (564-565). The private pages are the
  monitor data page, the state block and the runtime stack only (`VcpuPages::private`, `lib.rs` 103-111;
  `Guest::private_chain`, `guest.rs` 1202-1206).
- **Every service's C fails at one store: `w2m_logged`** (`runtime.c` 60-70), which each of the nine returns through
  (`runtime.c` 74, 80, 85; `process.c` 56, 96; `thread_storage.c` 93, 97, 104, 109, 139, 142, 164, 167). It writes the
  ring at `W2M_RING_VA`, `PROCESS_VA + RING_OFF` (`guest_layout` 100, 159), which every vCPU maps alike:
  `r->per_api[api]++` (62), `r->rec[r->count].api/.arg0/.ret = ...` (64-66), `r->count++` (68). The built image
  (`build/guest_runtime_rust/22a9eaf0...bin`, `_w2m_logged` at 0x7f0004182050) does each as a plain load, add, store
  (`ldr x10, [x9, #0x10]; add; str x10, [x9, #0x10]` at ...064-06c), and reloads `count` before each field's store
  (`ldr x9, [x8]` at ...090, ...09c, ...0a8). On two vCPUs, increments are lost, and a record checked at index 10750
  can be written at 10751, the capacity ((0x40000 - 0x1010) / 24): its `.arg0` and `.ret` land at
  `PROCESS_VA + 0x45000` and `+0x45008`, the exception area (`EXCEPTION_OFF = RING_OFF + RING_SIZE`).
- **The rest of each service** (stores to the calling thread's or vCPU's own state unless named):
  - GetCurrentThreadId, GetCurrentProcessId: read the TEB (`w2m_teb()`, the state block's +0xb0, `runtime.h` 39),
    `client_thread` (`runtime.c` 74), `client_process` (`process.c` 96). No store but the ring.
  - SetLastError, GetLastError: `w2m_teb()->last_error_value = code` and its read (`last_error.c` 8, 10).
  - TlsGetValue, TlsSetValue: `set_error`, the TEB's +0x68 (`thread_storage.c` 58); the slot, the TEB's +0x1480 or
    the array its +0x1780 names (60-65): a created thread's own, `teb + 0x2000` (`threads.rs` 1249), the main
    thread's `W2M_TLS_EXPANSION_VA` (78-79). `*slot = value` (108).
  - FlsGetValue: reads the thread's block (`fls_mine(0)`, 43-55); shared read: `b->value[index]`, which FlsAlloc and
    FlsFree on any thread write under the word (126, 177).
  - FlsSetValue: a second shared store. The thread's first set takes heap (`w2m_heap_take`, 46, the process heap)
    and links the block into the process-wide list (`b->next = fls_blocks; fls_blocks = b;`, 50-51). Shared read:
    `fls_used[index]` (162).
  - QueryPerformanceCounter: `*(u64 *)out = counter_now()` (`process.c` 55), the caller's own memory. Clock mode 0
    reads `cntvct_el0` and `cntfrq_el0`. Mode 2 reads the vCPU's monitor word 0 and, shared, `W2M_CLOCK_WARP_VA`
    (45). Mode 1 is a third shared store: `w2m_host` writes the transport page, `PROCESS_VA + TRANSPORT_OFF`, and
    makes `hvc #8` (`runtime.c` 26-35).
- **Before figures** (the latest record, `store/runs/w2m-vm-measurements-1795656377000000000`): `qpc`'s `bridge` rows
  `runtime_word_hold_ns_p50` 3417 and `_p99` 4625 (`variant` "bridge; 2 takings"), 5375 and 5375 across an exit
  ("1 takings"), the wait rows over "3 takings".
- **What unblocks it.** `w2m_logged`'s three updates made safe for concurrent callers. `__atomic_fetch_add` compiles to
  an inline `ldadd` under the runtime's CFLAGS (probe: `clang --target=arm64-apple-macos14 -O2 -ffreestanding
  -fno-builtin -mgeneral-regs-only`, `ldadd x8, x0, [x0]`, no call). Reserving the index once and writing the record
  at that index closes the overrun. FlsSetValue's first set and QueryPerformanceCounter's clock mode 1 need their own
  answer or stay on the word. No Rust code reads the ring: `RING_OFF` appears only where `process.rs` 395-396
  initializes it.
- **Plant 1** reached no judge: the lock class it plants into the service table was not built, the proof stopping the
  sprint before any change. Its replacement is the judge that stopped it, the built `_w2m_logged`'s unlocked update,
  "ldr x10, [x9, #0x10]; add; str x10, [x9, #0x10]", which keeps CreateEventW and all nine on the word.
- **Plant 2** reached no judge: no none-class bridge exists to plant a `take_runtime` into. Its replacement is the
  same judge, `w2m_logged`'s reload of `count` before each field's store, "ldr x9, [x8]" at ...090, ...09c and ...0a8.
- **The gates** (the tree with this result as its only change, copied from `tail -1 store/gates/passes.tsv`):
  `exit 0 in 196 s	default	2026-11-26T02:20:43Z	sprint 339	6db5fcdfd7a917e73259733a2620985a36c50ea8	4f4af8b7370a80f502f99647c0a7944dcd089be7d67459b8a4c25f241660ce63	root=9fffa122ddd06efad951901c17490401dc20044f	rust=crates python=0 samples=0	base=cc2c67578a6b088d0595140e34127b4aef2b5bd9`
