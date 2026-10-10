# w2m roadmap

This file is the plan's one home (Decision 2026-10-09; review `docs/review-2026-10-09-progress.md`, K3). It is edited in place, with history in git; `BLACKBOARD.md` Decisions record why it changed. Every sprint states what it delivers and how it is judged: a bound, an expected value, or a planted fault that must be caught (WORKING_AGREEMENT, observation contracts).

**Goal of this roadmap:** Hail to the Rainbow (Steam app 1699230, build 24247624; x86-64; Unity 2019.4 with Mono) is playable on this Mac. Milestones:

- **A — batch-mode parity:** the game runs under w2m with `-batchmode -nographics` and writes the same `unity.log` the Windows runner wrote (168 lines, Sprint 033).
- **B — first frame:** the game draws its first frame through Metal.
- **C — playable:** input, audio, video and Steam work, and frames take under 16.7 ms.

Then: the 32-bit profile (BioShock 2 Remastered), and further profiles.

## Where the review's findings land

| Finding | What it asks | Sprint |
|---|---|---|
| N1 vCPU cap of 64 | M:N scheduling; measure the game's threads | Decision done; measured 038; scheduler 047 (it needs a threaded program w2m can run, which needs 045's C runtime functions) |
| N2 reconstruction rule | commit after the last fault point; run forward on asynchronous stops | Spec done (guest_state 0.2.0); oracle fault cases 039; built 040, judged on 039's Windows fault records |
| N3 AF, DF | in the state and the oracle | Spec done; oracle 039; built 040 |
| N4 SSE vs AArch64 | oracle with XMM and FP boundaries; FPCR.AH probe | 039; SSE rules through the pool 041 |
| N5 reference CPU and guest CPUID | a CPUID decision; the game's CPUID reads; Intel references | Decision done; reads measured 038; profile built 040; Intel-run rule 039 |
| N6 transport round trip | measure; design graphics around a ring | Measured (≈27 µs); shared-memory transport 044; command ring in phase 6 |
| N7, N8, K2, K3, K4, S5 | record fixes, affirmation, plan home, outcome rule, replay gate | Done |
| N9 startup-only capture | play-time capture | 038 |
| K1, M3 tags without emitters | decisions on the signal plane | Backend done; acquisition, contract and probe tags 038; translation invalidation 049; the rest by first use. `BRIDGE_CALL_UNMAPPED` stays for Steam interface methods (phase 7); the frontier uses `SERVICE_UNIMPLEMENTED` (vocabulary 0.6) |
| S1, S3 records without causality | real topologies; discrepancies resolved across runs | 041 (instruction items), 043 (existing differentials converted) |
| S2 reuse substrate's correction flows | `best_of_n_correction`, `coding_flow` where the shape fits | 041, 042 |
| S4 duplicated record code | a `w2m.record` helper | 038 |
| §6.3 breadth as many runs | one run per item; a process pool; the frontier as a topology | 038 (frontier), 041 (instruction pool), 045 (function pool) |
| §5.4 the generation bet | CreateFileW against contracts | 042 reframed (user, 2026-10-10): the pipeline first, model generation measured inside it |
| Recommendation 9 | track comparison subjects | standing, at each phase boundary |

## Phase 3 (closing): measure the game and build the floor

