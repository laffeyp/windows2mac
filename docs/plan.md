# w2m plan — the current order of work

This file is the plan's one home (Decision 2026-10-09, review 2026-10-09 progress K3). It is edited in place; git holds its history; `BLACKBOARD.md` Decisions record why it changed. Each sprint below names what it delivers and how its result is judged.

Goal of the current phase: Hail to the Rainbow (Steam app 1699230, build 24247624, x86-64, Unity 2019.4 with Mono) runs on this Mac. Milestones: **A** batch-mode parity (the game under w2m writes the runner's `unity.log`), **B** first frame, **C** playable (review 2026-10-09 progress §5.1).

## Done (2026-10-09)

- Decoder exit criteria (a)-(d) met; (d) on the game: 718 of 718 rows XED-confirmed (Sprint 036).
- Runtime architecture decided and on the signal plane: managed VM; Windows-visible state in guest memory; services as guest C (Sprints 034-035; `store/runs/backend-selection-2026-10-09`).
- Lifter judged by real x86 CPUs: 20,116 cases, all agreeing on Intel (Sprint 037).
- Threading M:N on at most 64 vCPUs; reconstruction runs forward; AF and DF in the state; guest CPUID x86-64-v2 Intel (Decisions 2026-10-09).

## Next, in order

| Sprint | Delivers | Judged by |
|---|---|---|
| 038 | The frontier as a substrate topology: the game runs in w2m until it stops; each stop is `EXECUTION_FAILED` or `BRIDGE_CALL_UNMAPPED`; the topology starts an item run for the cause and resumes when its artifact is accepted. Emitters: `DISTRIBUTION_RESOLVED`/`ACQUIRED`/`ACQUISITION_FAILED` (SteamCMD driver, both depots), `CONTRACT_PROPOSED`/`ACCEPTED`/`REVISED`, `EVIDENCE_RECORDED`, `PROBE_AUTHORED`. On the runner: the game's calls to Windows functions with counts, its peak and total thread counts, the CPUID leaves its code reads; a play-time capture past the main menu. A `w2m.record` helper replaces the hand-written record functions. | The stop list reproduces from the record; peak threads measured; every new emitter fires on a real run. |
| 039 | The semantics oracle extended: XMM and MXCSR state; NaN, denormal, overflow and float-to-integer boundary cases; AF, DF; fault cases that test the commit-after-last-fault rule. A VM probe for FEAT_AFP in the guest and FPCR.AH. | Planted faults: a lifted MINPS returning the NaN operand, a CVTTSS2SI that saturates, an AF left undefined, and a store committed before a fault are each reported by the CPU alone. |
| 040 | Guest state built: the state block, the reconstruction table and CONTEXT production in the VM back end; lazy AF, DF; the guest CPUID profile; the M:N scheduler skeleton (one vCPU, many guest threads). | Sprint 034's runner records (fault CONTEXT, Get/SetThreadContext) reproduced inside w2m; a suspend inside a multi-effect sequence resumes without re-executing a store. |
| 041 | The instruction-class item topology: lifter rules split one per class; one substrate run per class, the oracle as its comparator; a process pool over the 323 classes UnityPlayer reaches, ordered by the frontier. | Each accepted rule carries an oracle verdict on its record; discrepancies reported, routed and resolved across runs. |
| 042 | The generation bet measured on a stateful service: CreateFileW (sharing modes, dispositions, errors) generated against a contract and judged against runner traces. | Its accepted body passes the contract's cases; the attempt count and the model's usage are on the record. If the loop cannot produce it, the plan's breadth model changes before the pool depends on it. |
| 043 | The Windows-function item pool: one run per import the frontier reaches, kernel32 first; several images (relocations, TLS, DllMain); VEH and SEH. | Milestone A's ordered sub-goals: CRT start-up, UnityMain entered, Mono initialized. |
| later | Threads with waits and suspension on the M:N scheduler; the Mono JIT region with write detection and retranslation (`TRANSLATION_INVALIDATED`); file I/O over the 15.6 GB of assets. | Milestone A. |
| phase | Graphics: a window over AppKit, DXGI to CAMetalLayer, D3D11, DXBC to Metal (comparison subjects DXMT and D3DMetal). Commands batch through a shared ring: one host round trip costs about 27 µs (measured 2026-10-09), about 600 per frame. | Milestone B. |
| phase | Input, audio, Steamworks, video; frame time (ahead-of-time translation, flags in NZCV or lazy, register allocation across blocks). | Milestone C. |
| phase | 32-bit: BioShock 2 Remastered (depot on disk). | Its own milestones, after B on the x86-64 profile. |

## Standing items

- Comparison subjects, checked at each phase boundary and recorded: Wine and CrossOver (D3DMetal, DXMT, CrossOver's macOS FEX port), Rosetta's post-macOS-27 games subset, Prism, box64, FEX.
- The Blood of Dawnwalker (D3D12 profile) waits on the user's purchase.
- Every probe run records its CPU; a verdict names the CPU it came from.
