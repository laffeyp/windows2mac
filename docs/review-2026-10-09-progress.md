# w2m progress review and distance to play — 2026-10-09 (evening)

Scope: commits d7a6f36 through bae6143. This covers Sprint 033 part 2, the record repairs, the runtime architecture decision, Sprints 034–036, and Sprint 037, which closed during the review (3508aca). The work is measured against the morning review, `docs/review-2026-10-09-project.md` ("the morning review" below). Section 5 then sets a baseline: how far w2m is from playing a game, any game, 32-bit or 64-bit, and where it stands against Wine, CrossOver and Proton.

---

## Method

1. **Read:**
   - the Sprint 034–037 cards in full;
   - every BLACKBOARD Decision dated 2026-10-09;
   - `specs/guest_state.json`;
   - `tools/guest_runtime/runtime.c`;
   - the Sprint 033 runner's `unity.log`.
2. **Ran the gates:** `scripts/gates.sh` exits 0, with 231 tests passed.
3. **Re-ran the frontier:** loaded the game's executable from the depot and ran it in `runtime.Runtime`.
4. **Judged one of the semantics oracle's runner outputs** without recording a run. The command was `scripts/semantics_differential.py --judge store/probe-runs/38001576821`.
5. **Measured the Hypervisor.framework vCPU limit on this Mac** with `hv_vm_get_max_vcpu_count`.
6. **Gathered external facts** on Wine, CrossOver, Game Porting Toolkit, Rosetta 2, FEX, box64, Prism and ProtonDB, from primary sources. Each one is labeled with its kind of source in section 5.

---

## 1. Verdict

The morning review was applied in full and in the order it asked for, within one day.

- **Done:**
  - all six medium findings;
  - all eight low findings;
  - H1 (the runtime architecture), H4 (the decoder on the game) and H5 (Sprint 033 put on the record).
- **Specified and measured on Windows, but not built:** H2, the full guest state.
- **Done, and the strongest result of the five sprints:** H3, the semantics oracle. It found two lifter defects that the existing checks between w2m's own two back ends could never have found.
- **Held as promised:** the sequence entry said no lifter rule would be added before the state design. None was. The lifter handles the same 22 instruction kinds as this morning: 53 labels, once each Jcc and SETcc condition is counted separately.

The game has not moved. `Hail to the Rainbow.exe` still stops after 13 instructions:

> `Unimplemented: import KERNEL32.dll!GetSystemTimeAsFileTime`

That was the expected result. These five sprints built the floor the game will stand on, not reach toward the game.

The new work also brings two design problems into view that were invisible this morning (section 3):

- **A thread limit.** The architecture gives each guest thread its own vCPU, and Hypervisor.framework allows 64 vCPUs per VM on this Mac.
- **A wrong reconstruction rule.** The rule for building a CONTEXT after an asynchronous stop re-executes instructions that are only half done.

---

## 2. How each morning finding was handled

| Finding | Action | Where | Status |
|---|---|---|---|
| H1 runtime undecided | The managed VM; Windows-visible state in guest memory at Windows layouts; services in the guest as freestanding C; one vCPU per guest thread; the translator stays Python on the host behind one interface | Decision 0610f1e; Sprint 035 (366e255) | Done. The probes' kernel32 now runs as AArch64 C inside the VM. No VM run calls `kernel32.py`. The last error lives at TEB+0x68, where Windows keeps it. |
| H2 guest state | `specs/guest_state.json` 0.1.0: a 0x380-byte state block, a host register map, a reconstruction-table format, and CONTEXT/TEB/PEB offsets | Sprint 034 (f6d1a6e) | Specified and measured on the runner (62 fields, 0 differences, four planted faults caught). **Not implemented**: `lift.py`, `runtime.py` and `vm_backend.py` contain no XMM, MXCSR or reconstruction code. Flags are still computed eagerly into x19–x23. |
| H3 no independent semantics check | Single instructions run on the runner's CPU from stated states, then judged against `runtime.Runtime` | Sprint 037 (740dd25, bae6143, 3508aca) | Done for general registers and flags (section 3.1). XMM is not yet compared. |
| H4 decoder measured on the wrong fixture | Rows generated from the SDM's opcode lines and judged by XED; strict mandatory-prefix rule | Sprint 036 (a728a16) | Done. All 718 spec rows the game uses are XED-confirmed. Of 6,192,478 positions, 6,192,476 decode; the 2 that don't are APX encodings, which XED also refuses for Sapphire Rapids and Alder Lake. |
| H5 Sprint 033 off the record | Capture format 2 (image names, `jit_code_buffer`, CPUID, checked writes); `game_code_differential.py` on the signal plane | 666c4eb, 222969c | Done. 4,500 methods were captured, 564 of them from Assembly-CSharp, plus 5,124 trampolines and stubs. |
| M1 counts | 298 = 297 depot files + the SteamCMD manifest; 122 assemblies | fff572f | Done |
| M2 candidate runs not in git | Copied to `store/runs/` | fff572f | Done |
| M3 tags with no emitter | `CODE_TARGET_DISCOVERED`, `TRANSLATION_PUBLISHED` (62 each per VM run) | Sprint 035 | Partial; the rest are assigned to Sprint 038 by name |
| M4 hard rule 6 | Project override: one concept, one observation contract and one verdict per sprint; the file count is recorded, not capped | fff572f | Done; KIT_DIARY H3 recorded as refuted |
| M5 tools and stale agreement | Bridge mappings for SteamCMD, zstd, the Mono profiler API and the Unity player; vocabulary 0.5; registry rows | fff572f | Done |
| M6 profile grounds | Grounds recorded, read from PE headers | fff572f | Done |
| L1–L8 | Free-disk check, checked writes, dedupe (`jit_done` wins over METHOD buffers), unlisted-file check, CPUID recorded, `test_jit_capture.py`, moffs MOV raises `LiftError`, `VOCAB_VERSION`, KIT_DIARY #17 | 666c4eb, fff572f | Done |