| Sprint | Delivers | Judged by |
|---|---|---|
| **038 — the frontier and the game's demands** | The frontier as a substrate topology: the game runs in w2m until it stops; each stop becomes `SERVICE_UNIMPLEMENTED` (vocabulary 0.6, P15); the run records what it needs next. On the runner: every Windows function the game calls, with counts and threads; peak and total thread counts; the CPUID leaves its code reads; a play-time capture past the main menu. Emitters: `DISTRIBUTION_RESOLVED`, `DISTRIBUTION_ACQUIRED`, `ACQUISITION_FAILED` (a SteamCMD driver, both depots), `CONTRACT_PROPOSED`, `CONTRACT_ACCEPTED`, `CONTRACT_REVISED`, `EVIDENCE_RECORDED`, `PROBE_AUTHORED`. The `w2m.record` helper. | The frontier's first stop is `SERVICE_UNIMPLEMENTED` for `GetSystemTimeAsFileTime` after 13 instructions, as measured by hand; the runner call log names at least the 7 functions w2m already serves; peak thread count is a number on the record; every new emitter fires on a real run and the records replay. |
| **039 — the floating-point and fault oracle** | The semantics oracle compares XMM and MXCSR, AF and DF, and covers NaN, denormal, overflow and float-to-integer boundaries and fault cases (a store before a fault). A VM probe for FEAT_AFP in the guest and FPCR.AH. Verdicts that must come from Intel retry until an Intel runner answers. | Planted faults, each caught by the CPU alone: MINPS returning the NaN operand; CVTTSS2SI saturating; AF left undefined; a store committed before a fault. |
| **040 — guest state built** | In the VM back end: the per-thread state block in guest memory, the reconstruction table (host PC to guest instruction), the commit rule (no guest-visible effect before an instruction's last fault point) and the forward rule (an asynchronous stop runs on to the next boundary), CONTEXT and EXCEPTION_RECORD built in the guest runtime at a fault, lazy AF and DF, and the guest CPUID profile (Intel x86-64-v2). | Sprint 039's Windows records: for every fault case the VM can execute, the CONTEXT and exception record the guest runtime builds equal the ones Windows gave its handler on Intel (exception code, RIP, the 16 GPRs, RFLAGS; and no memory store before the fault, from the recorded windows); XMM and MXCSR join when the VM carries vector state (041); AF and DF equal the CPU's on every case the VM executes; with the vCPU stopped at random host instructions, a loop of `ADD [m], 1` and PUSH/POP ends with the exact count; the lifter's every rule passes a static commit-order check. |

## Phase 4: the breadth engine

| Sprint | Delivers | Judged by |
|---|---|---|
| **041 — instruction classes as item runs** (closed 2026-10-10: 139 classes accepted on their own records; the VM runs SSE and the next block; rules are hand-drafted, model drafting is 042's bet) | Lifter rules split one per class (the 47 legacy-SSE floating-point classes Sprint 039 gave the model, over `x86fp`, are the first block already judged by the CPU; the AArch64 lowering of each uses FPCR.AH=1, which Sprint 039 measured to match x86 on MIN/MAX, NaN results, tininess and denormal inputs, and a fix-up after FCVTZS, which does not match); a substrate topology per class (draft a rule, build, run the oracle's cases, accept or report-route-repair), reusing substrate's correction flow where its shape fits; a process pool over the 323 classes UnityPlayer reaches, ordered by the frontier; SSE rules on the FP oracle. | Every accepted rule has an oracle verdict on its own record; discrepancies are reported, routed and resolved across runs; the pool clears a first block of classes chosen by frequency (MOVAPS, MOVSS, MOVUPS, MULSS, the LOCK atomics, IMUL, CMOVcc, shifts). |
| **042 — the Windows-function pipeline, on CreateFileW** (reframed 2026-10-10, user decision; closed 2026-10-10: the VM prints Windows's 188 rows; a cloud model wrote an accepted file.c in substrate's session topology) | A no-CRT probe on the runner records Windows's CreateFileW across dispositions, sharing modes and errors; a contract from those records; a file-system bridge from the guest runtime to the Mac; CreateFileW, CloseHandle, DeleteFileW, ReadFile, WriteFile and SetFilePointerEx (the game's file calls in play run 38006665419) in the guest runtime; a judge. Then a measurement: can a cloud model (user decision 2026-10-10: local models cannot) write a passing body inside this pipeline. | The VM's output equals Windows's on every scenario; planted faults (a sharing rule inverted, ERROR_ALREADY_EXISTS dropped) caught; the model measurement's attempts and outcome on the record, nothing depending on it. |
| **043 — causality in every record** (closed 2026-10-10: one differential topology, nine causal records, a ledger derived from them) | The existing differentials (decoder, game code, guest state, semantics, capability, ordering) become real topologies: the comparison causes the discrepancy, the route causes the repair run, a later acceptance resolves it. | `explain_producer` and `trace_ancestry` on each record show the chain; replay stays green. |
| **044 — the launcher's C runtime surface, by family** (reordered 2026-10-10, review 2026-10-10 D3/D5/D6; closed 2026-10-10: eleven families accepted, the frontier stops at `UnityPlayer.dll!UnityMain` after 19,084 instructions; a cloud model wrote an accepted family) | The 042 pipeline per family of shared state: heap, TLS, critical sections, code pages and strings, command line and environment, modules, unwinding and failure, console and files, and one probe for the stateless queries; each family one probe on the runner, one contract from its rows, guest C, one item run; families run concurrently as separate substrate runs; one or more families written by a cloud model in the session topology (D7). QueryPerformanceCounter (CNTVCT_EL0 scaled to 10 MHz), TlsGetValue and GetCurrentThreadId stay in the guest (Decision 2026-10-09 hot imports; D4). The census of instruction classes the launcher and UnityPlayer reach without a rule, rerun. | The frontier on the launcher: its C runtime start-up completes and the stop is the call to `UnityPlayer.dll!UnityMain`; every family accepted on its own record. |

