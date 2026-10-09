# w2m project review — 2026-10-09

Scope: all work from Sprint 000 through Sprint 033 part 1 (commit d7a6f36), measured against `windows_to_macos_research_direction_v0_2.md` (the plan), `sdd-kit-3/AGENTS.md` (the method), and the binaries of the x86-64 application profile, Hail to the Rainbow (Steam app 1699230, build 24247624).

Previous review: `docs/review-2026-10-08-project.md` (at commit 34b85f0).

---

## Method

1. **Read in full:**
   - the kit, `sdd-kit-3/` (all text files);
   - the plan, v0.2;
   - `BLACKBOARD.md`, `WORKING_AGREEMENT.md` and `KIT_DIARY.md`;
   - the Sprint 033 card and every Sprint 033 artifact;
   - `lift.py`, `runtime.py`, `kernel32.py`, `loader.py`, and the compiler half of `vm_backend.py`.
2. **Ran the gates:** `scripts/gates.sh` exits 0 with 202 tests passed, ruff and mypy clean, `check_vocab.py` reporting 61 tags with 101 of 101 foreign keys resolved, and both generators current.
3. **Re-ran recorded claims from committed data.**
   - The judge verdicts on stored runner executions: event contract 0.2.0 conforms 10/10; the same executions are inconclusive under 0.3.1; the witnessed executions conform 20/20 under 0.3.1; last-error 0.1.0 and 0.2.0 conform 3/3 each.
   - The VM back end's lockstep and state differentials on both runner-built probes: 0 differences in 5,159 and 11,931 compared steps.
   - The 548 of 809 XED row confirmations.

   Each matches its Built entry.
4. **Measured the game's own binaries** (`~/w2m-depots/hail-to-the-rainbow`) with w2m's PE reader, its Capstone traversal (`x86ref.walk`), its decoder, its lifter, and its model runtime.
5. **Read primary sources on Unity's Mono runtime and its Windows behavior:**
   - the Unity-Technologies/mono `unity-2019.4-mbe` source;
   - upstream bdwgc;
   - box64 and FEX source and release notes;
   - Valve, Unity, Apple and Microsoft documentation.

   Each claim below cites its source and says what kind of source it is.