Morning recommendations 5 (the frontier as a sprint), 6 (widening the skeleton) and 8 (graphics as a phase) are scheduled, not started. Sprint 038 covers the frontier, the missing emitters and a play-time capture. After it come several images, then VEH, then threads, then the Mono JIT region. Graphics opens as a phase after that.

---

## 3. Review of the new work

### 3.1 What the new work proves

**The semantics oracle earned its place in the plan within hours.** It has found two defects in rules that had passed lockstep and the state differential with 0 differences since Sprint 030:

- The lifter read `gs:[0x1480]` as absolute address 0x1480 (fixed in 740dd25). That is exactly the TLS form Mono's JIT emits, so the defect would have been the first wrong value in any managed thread.
- 16-bit PUSH and POP moved RSP by 8 instead of 2 (fixed in bae6143).

Sprint 037 closed while this review was being written (3508aca).

- On an Intel Xeon Platinum 8573C, the model agrees with the CPU on all 20,116 cases.
- A planted SUB-carry fault is caught with the CPU as the only reference. Lockstep alone missed that same fault in Sprint 030.

This review judged one AMD run itself (38001576821, EPYC 9V74). It reproduced the recorded result:

- 20,116 cases over the 53 labels;
- 3 runs, with 0 cases whose CPU result differed between runs;
- one class of disagreement: 128 `0F 0D` register forms that fault on AMD.

The oracle compares general registers, flags and two 64-byte memory windows. It does not yet compare XMM state.

**The guest-state probe measured what this morning was argued from source.** A first vectored handler gets a writable CONTEXT, and the thread resumes at the RIP the handler writes. `GetThreadContext` returns a spinning thread's registers, and `SetThreadContext` changes them. The TEB slots sit at 0x1480 and 0x1780, the two places Mono's JIT reads TLS from. The probe set its expected values before the run, and every one held.

**The guest runtime is the right shape.** Its features:

- kernel32 runs as C at a fixed VA inside the VM;
- each import gets an AArch64 bridge;
- host effects cross through one transport page and `hvc #8`;
- calls are logged in a guest ring.

Candidates are now C as well. qwen2.5-coder:7b's body was accepted on the first attempt: `w2m_teb()->last_error_value = code;`. A planted constant was rejected under contract 0.2.0 and then repaired. That proves the generate-verify-repair loop end to end, inside the VM, on a function whose whole body is one line (section 5.5 returns to this).

**The decoder rebuild is reproducible.** `scripts/rebuild_decode_spec.py` rebuilds spec 0.7.0 from 0.6.3 with 0 rows different. The strict prefix rule exposed five instructions that the old fallback had silently decoded as their neighbours.

### 3.2 Findings

**N1 (high). One vCPU per guest thread runs into a hard cap of 64.**
- `hv_vm_get_max_vcpu_count` returns 64 on this Mac (Mac17,7, macOS 26.5.1; measured for this review with a signed test binary).
- Architecture decision item (4) gives every guest thread its own vCPU.
- Unity starts job-system workers, loading, audio and Mono thread-pool threads. Engines built for 16-core desktops routinely pass 64 threads, and Windows thread pools create threads on demand.
- The game's peak thread count is not on the record.