## Phase 5: milestone A, batch-mode parity

| Sprint | Delivers | Judged by |
|---|---|---|
| **045 — the transport** (run when a measured call rate needs it; review 2026-10-10 D5; waiting 2026-10-10: 17 host requests over the launcher's start-up, BLACKBOARD) | The text pipe between Python and the VM monitor replaced by shared-memory rings; host requests batched. | The round trip measured before and after (27 µs today); lockstep and the oracle unchanged. |
| **046 — several images** (closed 2026-10-10: DllMain returns, the launcher enters UnityMain, UnityPlayer stops at `SHELL32.dll!CommandLineToArgvW` after 403,044 instructions) | Loading `UnityPlayer.dll`, `mono-2.0-bdwgc.dll` and the rest: relocations, imports between images, TLS callbacks, DllMain, the PEB loader list. | Each native file's DllMain runs to its return; the loader list matches the runner's; the launcher enters `UnityMain` (moved from 045, review 2026-10-10 D3). |
| **047 — exceptions and threads** | VEH dispatch to guest handlers over the CONTEXT built in 040, SEH unwinding, C++ exceptions; the M:N scheduler (moved from 040) with waits, events and suspension. | Sprint 028's event contract and Sprint 034's own probe (the C runtime and all) conform under w2m: the vectored handler's CONTEXT and write-back, GetThreadContext and SetThreadContext on a suspended thread. |
| **048 — memory and files** | VirtualAlloc/Protect at 4 KB, file mapping, file I/O over the 15.6 GB of assets. | The game reads its first asset bundle; page-protection faults match the runner. |
| **049 — the Mono JIT region** | Write detection on RWX regions, invalidation and retranslation, `TRANSLATION_INVALIDATED`. | Mono's first managed method runs; a rewritten call site is retranslated, never stale. |
| **050 — milestone A** | The game under w2m in batch mode. | Its `unity.log` matches the runner's 168 lines. |

## Phase 6: milestone B, first frame

A window and message loop over AppKit; DXGI to CAMetalLayer; the D3D11 device and context; DXBC to Metal (comparison subjects DXMT and D3DMetal); commands through a shared ring (one round trip ≈27 µs, about 600 per frame). Sprint cards are written when phase 5 closes, from the frontier's record.

## Phase 7: milestone C, playable

Input (XInput, raw input), audio (CoreAudio), Steamworks (a stub first: the game continues after `SteamAPI_Init` fails), Media Foundation video; frame time under 16.7 ms (ahead-of-time translation, flags in NZCV or lazy, register allocation across blocks).

## Phase 8 and after

- 32-bit: BioShock 2 Remastered (depot on disk): a second translator mode, the TEB through FS, x87, stdcall and thiscall, 32-bit entry points for each service.
- Further profiles: The Blood of Dawnwalker (D3D12/DXIL) after the user's purchase; then by engine and API.

## Standing items

- Comparison subjects, recorded at each phase boundary: Wine and CrossOver (D3DMetal, DXMT, CrossOver's macOS FEX port), Rosetta's post-macOS-27 games subset, Prism, box64, FEX.
- Every probe run records its CPU; a verdict names the CPU it came from.
- Gates on every commit include record replay.
