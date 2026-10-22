# w2m in Rust: the port plan

**Date:** 2026-10-21. **Status:** draft, revised after review the same day. **Decision:** all of w2m is rewritten in
Rust, and Rust only (user, 2026-10-21; BLACKBOARD).

## Principle: keep the behavior, change the shape

Practice for moving code to a new language:

- **A working system at every step.** fish moved 57,000 lines of C++ to 75,000 lines of Rust "component by
  component until no C++ was left", with a working shell throughout and its end-to-end tests guarding the port:
  first PR 2023-01-28, last C++ removed 2024-01 (fish blog, "Fish 4.0: The Fish Of Theseus").
- **Strangler fig, not big bang.** Replace parts at their seams, with a transitional architecture that lets old and
  new run together and is removed at the end (Fowler, "Strangler Fig Application"). Holding out for full feature
  parity before any cut-over leads to a single big-bang release ("Patterns of Legacy Displacement").
- **The old tests judge the new code.** typescript-go carries the TypeScript repository as a git submodule
  (`_submodules/TypeScript`, branch `tsgo-port`, from its `.gitmodules`) and runs its tests against the port.

typescript-go also kept tsc's structure (Microsoft, "A 10x Faster TypeScript"). It could, because that structure was
sound and Go fit it. w2m's is not:

- the `VM` class is 3,040 lines (src/w2m/vm_backend.py:2328 to 5367) and holds the compiler, memory, services,
  scheduling, threads and diagnostics;
- the host for several vCPUs follows a big-kernel-lock design on purpose: one lock held by whoever serves
  (docs/design-several-vcpus.md; Sprint 113), plus kernel.Machine's RLock over every shared table.

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

Written as a design (docs/design-rust-host.md) before step 2's code. It states which state each vCPU owns, which
state is shared, and the lock for each shared part, with sources.

- **No global lock.** Each vCPU is a native thread that creates its vCPU and serves that vCPU's exits itself.
  Apple's hv_vcpu.h requires the first part: hv_vcpu_create "Creates a vCPU instance for the current thread", and
  "Each thread can only have one vCPU associated at a time". Firecracker works the same way: "one or more vCPU threads
  (one per guest CPU core)", which "execute synchronous I/O and memory-mapped I/O operations on devices models"
  (firecracker docs/design.md).
- **A lock per shared part, held only for the access.** Files, handles, processes, the code cache, the clock. crosvm:
  "Each `BusDevice` is reference counted and wrapped in a mutex, so implementations of `BusDevice` need not worry about
  synchronizing their access across multiple VCPUs and threads" (crosvm book, architecture overview).
- **One struct per layer:** cpu, memory, nt, host, diagnostics, the five-layer split plan.md step 3 already set. No
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

0. **Freeze the oracle.** Tag f2d071af `python-final`. Store the current verdicts as golden records. The two
   instruction-clock batch runs of Sprint 122 whose trace hashes differed (1d9c4e4b… at f1838b96, 9b1834ee… at 4b36b5ad,
   whose code is f2d071af's) did the same work: the
   traces differ only in line 1, GUEST_TRACE_OPENED's `execution_id`, and lines 2 to 10,535 hash to f927ac28… in both
   (2026-10-22). So a golden trace is compared by its body, lines 2 on, never by `trace_sha256`, which names the run.
1. **The design** (above), before any host code.
2. **A Rust host owns the VM**, on one vCPU, with a native thread for it. Transitional architecture: exit kinds not yet
   ported go to the Python host over the existing request protocol. Gate: the batch run conforms (168 lines, 0
   divergences, its exit kinds), with 456 of 458 classes and 68 of 68 families.
3. **Exit kinds move to Rust**, largest host share first, from the records: hvc 8 by request op (41,299 requests,
   4.69 s on the instruction clock), hvc 12 the scheduler's hand-overs, hvc 13 code writes, then 1, 4 and 14. Each one
   removes a forward. Same gate.
4. **The translator (S3) in Rust.** Gate: the same arm64 words as `compile_insn` under scripts/stencil_judge.py's four
   judges (instructions, blocks, cases, spec), which already compare a second compiler against the Python one; then
   the class sweep.
5. **Loading (S2) in Rust.**
6. **The model and slow paths (S4) in Rust.** The forward and the Python host are removed, and from here no Python
   runs during a game.
7. **Several vCPUs on the Rust host.** The instruction clock is refused on several vCPUs
   (docs/design-several-vcpus.md), so the gate is its own:
   - the litmus tests MP, SB, LB, 2+2W and IRIW;
   - the 68 families on two vCPUs;
   - the batch run on the wall clock;
   - instructions a second against one vCPU, on a workload that is not mostly waiting.
8. **Guest runtime (S5) in Rust**, `no_std` for `aarch64-unknown-none`, file by file, C and Rust objects linked into
   one guest image while both exist.
9. **Compiler in the VM (S6) in Rust**, `no_std`, with its table generators. Gate: stencil_judge.py.
10. **Judges and tools (S7) in Rust.** Each Rust judge gives the stored verdict on every stored record before its
    Python version retires. Substrate stays the harness under them: the Rust judges write records substrate reads
    and replays (user, 2026-10-22).

Steps 8 to 10 port working code that does not set the game's speed. They follow several vCPUs.

## Set-up

- **Toolchain:** the Mac's rustup install, stable rustc 1.99.1 (2026-10-13), pinned in `rust-toolchain.toml`.
  `aarch64-unknown-none` is added at step 8.
- **Workspace:** one Cargo workspace at the repository root, a crate per layer, `Cargo.lock` committed.
- **Hypervisor.framework bindings:** written from Apple's headers, for the 26 functions vm_monitor.c calls:
  - the VM and its configuration;
  - vCPUs, their registers and the vtimer mask;
  - `hv_vcpus_exit`, for the kick;
  - nine `hv_gic_*` calls for the GICv3.
- **The vocabulary in Rust:** Rust code emits only through types generated from the version `signals/current.json`
  names, with a `--check` mode and an emit-site parity check in `scripts/gates.sh`, both in place before step 2's host
  writes a record (BLACKBOARD Decision 2026-10-22).
- **Signing:** a cargo runner signs each binary that creates a VM with `com.apple.security.hypervisor`
  (tools/vm_monitor/entitlements.plist).
- **CI:** builds and tests the workspace beside the Python gates until step 10 ends them.

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