The decision needs an M:N path: a guest scheduler that runs many guest threads on at most 64 vCPUs. That also simplifies SuspendThread, since a suspended thread then holds no vCPU. Sprint 038's runner call log should record peak and total thread counts, so the requirement is measured rather than assumed.

**N2 (high). The reconstruction rule re-executes half-done instructions.**

`specs/guest_state.json` says an asynchronous stop maps "to the guest boundary that precedes it". One guest instruction often becomes several host instructions with more than one guest-visible effect:

- PUSH: a store, then an RSP update;
- `ADD [m], r`: a load, a store, then flags;
- CALL: a store, RSP, then RIP.

Suppose `hv_vcpus_exit` lands after the store and before the register update. Moving RIP back to the preceding boundary runs the store again on resume. For a LOCK XADD reference count, that silently corrupts the count.

Two rules are needed:

- For faults: no host code may commit a guest-visible effect of an instruction before that instruction's last possible fault point.
- For asynchronous stops: the vCPU runs forward to the next table boundary before the CONTEXT is built, instead of the table mapping backward.

The oracle can test the first rule with fault cases. The second needs a probe that suspends inside multi-effect sequences.

**N3 (medium). AF and DF have no home in the state.**

The host map holds CF, ZF, SF, OF and PF. Two flags have nowhere to live:

- **DF** sets the direction of REP MOVS and REP STOS. MSVC's CRT uses STD and CLD for overlapping copies.
- **AF** is visible through LAHF, PUSHFQ and `CONTEXT.EFlags`.

The oracle compares "each flag the model defines", so it cannot report a flag the model omits. The spec and the oracle should both cover all six arithmetic flags plus DF.

**N4 (medium). SSE needs more than a rounding-mode mapping.**

The spec mirrors MXCSR's rounding control into FPCR. x86 and AArch64 also differ in results:

| Behavior | x86 | AArch64 |
|---|---|---|
| Out-of-range float-to-int conversion (CVTTSS2SI) | 0x80000000, the "integer indefinite" value | FCVTZS saturates |
| MINPS/MAXPS when either input is NaN | Returns the second operand | FMIN/FMAX propagate the NaN |
| Default NaN | Negative, 0xFFC00000 | Positive, 0x7FC00000 |

Unity's math code and Mono's float conversions reach all three. Two steps belong before the first SSE rule:

- Extend the oracle with XMM state and NaN, denormal and overflow boundary cases.
- Probe whether this host's FPCR offers Arm's alternate floating-point behavior (FEAT_AFP, FPCR.AH), the same way Sprint 008 probed EnTSO.

**N5 (medium). The reference CPU is whatever GitHub assigns, and the guest's CPUID is undecided.**
- Sprint 037's runs landed on three CPU models: AMD EPYC 7763, AMD EPYC 9V74 and Intel Xeon Platinum 8573C. Intel and AMD differ in 904 cases:
  - 128 `0F 0D` register forms, which are a NOP on Intel and #UD on AMD;
  - SHR and MUL cases that differ only in flags the SDM leaves undefined.
- The project handled the first half well. Every verdict now names its CPU, and a Decision records the vendor split.
- The second half is open. Mono's JIT selects instructions from CPUID, and Sprint 033's capture came from a Zen 3 CPU. The code Mono generates under w2m depends on the CPUID w2m reports.

Reporting no AVX, for example, removes the VEX forms from every method the JIT compiles. The guest CPUID needs a recorded decision with grounds, and the oracle and the capture should run on a CPU that matches it.

**N6 (low). The transport's round trip is unmeasured.** The architecture's revisit condition reads "the transport's measured round trip makes a host effect miss its frame budget". No sprint has measured `hvc #8` service time. Graphics will cross this boundary thousands of times a frame unless commands are batched through a shared ring. The number decides that design.

**N7 (low). Records drift from what was built.**
- Decision 0610f1e says `clang --target=aarch64-none-elf`. Sprint 035 built a Mach-O MH_PRELOAD image with `ld -preload`, because the host has no `ld.lld`. The card records this; no Decision does.
- `WORKING_AGREEMENT.md` still says "Primary build commands: none yet; Sprint 0 is content-only."

**N8 (low). Two observation contracts still say "whatever the record shows".** These are the candidate loop in Sprint 035 and the judge in Sprint 037. Sprint 037 backs its contract with a planted fault, which makes it gradable; Sprint 035's does not.

**N9 (low). The Sprint 033 capture covers 190 s of startup only.**
- It ran in batch mode with the Null graphics device.
- `SteamAPI_Init` failed, because the runner has no Steam client.
- Media Foundation video decoding was disabled.

Criterion (d) holds for the startup code. Play-time code, rendering, Steam and video are outside it. Sprint 038's play-time capture is the right next step.

---

