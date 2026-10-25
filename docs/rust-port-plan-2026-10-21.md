# w2m in Rust: the port plan

**History from 2026-10-24.** The order of work is `docs/rust-port-plan-2026-10-24.md` (BLACKBOARD Decision 2026-10-24
user: this plan reached no frame, and its design kept the big lock).

**Date:** 2026-10-21. **Status:** draft, revised after review the same day; revised 2026-10-23 so that no step runs
Python. **Decision:** all of w2m is rewritten in Rust, and Rust only (user, 2026-10-21; BLACKBOARD). No step of the
port runs Python beside Rust, and no new Python is written (user, 2026-10-23).

## Principle: keep the behavior, change the shape

Practice for moving code to a new language:

- **Component by component, each judged on its own.** fish moved 57,000 lines of C++ to 75,000 lines of Rust
  "component by component until no C++ was left", its end-to-end tests guarding the port (fish blog, "Fish 4.0: The
  Fish Of Theseus"). Fowler's strangler fig keeps old and new running together through a transitional architecture
  ("Strangler Fig Application"); w2m does not take that part, by the user's decision of 2026-10-23: no Rust component
  calls Python. What w2m keeps of it is the guard against a big-bang release ("Patterns of Legacy Displacement"):
  each component is judged alone, against data, before the next is built on it.
- **What judged the old system judges the new code.** typescript-go carries the TypeScript repository as a git
  submodule (`_submodules/TypeScript`, branch `tsgo-port`, from its `.gitmodules`) and runs its tests against the port.
  w2m's old system is frozen at `python-final`. What judged it is data and judges the port: the Xeon's results for
  each instruction class, the Windows runs, the stored verdicts (Sprint 123), and the values its tests assert. No Rust
  component calls Python, and no Python is written to extract anything from the old code.

typescript-go also kept tsc's structure (Microsoft, "A 10x Faster TypeScript"). It could, because that structure was
sound and Go fit it. w2m's is not:

- the `VM` class is 3,040 lines (src/w2m/vm_backend.py:2328 to 5367) and holds the compiler, memory, services,
  scheduling, threads and diagnostics;
- the host for several vCPUs follows a big-kernel-lock design on purpose: one lock held by whoever serves
  (docs/design-several-vcpus-2026-10-17.md; Sprint 113), plus kernel.Machine's RLock over every shared table.

Ported as it stands, that design gives native threads that still queue behind one mutex. So the port keeps the
behavior and changes the shape. Spolsky's warning that a rewrite loses "all those collected bug fixes" ("Things You
Should Never Do, Part I") is met by the judges, not by the code's shape. In w2m, the fixes that matter are fixed in
the judges and the test guests:
- the Windows game runs: GitHub run 38216081440 and the laptop's ten;
- the Xeon's results for 458 instruction classes;
- the 68 families;
- the batch run's call comparison;
- the guests in tests/data, such as Sprint 122's handshake, busychild, crosswake, pingpong and statsize.

All of that is data and survives the port. A changed behavior shows up as a failed judge.

## The new shape

Written as a design (docs/design-rust-host-2026-10-23.md, Sprint 124) before any host code. It states which state each vCPU owns, which
state is shared, and the lock for each shared part, with sources.

- **No global lock.** Each vCPU is a native thread that creates its vCPU and serves that vCPU's exits itself.
  Apple's hv_vcpu.h requires the first part: hv_vcpu_create "Creates a vCPU instance for the current thread", and
  "Each thread can only have one vCPU associated at a time". Firecracker works the same way: "one or more vCPU threads
  (one per guest CPU core)", which "execute synchronous I/O and memory-mapped I/O operations on devices models"
  (firecracker docs/design.md).
- **A lock per shared part, held only for the access.** Files, handles, processes, the code cache, the clock. crosvm:
  "Each `BusDevice` is reference counted and wrapped in a mutex, so implementations of `BusDevice` need not worry about
  synchronizing their access across multiple VCPUs and threads" (crosvm book, architecture overview).
- **One struct per layer:** cpu, memory, nt, host, diagnostics, the five-layer split plan-2026-10-09.md step 3 already set. No
  Rust struct holds all of it.

## Seams, measured on f2d071af

| Seam | Today | Lines |
|---|---|---|
| S1 vCPU loop and exit serving | tools/vm_monitor/vm_monitor.c; the serving parts of `VM` (vm_backend.py), kernel.py, scheduler.py, file_bridge.py, virtual_memory.py | 544 C; within `VM`'s 3,040, plus 470 + 211 + 368 + 352 Python |
| S2 loading | pe.py, loader.py, modules.py, apiset.py | 352 + 307 + 190 + 66 = 915 |
| S3 the host's translator | `compile_insn` (vm_backend.py:1417) and the block assembler; lift.py, x86dec.py, x86dec_tables.py, x86ops.py, a64_encoder.py; the rules (src/w2m/rules/) | 355 + 287 + 659 + 167 + 926 + 4,135 |
| S4 the model and slow paths (hvc 10, 11) | runtime.py, x87.py, x86fp.py | 1,191 + 696 + 560 |
| S5 guest runtime, inside the VM | tools/guest_runtime | 9,704 C |
| S6 compiler inside the VM | tools/vm_compiler | 1,499 C + generated tables |
| S7 judges, records, generators | scripts/ (64 files), events.py, contract.py, compare.py | 18,045 in scripts |

The rules are the only statement of what each x86 instruction means, and 456 of the 458 classes are judged on them.
The host's translator also compiles every block the in-VM table lacks, so it sits on the running game's path.

## Order

No step runs Python beside Rust. Each component is judged alone against data, and the end-to-end gates (the batch
run, the class sweep, the families) come when the host can run a program.

0. **Freeze the oracle.** Tag f2d071af `python-final`. Store the current verdicts as golden records. The two
   instruction-clock batch runs of Sprint 122 whose trace hashes differed (1d9c4e4b… at f1838b96, 9b1834ee… at 4b36b5ad,
   whose code is f2d071af's) did the same work: the
   traces differ only in line 1, GUEST_TRACE_OPENED's `execution_id`, and lines 2 to 10,535 hash to f927ac28… in both
   (2026-10-22). So a golden trace is compared by its body, lines 2 on, never by `trace_sha256`, which names the run.
   (Done: Sprint 123.)
1. **The design**, before any host code. (Done: Sprint 124, `docs/design-rust-host-2026-10-23.md`.)
2. **The judges in Rust (S7), before the code they judge.** The workspace, the vocabulary's generated types, and a
   Rust version of each judge a later step needs: the golden check (`scripts/golden.py` today), the class judge, the
   family judge, the batch run's call comparison, and the stencil judges. Gate: each gives the stored verdict on every
   stored record (530 golden keys). Substrate stays the harness under them: the Rust judges write records substrate
   reads and replays (user, 2026-10-22). From here no Python judge decides a gate.
3. **The tests' expected values, carried over.** Each Python test of a component asserts values (a PE header's
   fields, a decoded instruction's length and operands, an encoded arm64 word, a model's result); those values become
   the expectations of Rust tests, read from the Python test files by hand and cited by file and line. The tests that
   need a running host wait for step 7.
4. **Loading (S2) in Rust.** Gate: its carried tests.
5. **The translator (S3) in Rust.** Gate: its carried tests and the decode spec (`specs/x86_decode.json`), then the
   class judge against the Xeon's results once step 7 runs code.
6. **The model and slow paths (S4) in Rust.** Gate: its carried tests, then the Xeon's results at step 7.
7. **The Rust host on one vCPU (S1)**, by the design: a `w2m-vm` process per guest process, the supervisor, every exit
   kind served in Rust. Gate: the batch run conforms (168 lines, 0 divergences, its exit kinds), 456 of 458 classes,
   68 of 68 families, and the guests in tests/data with their stored output. The Python host's code moves under
   `oracle/` (git mv) and leaves CI.
8. **Several vCPUs on the Rust host.** The instruction clock is refused on several vCPUs
   (docs/design-several-vcpus-2026-10-17.md), so the gate is its own:
   - the litmus tests MP, SB, LB, 2+2W and IRIW;
   - the 68 families on two vCPUs;
   - the batch run on the wall clock;
   - instructions a second against one vCPU, on a workload that is not mostly waiting;
   - Sprint 122's guests (handshake, busychild, crosswake, pingpong, statsize) ending on two vCPUs in many runs, the
     count in the card: crosswake, quarantined in the Python host after reaching its limit once (BLACKBOARD Decision
     2026-10-23), is fixed here.
   The design takes two faults measured in the Python host on 2026-10-22 as requirements: a host thread that serves
   one vCPU never slows another's (a child's start there took 3.4 to 6.8 s against 0.25 s, its file reads waiting on
   the GIL behind vCPU 1's 90,000 exits a second), and every VM is closed and leaves no file when its process ends (a
   child's never was: 1,846 files, 88 GB).
9. **Guest runtime (S5) in Rust**, `no_std` for `aarch64-unknown-none`, file by file, C and Rust objects linked into
   one guest image while both exist. Gate: the step 7 and 8 gates.
10. **Compiler in the VM (S6) in Rust**, `no_std`, with its table generators. Gate: the Rust stencil judges.

Steps 4 to 6 need no running host and can go in any order. Step 7 is the first end-to-end run, and the step where a
fault between components shows: the price of building with no forward into Python, met at one step instead of spread
over six.

## Set-up

- **Toolchain:** the Mac's rustup install, stable rustc 1.99.1 (2026-10-13), pinned in `rust-toolchain.toml`.
  `aarch64-unknown-none` is added at step 9.
- **Workspace:** one Cargo workspace at the repository root, a crate per layer, `Cargo.lock` committed.
- **Hypervisor.framework bindings:** written from Apple's headers, for the 26 functions vm_monitor.c calls:
  - the VM and its configuration;
  - vCPUs, their registers and the vtimer mask;
  - `hv_vcpus_exit`, for the kick;
  - nine `hv_gic_*` calls for the GICv3.
- **The vocabulary in Rust:** Rust code emits only through types generated from the version `signals/current.json`
  names, by a generator written in Rust, with a `--check` mode and an emit-site parity check in the gates, both in
  place before the first Rust judge writes a record (BLACKBOARD Decision 2026-10-22).
- **Signing:** a cargo runner signs each binary that creates a VM with `com.apple.security.hypervisor`
  (tools/vm_monitor/entitlements.plist).
- **CI:** builds and tests the workspace from step 2; the Python gates leave CI at step 7, when the Python host
  moves under `oracle/`.

## Sources read

- fish, "Fish 4.0: The Fish Of Theseus": https://fishshell.com/blog/rustport/
- Fowler, "Strangler Fig Application": https://martinfowler.com/bliki/StranglerFigApplication.html
- Fowler et al., "Patterns of Legacy Displacement": https://martinfowler.com/articles/patterns-legacy-displacement/
- Microsoft, "A 10x Faster TypeScript": https://devblogs.microsoft.com/typescript/typescript-native-port/
- microsoft/typescript-go `.gitmodules`, read with `gh api` on 2026-10-21
- Spolsky, "Things You Should Never Do, Part I": https://www.joelonsoftware.com/2000/04/06/things-you-should-never-do-part-i/
- Firecracker, docs/design.md: https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md
- crosvm book, architecture overview: https://crosvm.dev/book/architecture/overview.html
- Apple, Hypervisor.framework headers hv_vcpu.h and hv_vm.h, in the installed SDK