An independent reviewer pass (the project's `reviewer` agent) ran in parallel. Its findings are merged here only where this review reproduced them.

---

## 1. Verdict

The project is run honestly and its evidence holds up.

- Every recorded number checked above reproduces from committed data.
- Records are corrected by appended entries, not edits.
- The October 8 review's eight recommendations were carried out in Sprints 023–033: the decoder phase closed, the walking skeleton ran, the candidate loop ran, CI went in, and fuzzing went in.
- A Windows PE built on the runner now runs on this Mac in two back ends. One is a host model; the other compiles lifted code to AArch64 inside a Hypervisor.framework VM. The run is judged against a contract, and a local model has written and repaired a contract implementation inside a substrate run.

The problem is distance, and its kind. Every stage was built to the width of one probe: one thread, one image, 22 instruction classes, 7 kernel32 functions, no exceptions, no graphics. The chosen game needs a machine of a different shape, not a wider one:

- vector registers and full architectural state;
- that state reconstructed precisely at any instruction;
- fault-driven control flow;
- asynchronous thread suspension;
- run-time code generation with in-place patching;
- many images loaded at run time.

The next phase has to change the architecture before it adds rules, or each added rule lands in a structure that will be replaced.

---

## 2. Where the game is, measured

All figures come from the depot fetched 2026-10-09 (297 depot files and 15,584,319,636 bytes, plus one SteamCMD manifest).

### 2.1 Execution frontier

`Hail to the Rainbow.exe`, loaded by `loader.load` and run by `runtime.Runtime`, stops after 13 instructions:

> `Unimplemented: import KERNEL32.dll!GetSystemTimeAsFileTime`

The call comes from the MSVC C runtime's security-cookie setup, before `main`. The executable's only real work is a jump through the IAT to `UnityPlayer.dll!UnityMain`. The loader cannot follow that jump, because it maps one image and resolves every import to a host thunk.

### 2.2 Instruction coverage

The traversal is Capstone-recursive from the entry point, exports and `.pdata` starts (`x86ref.walk`). The instruction class is w2m's decoder's.

| File | Reachable positions | Positions w2m's decoder refuses | Instruction classes | Classes the lifter handles | Share of instructions in those classes |
|---|---|---|---|---|---|
| `Hail to the Rainbow.exe` | 10,580 | — | 109 | 21 | 93.5% |
| `mono-2.0-bdwgc.dll` | 606,946 | 53 | 184 | 22 | 96.8% |
| `UnityPlayer.dll` | 4,879,842 | 8,713 | 345 | 22 | 80.9% |

**The share column overstates progress.** A run stops at the first instruction the lifter has no rule for. MOV, CALL, Jcc and the ALU group make up most instructions by count, while the missing classes include the ones every function eventually reaches:

- shifts other than SHR, IMUL, DIV and CMOVcc;
- the LOCK-prefixed atomics (CMPXCHG, XADD, INC, DEC, OR) that every lock and reference count uses;
- the entire SSE set.

UnityPlayer's most frequent unliftable instructions are MOVAPS (207,567), MOVSS (128,992), MOVUPS (58,131) and MULSS (57,389).

**The decoder gap is new information.** The decoder phase closed on "0 spec gaps" (criterion b), measured on the Python 3.14.8 corpus. On UnityPlayer.dll w2m's decoder refuses 8,713 reachable positions, all common SSE:

| Instruction | Positions |
|---|---|
| CMPPS (`0f c2`) | 4,779 |
| MOVMSKPS | 1,442 |
| RSQRTPS | 432 |
| RCPPS | 424 |
| MOVNTPS | 309 |

The Python corpus was a sound fixture for integer code. It was the wrong fixture for a game engine.

(The independent reviewer counted 373 classes in UnityPlayer.dll and 209 in mono, using Capstone mnemonics; the table uses w2m iclasses. The instruction counts agree.)

### 2.3 Machine state

`kernel32.Thread` holds 16 general registers, RIP, five flags and a last-error value. The lifter's operation set (`lift.py` header) has no vector type. Nothing in the runtime or the VM back end models:

- XMM/YMM registers;
- MXCSR;
- x87 state and its control word;
- segment bases, the TEB and the PEB.

The VM back end's register map gives the integer registers to x0–x15 and the flags to x19–x24, and has no plan for XMM in the NEON registers.

### 2.4 Windows surface

The seven native PE files import 634 distinct functions from 21 DLLs (`w2m.pe.parse`):

| DLL | Functions |
|---|---|
| kernel32 | 294 |
| user32 | 120 |
| ws2_32 | 51 |
| advapi32 | 39 |
| winmm | 26 |
| other 16 DLLs | 104 |

`UnityPlayer.dll` alone imports 490 functions from 18 DLLs. It imports nothing from `d3d11`, `dxgi` or `ntdll`. Its strings show it loads `d3d11.dll`, `dxgi.dll` and the XInput DLLs by name at run time and resolves `D3D11CreateDevice` and `CreateDXGIFactory1` with `GetProcAddress`. The static import table therefore undercounts the surface.

`src/w2m/kernel32.py` implements seven functions, in Python, on the host. Two contract families exist (`windows.event.auto_reset.single_signal`, `windows.thread.last_error.set_get`).

### 2.5 What the game's runtime does that w2m must reproduce

These come from the `unity-2019.4-mbe` Mono source, read 2026-10-09 (source code). Paths are relative to `https://github.com/Unity-Technologies/mono/tree/unity-2019.4-mbe`.

1. **Faults are control flow.**
   - Mono installs a vectored exception handler (`mono/mini/mini-windows.c`, `exceptions-amd64.c`: `AddVectoredExceptionHandler(1, ...)`).
   - Null checks in JIT code are implicit by default (`mini.c`, `explicit_null_checks` off), so every managed null dereference is a hardware access violation.
   - The handler reads the faulting CONTEXT and rewrites RIP and RSP to enter managed exception handling.
   - Stack overflow recovery rewrites RSP to the fault address and resumes in a `_resetstkoflw` stub.
2. **The garbage collector suspends threads at arbitrary instructions.**
   - Boehm's Windows port calls `SuspendThread`, then `GetThreadContext`, and scans RAX–R15 and XMM0–15 as possible roots (`bdwgc/win32_threads.c`, `copy_ptr_regs`; upstream source).
   - Mono's own thread layer injects callbacks with `SetThreadContext` (`mono/utils/mono-threads-windows.c`).
   - No safepoint polling was found in the amd64 JIT.
   - A guest register held in a host register and missing from the reconstructed CONTEXT is a missed root, and the object it points to is freed while live.
3. **JIT code reads the TEB directly.** `mono_amd64_emit_tls_get` emits `mov reg, gs:[0x1480 + key*8]` (TlsSlots) for keys below 64 and an indirect load through `gs:[0x1780]` (TlsExpansionSlots) above that. A `TlsGetValue` contract does not cover this; the TEB layout itself is guest-visible.
4. **Code is written and patched in place.**
   - Code memory is `VirtualAlloc(..., PAGE_EXECUTE_READWRITE)` (`mono/utils/mono-codeman.c`, `mono-mmap-windows.c`).
   - Call sites are patched after first use (`amd64_patch`), with no cache flush, as x86 needs none.
   - This is the v0.2 §5.1 code-publication contract, exercised on the first managed method.
5. **Unwind tables for generated code** are registered with `RtlAddGrowableFunctionTable`, which Mono resolves from ntdll at run time, falling back to `RtlInstallFunctionTableCallback`. Wine bug 46370 (bug tracker, `https://bugs.winehq.org/show_bug.cgi?id=46370`) records a 64-bit Unity game that crashed because Wine mishandled Mono's initial zero-count table.

Prior art agrees on where the difficulty lies.

- **box64** (source: `src/librarian/library.c`, `src/dynarec/arm64/dynarec_arm64_00.c`):
  - turns on strong memory ordering when it sees `libmonobdwgc-2.0`;
  - inserts barriers on specific field offsets in `unityplayer.dll`, empirical evidence that UnityPlayer's lock-free code depends on x86 store ordering.
- **FEX:** its 2609 release notes say its full self-modifying-code detection "is used for Unity games in some instances" (release notes, `https://fex-emu.com/`).

w2m's measured guest TSO in the VM (Sprints 008–009: MP litmus 2,245 forbidden outcomes in 50M with EnTSO off, 0 in 50M with it on) answers the ordering half of this. That is the strongest finding in the project's favor for the VM back end.

### 2.6 Graphics

Unity 2019.4 compiles D3D shaders with FXC at build time (Unity manual, "Shader compilation", 2019.4; official documentation), so the game carries DXBC, not DXIL. Apple's Metal Shader Converter accepts DXIL only (Apple, "Metal shader converter"; official documentation). DXMT (`https://github.com/3Shain/dxmt`; open source) contains a DXBC parser and a DXBC-to-AIR compiler; it is the nearest existing DXBC-to-Metal path for comparison. No graphics work exists in w2m.

---

## 3. Findings

Severity reflects how much later work depends on the item.

### High

**H1. The runtime's architecture is undecided and the interim one does not scale.**
- `WORKING_AGREEMENT.md:12` still reads "Guest runtime and translator: not yet decided" after 33 sprints.
- In the interim design, every kernel32 call and every dispatch miss leaves the VM, crosses a line-oriented text pipe to Python (`vm_backend.py`, `VM.run`), and runs there.
- Generated implementations are Python functions with no loops, calls or imports (`candidate.py`, `admit`). That shape fits SetLastError and cannot express CreateThread, VirtualProtect, WaitForMultipleObjects or a VEH dispatcher.
- Items H2–H4 all depend on where guest-visible state lives and in what language the runtime is written.

**H2. Guest state is incomplete, and no design exists for reconstructing it precisely.**
- Section 2.3 lists the missing state.
- Section 2.5 shows that the game's runtime reads and writes full CONTEXT records at faults and at asynchronous suspension points.
- The VM back end computes flags eagerly: roughly 15 AArch64 instructions per flag-setting x86 instruction (`_set_flags`, `_set_zf_sf_pf`), plus undefined-flag bookkeeping. That suits lockstep checking. It will need replacing (lazy flags or NZCV mapping, as in Rosetta 2, FEX and box64), and the replacement must still yield exact flags at any instruction boundary.
- Adding SSE rules before this design exists means writing them twice.

**H3. Nothing independent checks the lifter's x86 semantics.**
- `vm_backend.lockstep` and `state_differential` compare the AArch64 back end with `runtime.Runtime`. Both execute the same operations from `lift.py`, so a wrong lifting rule appears identically on both sides and passes both checks.
- The only independent check today is the outcome of two small probes on the Windows runner.
- The 2026-10-08 decoder-theory Decision, item 4 ("semantic comparison by execution", citing MeanDiff and Dasgupta et al.), was not implemented. The project's own citation of Knight and Leveson (1986) on correlated failure applies directly here.
- At 22 rules this is tolerable. At the 300-plus classes the game reaches it becomes the main correctness risk.
- `encgen` already generates single-instruction cases; running them on the runner's CPU with register and memory state captured before and after gives an oracle that shares no code with `lift.py`.

**H4. The decoder phase's exit criteria were met on the wrong fixture.** Criteria (a)–(c) hold on the Python corpus. On UnityPlayer.dll the decoder refuses 8,713 reachable SSE positions (section 2.2). Criterion (d) and Phase 3 acceptance (`WORKING_AGREEMENT.md:124`, `:126`) still name BioShock 2 Remastered, which the 2026-10-09 Decisions moved to the 32-bit profile.

**H5. Sprint 033's comparison is off the signal plane, and its observation contract cannot be graded.**
- `scripts/jit_capture_read.py` prints counts. It opens no substrate run and emits no `COMPARISON_COMPLETED` or `DISCREPANCY_*`, the defect Sprint 025 fixed for the decoder differential.
- The card's expectation for the differential is "whatever the record shows" (card line 61), which names no outcome.
- No code joins the capture against `specs/x86_decode_confirmations.json`, so criterion (d) is not measured.
- The contract's "at least one method of Assembly-CSharp" cannot be checked: `jit_capture.c:46` records `mono_method_full_name`, which carries no assembly name, and the reader's "assemblies" counter groups by namespace. `mono_method_get_class` → `mono_class_get_image` → `mono_image_get_name` supplies it.

### Medium

**M1. Records disagree with the measured depot.**
- The 2026-10-09 Decision and the Sprint 033 card say 298 files and 15,584,319,636 bytes. `files.json` says 297 files and the same bytes.
- The 298th file is `steamapps/appmanifest_1699230.acf` (736 bytes), which `pack_game.py:21` excludes by design. The record pairs a count that includes it with a byte total that excludes it. The card's observation contract, as written, fails on the count.
- The same Decision says 124 .NET assemblies. `Hail to the Rainbow_Data/Managed/` holds 122.
- Both errors sit in an entry labeled `user`; the correction is an appended agent-resolved entry.

**M2. The candidate-loop records are not committed.**
- The Sprint 031 and 032 Built entries cite `runs/candidates-*`. `/runs/` is in `.gitignore` (line 17), and six candidate runs exist only on this disk.
- They are the only evidence that `CANDIDATE_*`, `REDUCTION_COMPLETED` and `DISCREPANCY_RESOLVED` fired, and model output cannot be regenerated.
- This is the defect class of the October 8 review's item 4.8, fixed then only for Phase 2 records.

**M3. Most of the vocabulary has no emitter, including tags for work already done.**
- Of 61 tags, 25 have an emit site.
- Two depot fetches happened without `DISTRIBUTION_ACQUIRED`.
- Four contract acceptances happened without `CONTRACT_ACCEPTED` or `EVIDENCE_RECORDED`.
- Every block the VM back end compiles happens without `CODE_TARGET_DISCOVERED` or `TRANSLATION_PUBLISHED`.
- The translator's discovery and publication tags matter most, because Mono's run-time code is exactly what they describe.

**M4. Hard rule 6 is suspended without a recorded override.** Ten of the last eleven cards record an exception; Sprint 031 touched 33 files. KIT_DIARY hypothesis H3 (whether the two-file rule fits a compiler project) has been pending since 2026-10-03. The evidence now settles it.

**M5. Tools were adopted without decisions or bridge mappings, and the working agreement is stale.**
- SteamCMD (both depots), zstd (`pack_game.py`, the workflow), the Mono embedding API (`jit_capture.c`) and the Unity player command line have no agent-resolved entry and no bridge mapping. `WORKING_AGREEMENT.md:63` still lists DepotDownloader as pending.
- Lines 30–31, 72 and 80 name vocabulary 0.4; 0.5 is in force.
- The canonical home registry has no rows for `loader.py`, `lift.py`, `runtime.py`, `vm_backend.py`, `kernel32.py`, `candidate.py`, `a64_encoder.py` or `games/`.

**M6. No grounds are recorded for choosing Hail to the Rainbow.** The 2026-10-05 profile Decision recorded grounds and rejected alternatives; the 2026-10-09 entries do not. The BioShock 2 correction shows why the step matters: that profile was chosen from a store listing without reading a PE header, and the build turned out to be 32-bit.

### Low

- **L1. Runner disk.** GitHub documents 14 GB of SSD for a standard Windows runner in a private repository (GitHub Docs, "GitHub-hosted runners"; official documentation). `windows-game.yml` downloads all five parts (7.9 GB) before it unpacks 15.6 GB. The run fits only if the D: drive has room, which no record has measured. Downloading and unpacking one part at a time removes the peak.
- **L2.** `jit_capture.c:38` ignores `WriteFile`'s return value; one short write corrupts every later record.
- **L3. The JIT hook's blind spots.** `jit_done` can fire twice for one method (wrappers; `mini.c`), so records need deduplication by `code_start`. Trampolines and IMT, generic, delegate and exception stubs raise `jit_code_buffer`, not `jit_done` (`profiler.h`, `tramp-amd64.c`). Captured bytes predate call-site patching.
- **L4.** `windows-game.yml` checks that listed files are present but not that no other file is. It records the CPU's name but not its feature set; Mono's JIT selects instructions by CPU feature, so a capture is evidence for that CPU only.
- **L5.** `jit_capture_read.py` decodes every position twice and has no test.
- **L6.** `lift.py:146` raises `IndexError`, not `LiftError`, on the moffs form of MOV (`a0` + imm64); reproduced. There are 2 such sites in mono and 13 in UnityPlayer. The runtime's `run` loop does not catch `IndexError`, so such a site crashes the harness rather than stopping the run.
- **L7.** `scripts/run_skeleton.py:44` hard-codes `"vocab_version": "0.5"` where `VOCAB_VERSION` exists.
- **L8.** No KIT_DIARY entry records the 2026-10-09 lesson (a profile chosen from a listing, not a header). Hypotheses H1 and H3 are six days pending.

---

## 4. What holds up

- **The evidence model and the contract format** (`contract.py`). Documented evidence needs a quote and a retrieval date; a hypothesis cannot carry acceptance; observed evidence names its stored executions.
- **The comparator and judge path.** Allowed-outcome sets, the exact relation of 0.2.0, preconditions with witnesses, and `inconclusive` when a witness is missing. Every recorded verdict reproduces from `store/`.
- **The last-error contract's revision (Sprint 032).** A constant passed contract 0.1.0. Contract 0.2.0 draws values at run time from a 256-entry table, so a candidate that ignores its argument passes only by luck. That table carries each value's hash, so the trace needs no SHA-256 in the guest. The constant is rejected and the old verdict is pinned by a test.
- **The VM back end's evidence.** Lockstep plus the random-state differential found a planted signed-carry fault, a parity fault and a store-width fault, after the project learned that lockstep alone missed them (KIT_DIARY #14). Two harness bugs that only a new probe could reach were found and fixed (#16).
- **Guest TSO and 4 KB guest pages in the VM,** measured with controls that can fail (Sprints 008, 009, 012). These are the facts that matter most against section 2.5's ordering and fault requirements.
- **The diary.** Findings #10–#16 are specific, sourced and actionable, and most were turned into gates within a sprint.

---

## 5. Recommendations, in order

1. **Repair the records.**
   - Append corrections for the file count (297 plus one manifest) and the assembly count (122).
   - Re-point decoder criterion (d) and Phase 3 acceptance at Hail to the Rainbow.
   - Bring `WORKING_AGREEMENT.md` to vocabulary 0.5, with registry rows and bridge mappings for SteamCMD, zstd, the Mono embedding API and the Unity command line.
   - Copy the `runs/candidates-*` records into `store/runs/`.
   - Record a project override for hard rule 6 and settle H3.
2. **Decide the runtime architecture**, with grounds:
   - the language of the guest runtime;
   - whether Windows services run inside the guest or across the VM boundary;
   - how guest-visible state is held.

   Items 3–7 build on this decision.
3. **Design the complete guest state before adding instruction rules:**
   - GPRs and RIP;
   - XMM/YMM, MXCSR, and x87 with its control word;
   - flags, with a lazy or NZCV-mapped representation that reconstructs exactly;
   - segment bases, TEB and PEB at their Windows offsets.

   Specify how a CONTEXT is produced at any guest instruction boundary, including mid-block.
4. **Build an independent semantics oracle**: `encgen` cases executed on the runner's CPU with full before-and-after state, judged against `lift.py`. Do this before the lifter grows past its current 22 classes.
5. **Measure the frontier as a sprint of its own.**
   - Load each of the seven native files and run it until it stops.
   - Emit each stop as a signal: `EXECUTION_FAILED` and `BRIDGE_CALL_UNMAPPED` exist in the vocabulary.
   - Separately, extend `jit_capture` to log which imported functions the game calls on the runner, with counts and threads.

   Rank lifter rules, decoder gaps and contracts by those two lists, not by import tables.
6. **Widen the skeleton in the order the game's runtime forces:**
   1. multiple images, with relocations, TLS and DllMain;
   2. faults with VEH dispatch and precise CONTEXT;
   3. threads with suspend, get-context and set-context;
   4. a JIT region Mono writes and patches, with write detection and translation invalidation (v0.2 §5.1).

   Each step gets a probe on the runner and a contract.
7. **Finish Sprint 033 on the record.**
   - Put the capture through the `--record` path.
   - Write expected bounds into the card before the run.
   - Record the image name in the hook, and add the `jit_code_buffer` callback.
   - Deduplicate by `code_start`.
   - Check `WriteFile`.
   - Record the runner's CPU feature set.
8. **Open graphics as its own phase**, with the DXBC input path decided against the comparison subjects DXMT and Metal Shader Converter. UnityPlayer cannot reach a first frame without it.

---

## Sources

Project artifacts are cited by path in the text. External sources:

- Unity-Technologies/mono, branch `unity-2019.4-mbe`. Source code. `https://github.com/Unity-Technologies/mono/tree/unity-2019.4-mbe`. Files cited: `mono/mini/mini-windows.c`, `mono/mini/exceptions-amd64.c`, `mono/mini/mini-amd64.c`, `mono/mini/mini.c`, `mono/mini/mini-runtime.c`, `mono/mini/tramp-amd64.c`, `mono/utils/mono-codeman.c`, `mono/utils/mono-mmap-windows.c`, `mono/utils/mono-threads-windows.c`, `mono/metadata/profiler.c`, `mono/metadata/profiler.h`.
- bdwgc, `win32_threads.c`. Source code. `https://github.com/ivmai/bdwgc/blob/master/win32_threads.c`. Unity's fork, `external/bdwgc`, was not read.
- box64. Source code. `https://github.com/ptitSeb/box64`. Files cited: `src/librarian/library.c`, `src/dynarec/arm64/dynarec_arm64_00.c`, `docs/USAGE.md`.
- FEX-Emu release notes. `https://fex-emu.com/`.
- Wine bug 46370. Bug tracker. `https://bugs.winehq.org/show_bug.cgi?id=46370`.
- Unity Manual 2019.4, "Shader compilation". Official documentation. `https://docs.unity3d.com/2019.4/Documentation/Manual/shader-compilation.html`.
- Unity Manual 2019.4, "Player command line arguments". Official documentation. `https://docs.unity3d.com/2019.4/Documentation/Manual/PlayerCommandLineArguments.html`.
- Apple, "Metal shader converter". Official documentation. `https://developer.apple.com/metal/shader-converter/`.
- DXMT. Open source. `https://github.com/3Shain/dxmt`.
- Valve, "steam_api.h". Official documentation. `https://partner.steamgames.com/doc/api/steam_api`.
- GitHub Docs, "GitHub-hosted runners". Official documentation. `https://docs.github.com/en/actions/reference/runners/github-hosted-runners`.
- Knight, J. C., and Leveson, N. G. "An experimental evaluation of the assumption of independence in multiversion programming." *IEEE Transactions on Software Engineering* SE-12(1), 1986.