## 4. Where the game stands, measured tonight

| Measure | Morning | Evening |
|---|---|---|
| Game exe frontier | 13 instructions, `GetSystemTimeAsFileTime` | same |
| Instruction kinds the lifter handles | 22 | 22 (two rules corrected) |
| Decoder positions refused in the game's code | 8,713 in UnityPlayer alone | 2, both APX and correct to refuse |
| Spec rows the game uses that XED confirms | not measured | 718 of 718 |
| kernel32 functions implemented | 7, Python on the host | 7, C in the guest |
| Distinct imports across the game's 7 native files | 634 from 21 DLLs | same |
| Guest state: XMM, MXCSR, x87, reconstruction | none | specified and measured; not built |
| Independent semantics check | none | 20,116 cases on a real CPU, all agreeing on Intel; GPRs and flags only |

UnityPlayer.dll imports from 18 DLLs: advapi32, bcrypt, crypt32, gdi32, hid, imm32, kernel32, ole32, oleaut32, opengl32, setupapi, shell32, shlwapi, user32, version, winhttp, winmm and ws2_32. It loads d3d11, dxgi and XInput at run time. The game also uses Media Foundation for video and Steamworks.NET for Steam.

---

## 5. What stands between w2m and a played game

This section lists the work and compares w2m with the systems that run Windows games today. It makes no projection of time.

### 5.1 One game: Hail to the Rainbow, playable

| Milestone | What it needs that does not exist |
|---|---|
| **A. Batch-mode parity**: w2m runs the game with `-batchmode -nographics` and writes the same 168-line `unity.log` the runner wrote | Several images with relocations, TLS and DllMain. The CRT and Win32 startup surface; Sprint 038's call log will give the count, and UnityPlayer imports 490 functions. Rules for 323 of UnityPlayer's 345 instruction classes, among them SSE, the LOCK atomics, IMUL/DIV, CMOVcc, shifts and string instructions, with the oracle extended to XMM. Exceptions: VEH, SEH unwinding and C++ exceptions. Virtual memory and file mapping. File I/O over 15.6 GB of assets. Threads with waits and suspension (N1, N2). Mono's JIT region with write detection and retranslation. |
| **B. First frame** | A window and message loop over AppKit through the transport (N6). DXGI swapchain to CAMetalLayer. The D3D11 device and context. A DXBC-to-Metal shader path for the game's FXC shaders. The comparison subjects are DXMT (bundled in CrossOver 26 as 0.72) and D3DMetal. |
| **C. Playable** | Input (XInput, raw input). Audio over CoreAudio. Steamworks; the runner log shows the game continues after `SteamAPI_Init` fails, so a stub may hold at first. Media Foundation video. Frame time under 16.7 ms: ahead-of-time translation, flags kept in NZCV or computed lazily instead of about 15 instructions per flag-setting op, and register allocation across blocks. |

Two facts could force rework:

- **The generated-implementation bet.** The candidate loop has so far produced two one-line functions (section 5.4).
- **N1 and N2,** if they force a redesign after the state is built rather than before.

### 5.2 Any 64-bit game

"Any game" is not a bar any system meets, Wine and Proton included.

- **Anti-cheat.** AreWeAntiCheatYet tracks 1,167 anti-cheat games; 639 are listed Broken and 53 Denied under Wine/Proton (repo data, 2026-10-08). Kernel-mode systems such as Vanguard refuse any translation layer by design. EAC and BattlEye work under Proton only when each developer opts in. No translator can change that.
- **Valve's own measure.** As of August 2026, 8,964 Steam games are Deck Verified and 21,083 are Playable (SteamDB counts via PC Gamer; press). Boiling Steam's analysis of ProtonDB reports puts the share of games that refuse to launch near 10% (independent analysis, 2025-10-26). That figure leans toward recent, popular titles.

The reachable target is single-player games without kernel anti-cheat. Beyond Hail to the Rainbow, each new engine or API adds a surface:

- **D3D12 and DXIL** (The Blood of Dawnwalker): Metal Shader Converter covers DXIL. The D3D12 runtime itself is a second graphics stack.
- **D3D9 and older:** a third graphics stack, in many 32-bit games.
- **Unreal Engine 5:** no JIT, but a larger native surface and heavy threading (N1).
- **Launchers** (EA app, Ubisoft Connect): embedded Chromium, a very large Win32 and COM surface.
- **Denuvo and SteamStub:** self-modifying, obfuscated code that stresses the translator's code-publication path.
- **Video and middleware:** Media Foundation, Bink, FMOD and Wwise.

### 5.3 32-bit games

32-bit is a second translator mode, not a flag:

