# Sprint 035 — the guest runtime: kernel32 in C inside the VM

```yaml
---
id: 035
status: closed
phase: 3
pass_kind: functional
---
```

## scope

The runtime architecture (BLACKBOARD 2026-10-09) runs Windows services inside the guest as freestanding C compiled to AArch64, with Windows-visible state in guest memory at Windows layouts (`specs/guest_state.json`, Sprint 034). Today every kernel32 call leaves the VM and runs in Python (`kernel32.py`). This sprint moves the probes' kernel32 surface into the guest:

1. `tools/guest_runtime/runtime.c` (+ `runtime.h`): freestanding C (`clang --target=arm64-apple-macos14 -ffreestanding -fno-builtin -mgeneral-regs-only`, linked by Xcode's `ld -preload` at a fixed VA; checked to work 2026-10-09: PC-relative globals, __DATA after __TEXT). It holds one thread's state block, its TEB and the PEB in guest memory at the spec's offsets, and implements GetCurrentThreadId (TEB ClientId.UniqueThread, 0x48), SetLastError and GetLastError (TEB LastErrorValue, 0x68 — where Windows keeps it, Sprint 034), GetStdHandle, WriteFile, ExitProcess and QueryPerformanceCounter. Host effects (the bytes WriteFile writes, process exit, the performance counter) go through one typed transport: a request record in a shared page, then `hvc #8`; the host services it and resumes the vCPU.
2. `src/w2m/guest_runtime.py`: builds the image, reads its Mach-O segments and symbols, and places it.
3. `src/w2m/vm_backend.py`: maps the runtime image, the state block, TEB and PEB; resolves each import to an AArch64 bridge stub (Microsoft x64 arguments RCX, RDX, R8, R9 and the stack → AArch64 x0-x4; the guest GPRs saved in the state block around the call; the result in RAX; RET's pop of the return address) instead of an exit to Python; services `hvc #8`; reads a ring buffer the runtime appends one record per service call to, so `Result.calls` comes from the guest. The model runtime (`runtime.py` with `kernel32.py`) stays the lockstep reference.
4. Translation on the record: each compiled block emits CODE_TARGET_DISCOVERED (discovery `runtime_transfer` or `static` for the entry, stage `execution`) and TRANSLATION_PUBLISHED (guest range, code version 0) in a `compilation` session `run_skeleton.py` writes per VM run. TRANSLATION_INVALIDATED moves to the Mono JIT-region sprint: nothing modifies or unmaps code before it, and its `cause` enum has no honest value here (amends the BLACKBOARD entry assigning it to 035).
5. Candidates in C: `candidate.py` admits a C body for SetLastError/GetLastError (the declared signature, no inline assembly, no calls, no pointer arithmetic beyond the TEB accessor), compiles it into the runtime image, and the candidate loop runs it in the VM; the Python kernel32 path for candidates ends.

## prerequisites

- 034

## context_files

- BLACKBOARD Decision 2026-10-09 (runtime architecture); `specs/guest_state.json`; Microsoft Learn: x64 calling convention ("x64 calling convention", parameter passing: RCX, RDX, R8, R9, then the stack above 32 bytes of shadow space; RAX return; RBX, RBP, RDI, RSI, RSP, R12-R15 nonvolatile); Arm AAPCS64 (x0-x7 arguments, x19-x28 callee-saved, x18 platform register)
- `src/w2m/vm_backend.py`, `tools/vm_monitor/vm_monitor.c`, `src/w2m/kernel32.py`, `src/w2m/candidate.py`, `scripts/run_candidate_loop.py`, `scripts/run_skeleton.py`

## signal contract

### Emits

- harness plane, new: CODE_TARGET_DISCOVERED and TRANSLATION_PUBLISHED (first emitters of either tag) in a `compilation` session per VM run.
- guest plane: unchanged — the probes write their own traces; the 0.1.0 and 0.2.0 last-error contracts judge them unchanged.

### Invariants

- No vocabulary change.
- One TRANSLATION_PUBLISHED per compiled block; every CODE_TARGET_DISCOVERED precedes the TRANSLATION_PUBLISHED whose range starts at its address.

## artifact contract

### Files created / modified

- created: `tools/guest_runtime/runtime.c`, `tools/guest_runtime/runtime.h`, `src/w2m/guest_runtime.py`, `tests/test_guest_runtime.py`, this card
- modified: `src/w2m/vm_backend.py`, `src/w2m/candidate.py`, `scripts/run_candidate_loop.py`, `scripts/run_skeleton.py`, tests that drive the VM

### Command exit codes

- `scripts/gates.sh` → 0

## observation contract

### Expected runtime signals

- Skeleton and last_error_values in the VM with the guest runtime: exit 0; their traces conform 3 of 3 under 0.1.0 and 0.2.0; lockstep and the state differential against the model report 0 differences (lockstep compares after every x86 instruction and after every kernel32 call; a call now runs as AArch64 in the guest).
- No VM run calls into `kernel32.py`; the call list in `Result.calls` comes from the guest's ring buffer and equals the model's.
- After SetLastError(v), guest memory at TEB+0x68 holds v (read by the host after the run).
- Translation record per VM run: one CODE_TARGET_DISCOVERED and one TRANSLATION_PUBLISHED per compiled block.
- Candidate loop with `qwen2.5-coder:7b` writing C: whatever the record shows; a planted C constant is rejected under 0.2.0.

## done criteria

The probes' kernel32 calls run as C in the guest against guest-memory TEB state, the existing contracts judge them unchanged, and the translator's work is on the signal plane.

## self-review checklist

- [x] Scope concrete. [x] Sources named. [x] No new vocabulary. [x] Observation declared before running. [x] File count recorded (about 12; one concept: kernel32 moves into the guest, which the candidate path and the lockstep reference both depend on).

## observed
- Toolchain checked before writing code: there is no ELF linker (`ld.lld`) or `llvm-objcopy` on this host; Xcode's clang plus `ld -arch arm64 -preload -segaddr __TEXT <va>` links freestanding AArch64 C into an MH_PRELOAD Mach-O at a fixed address with PC-relative globals, which `src/w2m/guest_runtime.py` reads (segments and symbols) and copies into guest memory. The runtime image is one 4 KB text page.
- The encoder gained `stp` and `blr` (and `mov sp, xN` as `add sp, xN, #0`), each checked against clang in `tests/test_a64_encoder.py`.
- Both probes in the VM with kernel32 as guest C: exit 0; stdout byte-identical to the model's; the call list read from the guest ring equals the model's; after the skeleton, TEB+0x68 holds 1234 and TEB+0x48 holds the thread id 4. No VM run calls `kernel32.py`.
- Lockstep and state differential, unchanged in count and clean: skeleton 5,159 comparisons and 8,680 cases over 217 instructions; last_error_values 11,931 comparisons and 16,400 cases over 410 instructions; 0 differences each. A kernel32 call is one lockstep step on both sides: the bridge exits to the dispatch miss with the return address.
- `run_skeleton.py` under 0.1.0 (skeleton) and 0.2.0 (last_error_values): Windows reference, model and VM 3 of 3 conform each. VM time per run, measured directly: construction 6.9-7.1 ms plus run 15.5-17.1 ms, the Sprint 032 range (21-24 ms); the script's own timer first showed 31-44 ms because the translation record was written inside it (fixed: recorded after timing).
- Translation record per VM run (`store/runs/translations-1791579909159674000`): SESSION_INIT (purpose compilation), CODE_TARGET_DISCOVERED 62, TRANSLATION_PUBLISHED 62, SESSION_COMPLETED; the entry point is `static`, every other block `runtime_transfer`; first emitters of both tags. TRANSLATION_INVALIDATED moves to the Mono JIT-region sprint (no code is modified or unmapped before it).
- Candidates are C (`src/w2m/candidate.py`): the declared signatures replace Sprint 032's parameter-count rule; admission refuses preprocessor lines, assembly, loops, goto, literals, subscripts, pointer syntax (including a pointer through a cast), division and remainder, calls other than `w2m_teb()`, and self-calls, then compiles with `-Wall -Werror`; the compiler's message is the refusal. The model backend runs a host build of the same C behind kernel32.py (ctypes), so the loop's tests still run on Linux CI; the VM compiles it into the guest runtime image.
- Real loop, `qwen2.5-coder:7b` writing C, VM: `store/runs/candidates-1791579927493057000` — accepted on attempt 1, 3 of 3 conform (494 prompt and 57 completion tokens; 14.9 s, the model's load included); its code is `w2m_teb()->last_error_value = code;` and `return w2m_teb()->last_error_value;`. `store/runs/candidates-1791579948983398000` — the planted C constant (`probes/last_error_values/candidates/returns_constant.c`) rejected 3 of 3 ("returned 1234, which is not the argument of kernel32!SetLastError"), reported, routed implementation_repair; the model's repair accepted and the discrepancy resolved.
- Store hygiene found by a fixture: re-judging a runner body under a new `--prefix` stored second Windows reference executions of the same runs (the fixture's count went from 3 to 6). Fixed: `run_skeleton.py` reuses the stored execution id of an identical body. Also found: `--run-id` is written as each reference execution's `github_run_id`, so re-judging must pass the real GitHub run id; three records overwritten in this sprint were restored from git.
- Gates: `scripts/gates.sh` 0.