- no REX prefix, and 40–4F are INC/DEC;
- the TEB through FS;
- stdcall, thiscall and fastcall;
- code heavy in x87, which BioShock-era engines use;
- a 4 GB address space;
- every Windows service needs 32-bit entry points.

Wine needed until 11.0 (2026-01-13) to call its new WoW64 mode "fully supported" with "feature parity" (release notes). CodeWeavers stops creating 32-bit bottles by default in CrossOver 26, and CrossOver 27 removes them (press).

w2m has a structural advantage here. The VM owns the whole guest address space, so a 4 GB guest needs no low-memory tricks inside a macOS process. The ordering and 4 KB-page results carry over unchanged. Most services can share one implementation behind 32-bit entry points.

### 5.4 Against Wine, CrossOver and Proton

| | Wine 11 / CrossOver 26 (2026) | w2m (2026-10-09) |
|---|---|---|
| x86 translation on Apple Silicon | Rosetta 2 today. Apple: macOS 27 is the "final release to support Rosetta", with a subset kept "for older, unmaintained gaming titles that rely on Intel-based frameworks" (Apple Developer News, 2026-09-01). No published statement says Wine qualifies. CrossOver's answer is its own macOS port of FEX, in preview since 2026-07-31, without D3DMetal (press) | Its own lifter: 22 instruction kinds, every one confirmed on a real Intel CPU |
| Memory ordering | Rosetta sets TSO for its processes. A third-party translator in a macOS process has no public TSO control (w2m Sprint 008), so FEX on macOS presumably emits ordered accesses or barriers (inference, not verified) | Hardware TSO in the VM: 0 forbidden outcomes in 50M, with plain accesses at 0.99–1.10x native (Sprints 009, 011) |
| 4 KB pages | Wine 11 simulates 4 KB pages on 16 KB hosts, which its notes say works "for simple applications" (release notes) | Hardware 4 KB guest pages in the VM (Sprint 012) |
| Windows API | About 3.53M lines of C in the DLLs and server, excluding bundled libraries and tests; 33 years; about 2,100 named contributors (repo data, wine-11.19) | 7 kernel32 functions, about 100 lines of C |
| Graphics | D3DMetal 3.0 (D3D11/12), DXMT 0.72 (D3D11), vkd3d 1.18 | none |
| 32-bit | New WoW64 at parity in Wine 11.0; CrossOver 27 drops 32-bit bottles | after the first 64-bit frame |
| Games that run | tens of thousands under Proton on Linux; "thousands" claimed by CodeWeavers on Mac (vendor) | 0 |
| Verification | Wine's conformance tests, about 1.80M lines of C, run on real Windows | Contracts judged against Windows runs; a per-instruction oracle on real CPUs; every artifact on a signal record |

**The gap in Windows API code is the dominant term.** It is three to four orders of magnitude, and nothing else in the table comes close.

- Wine closed that gap with people.
- w2m's plan closes it by generating each service from a contract and judging it against Windows.
- That loop works end to end: generate, judge, reject a wrong body, repair, accept. Every body it has accepted is one line touching one TEB field.
- Whether the loop can write CreateFileW, WaitForMultipleObjects, a window procedure or an ID3D11DeviceContext method against contracts is the largest unmeasured fact in the project.

One hard, stateful service gives the first number. CreateFileW with sharing modes and its errors, judged against runner traces, is a good choice. Measure it before the plan depends on the bet for 634 imports.

**Where w2m is ahead.** The two facts that are hard on the Mac are memory ordering and page size, and w2m solved both in hardware, where CrossOver's coming FEX port must solve them in software. w2m does not depend on Rosetta, which loses its general mode after macOS 27. Its verification record is stricter than anything the comparison subjects publish. CodeWeavers, the market leader, is now building its own translator too.

**Where w2m is behind.** Everything a player sees. Wine plus D3DMetal runs games today; w2m runs none.

---

## 6. Use of the SDD kit and of substrate

The Rubber Duck Passes for Sprints 035–037 are specific and name dispositions. KIT_DIARY findings become gates within a sprint. The hard-rule-6 override was recorded with grounds and a revisit condition, instead of being broken quietly.

The defects below share one pattern. **Substrate is used as a file format, and the vocabulary as a list of what was reported.** The computations that move the project run by hand, one item at a time, in the agent session. Section 6.3 shows how to change that, and it is also where the work gets faster.

### 6.1 Substrate

**S1. Nine of eleven substrate users replay finished results through a one-producer topology.**
- `check_guest_state.py`, `semantics_differential.py`, `game_code_differential.py`, `generated_differential.py`, `judge_traces.py`, `page_geometry.py`, `probe_capabilities.py`, `ordering_cost.py` and `litmus_mp.py` all work the same way:
  1. compute everything in plain Python;
  2. build a topology with one producer;
  3. have that producer yield the precomputed SESSION_INIT, COMPARISON_COMPLETED, DISCREPANCY_* and SESSION_COMPLETED events in order.
- Each record shows that a list was written. It does not show that a comparison caused a discrepancy, or that a discrepancy caused a route.
- Substrate's provenance tools (`explain_producer`, `trace_ancestry`, `decisions_between`) have one producer and one trigger to explain. Its `replay` and `assert_replayable` have no decisions to re-verify.
- The w2m code calls 6 names from `substrate.api`: `Runtime`, `TopologyBuilder`, `read_record`, `threshold_count`, `assert_event` and `assert_no_event`. It also uses the Ollama adapter. It never calls `replay`, `first_divergence`, `explain_producer`, `embedded_substrate`, `attach` or any topology recipe.

**S2. The candidate loop is the right kind of topology, and it is the only one.**
- `scripts/run_candidate_loop.py` declares eight producer kinds and six triggers:
  1. a candidate starts an execution;
  2. the execution starts an ingest;
  3. the ingest starts a comparison;
  4. the comparison starts the acceptance guard;
  5. a rejection starts a report and a repair.
- Each step on its record is caused by the one before it. That is what substrate is for: one computation whose steps depend on each other, with the chain on the record.
- It has run on two functions. It is the template for every item run in section 6.3.
- Its correction structure (draft, gate, feed the failure back) is also what substrate ships as `best_of_n_correction` and `coding_flow`. Where the shapes match, w2m can reuse them instead of keeping a second copy.

**S3. The repair loops that moved the project this week ran outside the record.**
- In Sprint 037 the oracle found two real lifter defects: FS/GS operands and 16-bit PUSH/POP. Both were fixed between runner runs.
- The first recorded semantics run (`store/runs/semantics-1791586629398365000`) was taken after the fixes and conforms.
- So the sprint's most valuable discrepancies exist in commit messages and KIT_DIARY #18, not as DISCREPANCY_REPORTED, DISCREPANCY_ROUTED and DISCREPANCY_RESOLVED.
- Sprint 036's 125 decoder discrepancy classes are on the record (`gamediff-1791576553479425000`). Their resolution is implied only by a later run that shows 1. No event links the two runs.
- Sprint 036's card says the SDM row generator "learned, one failure at a time". That loop ran in the session, by hand.
- The vocabulary already routes each discrepancy as `implementation_repair` or `contract_revision`. A trigger on DISCREPANCY_ROUTED that starts a repair producer turns the hand loop into a recorded one. The producer can be a model drafting a lifter rule or a spec row, judged by the oracle or by XED.

**S4. Fifteen hand-written `record()`/`harness()` functions repeat one pattern.** A single `w2m.record.session(events, purpose, profile)` helper would make each script's record a few lines and remove this duplication. If S1 is fixed instead, the helper disappears into real producers.

**S5. Record replay is not a gate.** The morning review re-derived every recorded verdict by hand. Once the records carry real decisions, `assert_replayable` over `store/runs/` in `scripts/gates.sh` makes "every recorded number reproduces" a check that runs on every commit.

### 6.2 The SDD kit

**K1. Decisions the vocabulary has tags for were made in prose instead.**
- 27 of 61 tags in vocabulary 0.5 have an emit site in `src/`, `scripts/`, `tools/` or `probes/`. Of the 34 without one, four describe work that has already happened:
  - **`BACKEND_EVALUATED` and `BACKEND_SELECTED`.** The runtime architecture decision (0610f1e) is exactly a backend selection from measured evaluations (Sprints 008, 009, 011, 012). It exists only as a BLACKBOARD paragraph.
  - **`CONTRACT_REVISED`.** It never fired, though the event contract went to 0.3.1 and the last-error contract to 0.2.0.
  - **`PROBE_AUTHORED`.** It never fired, though the project has authored at least ten runner probes.
- The kit's founding claim is that description is the lossy step signals replace. The decision that most constrains the project is the one held only as description.

**K2. Hard rule 5 is due.**
- The last COMPREHENSION_AFFIRMATION is dated 2026-10-08, after vocabulary 0.4.
- Since then:
  - vocabulary 0.5 came into force;
  - the x86-64 profile changed from BioShock 2 Remastered to Hail to the Rainbow;
  - the architecture was decided;
  - 32-bit was brought into scope.
- Rule 5 names a scope change as a trigger.

**K3. The plan has no single home.**
- The order of work is spread across five Decisions written on 2026-10-09 (BLACKBOARD lines 81, 82, 90, 91 and 92). Each amends the one before it.
- To know the current sequence, a reader has to compose all five.
- `BLACKBOARD.md` is 93,950 bytes, and the kit has every session read its Decisions.
- The canonical home registry (hard rule 7) can name one plan file, edited in place with its history in git. Decisions would then record only why the plan changed. The Decisions stay append-only, and the current plan becomes one read.

**K4. Observation contracts that name no outcome (N8).** "Whatever the record shows" cannot fail. Rule 3's third leg needs a stated bound, even a wide one, or a planted fault that must be caught. Sprint 037 shows the right form.

### 6.3 How the work gets faster: many substrate runs, each one computation

A substrate run orchestrates one computation whose steps depend on each other. The run is valuable because its record holds the chain from cause to result. A substrate run is not a parallel map; independent work does not belong inside one run. **Speed comes from running many independent runs at once, each a complete topology with its own record.**

Today the project does the reverse. One agent session works one item at a time: a sprint per Windows function, per group of instruction rules, per probe. And most of the computation inside each sprint is not orchestrated at all (S1).

**The independent items are already known.** They need no frontier to find them:

| Item | Count | Measured from |
|---|---|---|
| Imported functions across the game's 7 native files | 634, from 21 DLLs (kernel32 294) | PE import tables |
| Instruction classes UnityPlayer reaches that the lifter lacks | 323 of 345 | Capstone traversal and w2m's decoder |
| Later: D3D11 interface methods, DXBC opcodes | not yet counted | — |

**One topology per item.** Each item is the same dependent chain the candidate loop already runs:

1. contract;
2. reference;
3. candidate;
4. build;
5. execute;
6. judge;
7. accept, or report, route and repair.

The two kinds of item differ only in their parts:

- **A Windows function.** The candidate is a C compilation unit in the guest runtime. The judge is the contract against stored Windows traces. This is `run_candidate_loop.py` as it stands.
- **An instruction class.** The candidate is a lifter rule. The judge is the semantics oracle's cases for that class, against stored CPU results. This is a second topology of the same shape, with the oracle as its comparator.

**Run them side by side.** A plain process pool (not substrate) starts K item runs, each writing its own record under `store/runs/`. A failed run stops only itself. Each run's record shows its own chain from contract to accepted artifact.

**The shared resources do not block this:**

- **The Windows runner is out of the inner loop.** Candidates run in the VM and are judged against stored reference traces and stored CPU results. The runner only produces references, once per contract or once per batch of oracle cases. Sprint 037 already did this: 20,116 cases in one job. A runner round trip takes 27–38 s from dispatch to completion, and three ran at once on 2026-10-09 (runs 38001667936, 38001673879, 38001680277; `gh run list`). Item runs that need a reference can wait for it with substrate's `pause_await_input` and continue with `Runtime.resume` on a persistent record, rather than blocking.
- **Model capacity** is the tightest limit. A single local Ollama server on one GPU serves a few requests at a time. Substrate's adapters already reach other models (its applications use `kimi-k2.6:cloud` and `glm-5.1:cloud`), and the record meters every call as `ModelUsage`.
- **Merges** need the artifacts split one item per file. The architecture already makes each Windows service its own compilation unit. Lifter rules live in one file, `lift.py`, so parallel runs would collide there. One rule per class in a table or module removes the collision and makes the lifter plainer.

**One frontier run on top.** The game run is the one inherently sequential computation:

1. run until the game stops;
2. emit BRIDGE_CALL_UNMAPPED or EXECUTION_FAILED;
3. if no item run exists for the cause, start one;
4. when its artifact is accepted, resume.

This run is a substrate topology in its own right. It orders the pool by what the game needs next, while the static lists keep the pool full. Every resume on its record is caused by an acceptance in an item run's record.

**Sessions can split the same way.** Today one session writes reviews while another builds sprints. Separate agent sessions in separate git worktrees can each own a disjoint slice of the item lists, for example kernel32 and user32, or one block of instruction classes. Each session drives its own pool of item runs. The kit's single-writer rule then applies per slice, not per project.

The result is a different structure, not more effort. Most items become a recorded run that needs no sprint card of its own. Sprints are spent on the parts that are one-off design work: N1 to N4, the transport, the shader path.

---

## 7. Recommendations, in order

1. **Decide threading with N1 in view.** Choose M:N scheduling of guest threads on at most 64 vCPUs, or record why one vCPU per thread suffices. Measure peak thread count on the runner in Sprint 038.
2. **Fix the reconstruction rule before building state (N2).**
   - For faults: the lifter commits an instruction's guest-visible effects only after its last fault point. Test this with oracle fault cases.
   - For asynchronous stops: run forward to the next table boundary instead of mapping backward.
3. **Finish the state spec and the oracle before the first SSE rule (N3, N4).**
   - Add AF and DF.
   - Add XMM and MXCSR to the oracle, with NaN, denormal and overflow cases.
   - Probe FPCR.AH (FEAT_AFP) on this host.
4. **Decide the guest CPUID with grounds (N5).** Run the capture and the oracle on a matching CPU.
5. **Restructure the breadth work as many substrate runs (section 6.3).**
   - Build the instruction-class topology with the oracle as its comparator.
   - Split lifter rules one per class.
   - Start a process pool over the 634 imports and the 323 instruction classes.
   - Build Sprint 038's frontier as a topology that starts item runs and resumes on their acceptances.
   - Put every discrepancy on the record, reported, routed and resolved across runs (S1, S3).
6. **Measure the generation bet on a stateful service** (section 5.4) before the pool depends on it.
7. **Put decisions on the signal plane and give the plan one home (K1–K3).**
   - Emit BACKEND_EVALUATED and BACKEND_SELECTED for the architecture decision.
   - Emit CONTRACT_REVISED and PROBE_AUTHORED from the contract loader and the probe workflow.
   - Name one plan file in the registry.
   - Write the affirmation that rule 5 requires.
   - Add `assert_replayable` over `store/runs/` to the gates.
8. **Measure the transport round trip (N6)** before graphics design starts, and design graphics around a shared command ring if the number demands it.
9. **Track CrossOver's macOS FEX port and Rosetta's macOS 27 games subset** as comparison subjects.

---

## Sources

Project artifacts are cited by path and commit in the text. External sources, with their kind:

- Apple Developer News, "macOS 27: Final release to support Rosetta", 2026-09-01. Official. `https://developer.apple.com/news/`
- Apple Developer Forums thread 830692, Rosetta after macOS 27, quoting the WWDC 2025 statement and an Apple engineer's reply. Official forum. `https://developer.apple.com/forums/thread/830692`
- Wine 9.0, 10.0 and 11.0 ANNOUNCE files; wine-11.19 source tree (line counts by `wc -l`, AUTHORS). Release notes and repository data. `https://github.com/wine-mirror/wine`
- CrossOver 26 release coverage. Press. `https://phoronix.com/news/CrossOver-26`; `https://www.omgubuntu.co.uk/2026/02/crossover-26-released`
- CrossOver native ARM64 preview, 2026-07-31. Press, relaying CodeWeavers. `https://appleinsider.com/articles/26/07/31/first-apple-silicon-native-crossover-build-in-testing-as-rosettas-end-nears`
- CodeWeavers, anti-cheat support page. Vendor documentation. `https://support.codeweavers.com/anti-cheat`
- Apple, Game Porting Toolkit page. Official (also vendor marketing for its own product). `https://developer.apple.com/games/game-porting-toolkit/`
- FEX-2610 release notes, 2026-10-07. Release notes. `https://fex-emu.com/FEX-2610/`
- box64 v0.4.0 coverage. Press. `https://www.phoronix.com/news/Box64-0.4-Released`
- Microsoft, Prism AVX/AVX2 rollout, 2025-12-05. Official. `https://techcommunity.microsoft.com/blog/windowsosplatform/-/4475631`
- AreWeAntiCheatYet `games.json`, pushed 2026-10-08. Community repository data. `https://github.com/AreWeAntiCheatYet/AreWeAntiCheatYet`
- PC Gamer, Steam Deck Verified and Playable counts, August 2026 (from SteamDB). Press. `https://www.pcgamer.com/hardware/handheld-gaming-pcs/30-000-games-are-either-steam-deck-verified-or-at-least-playable-on-valves-handheld/`
- Boiling Steam, ProtonDB analysis, 2025-10-26. Independent analysis. `https://boilingsteam.com/windows-games-compatibility-on-linux-is-at-a-all-time-high/`
- Intel SDM Vol. 2, MINPS, CVTTSS2SI, and the QNaN floating-point indefinite; Arm ARM, FMIN, FCVTZS and the default NaN. Official documentation.
- Apple Hypervisor.framework, `hv_vm.h` (`hv_vm_get_max_vcpu_count`), macOS 26 SDK. Official SDK header; value measured on this host.
- substrate `src/substrate/topologies/coding_flow/__init__.py` and `topologies/applications/best_of_n_verified.py`, commit b3874ac. Source.

Not verified: ProtonDB's current tier percentages; CrossOver's compatibility database totals; whether D3DMetal accepts DXBC; whether Rosetta's games subset after macOS 27 covers Wine; whether CrossOver's macOS FEX port uses barriers or another ordering method.

