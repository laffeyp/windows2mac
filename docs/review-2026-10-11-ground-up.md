# Review 2026-10-11 — ground up: direction, work, SDD, substrate

Question: measured against `windows_to_macos_research_direction_v0_2.md`, `sdd-kit-3/AGENTS.md` and substrate's own account of itself, is the work going where it says, is it built the way it says, and is substrate used for what substrate is?

## Method

Read `sdd-kit-3/AGENTS.md` and `sdd-kit-3/CLAUDE.md` in full; `docs/plan.md` and `WORKING_AGREEMENT.md` in full; BLACKBOARD's Decisions from 2026-10-10 on, Surfaced for review, Deferred, Open questions, Drift watchlist and Sprint tail; KIT_DIARY's findings #40–#49; v0.2 §1, §11.2–§11.4, §12.1; substrate `README.md` and `docs/api.md` (topology, routes, composition, replay). Read `scripts/run_frontier.py`, `scripts/family_run.py`'s topology, `scripts/work_list.py`, `src/w2m/kernel.py`, the scheduler's entry points in `vm_backend.py` and `threads.c`, `heap.c`. Counted substrate API use across `src/` and `scripts/`. Read the durations recorded in the frontier records under `store/runs/`, and CI runs 38195353233 and the latest on `bae2a4da`. Nothing was timed.

## 1. Verdict

The work moves fast and is honest about itself. In two days the frontier went from 13 instructions to past UnityPlayer RVA 0x143a6ca, through threads, COM, structured exceptions and a second process, and every sprint closed on green CI. The CPU half matches v0.2 §11.2: models (`integer_model.py`, `x86fp.py`) judged case by case against Intel CPUs.

Three things are off. The Windows half is judged on samples the implementer saw, and the last four sprints show what that misses (§2, G2). Substrate records the work but does not run it; every decision about what to start next lives in the agent's session and in BLACKBOARD prose, the opposite of substrate's design (§5, S2). And a user decision of 2026-10-10 — families in parallel worktrees — was deferred by the agent and never revisited when its own condition came true (§4, K1).

## 2. Direction

### G1 — The product is not on the plan

v0.2 §1 names the product: `w2m steam run <appid>`. §12.1 defines its slice: resolve the distribution, obtain it, prepare it, establish the Steam session, run, keep state, close, relaunch, update, remove. `pyproject.toml` has no `[project.scripts]` entry, `src/w2m/` has no command-line module, and no `.app` bundle is produced (§4.3). `scripts/acquire_depot.py` and `scripts/pack_game.py` cover acquisition. The plan's milestones end at "C — playable", and phase 7 holds a Steamworks stub. Running the game first is a fair order. But `docs/plan.md` is the plan's only home, and it has no row for the thing v0.2 says w2m is.

### G2 — The Windows half is a sample, not a behavioral definition

v0.2 §11.2 asks for contracts with "state, operations, preconditions, transitions, observable effects, permissible nondeterminism", from which "a contract compiler derives model tests, operation-sequence generators". Each w2m family contract is the runners' rows (`exact_rows`), plus relations that the implementer picks where machines differ (`family_run.py` lines 5–10; Decision 2026-10-10 on integrity levels: "compares whether the RID is low"). Feasibility review F2 is still open: no contract, script or source file holds rows back from the writer.

The record shows the result. A review of Sprint 054 found eight defects in six families that had passed:
- a path overflow in file search;
- an unchecked listing transfer;
- registry names narrowed to `char`;
- class names cut short;
- `MonitorFromWindow` ignoring position.

Sprint 058 found seven more in Sprint 055's processes code. KIT_DIARY #47 names the cause: "a probe that copies the program's inputs proves the implementation on those inputs only". Three of the last eight sprints (056, 057, 058) repaired or re-proved accepted work.

The independent judge was chosen and is unused. Decision 2026-10-10 point (3) made the game's own Windows calls "a judge the family writer has not seen". The capture exists: `store/game-runs/38113342549/jit_capture.bin.trace.tsv` holds 3,495 calls with arguments and returns. `work_list.py` reads it only to order families, and the frontier record still writes `trace_sha256=EMPTY, guest_line_count=0`.

### G3 — The host is becoming the Windows kernel, in Python

Decision 2026-10-09, runtime architecture item (3): "Faults, VEH dispatch, TLS, handles, waits, thread suspend and context access never leave the VM." Three separate decisions have since moved much of this into the Python host:
- the scheduler is `src/w2m/scheduler.py`, reached by `hvc #12` (Decision 2026-10-11, one vCPU: "the host switches guest threads");
- objects shared between processes live in `src/w2m/kernel.py`, reached by HOST_KERNEL (Decision 2026-10-11, second process);
- VirtualAlloc and its siblings are host requests 19–22.

Each step is a sound prototype choice. No Decision supersedes item (3) as a whole, though, and nothing measures what it does: every blocking wait and every thread switch is an exit to Python over the pipe (10.3–10.5 µs, Sprint 048) plus the Python work behind it. Plan row 063 measures the frontier's model-step share. No row measures its host share, although `vm.host_requests` already counts requests by op.

## 3. The work so far

### W1 — What stands

- **Frontier:** 2,522,889 instructions (Sprint 047, 2026-10-10), then 57,411,545 (Sprint 058). Sprint 059 runs past RVA 0x143a6ca and stops at the appmodel API set, batch trace seq 1768.
- **Guest runtime:** 60 contracts; 44 C files, 7,374 lines.
- **CPU:** CPUID profile x86-64-v3, with 131 VEX and BMI classes judged on the Xeon 8573C (runs 38117654018, 38118493713).
- **CI:** 38195353233 is green on 4a27afe9, and the latest run is green on bae2a4da.

### W2 — The feasibility review's findings

| Finding | Status |
|---|---|
| F1 (exit cost) | Measured and restated 2026-10-10. |
| F3 (replay label) | Relabelled 2026-10-10. |
| F4 (code cache) | 64 MB, but a full cache still raises `CompileError("code cache full")` (`vm_backend.py` line 2081); nothing flushes it. |
| F5 (heap) | Unchanged first-fit list, no split, no coalescing. With one vCPU, and switches only at translated-block entries, the missing lock does not bite yet; mono's 987,170 HeapAlloc calls in 180 s still go through it. |
| F2 (shown rows) | Open (G2). |

### W3 — The frontier now runs for minutes, and the deferral that waited on that has not moved

The six latest frontier records took 131.4–134.7 s each (`frontier-643a6283510c-1791756604995655000` through `…1791757748625534000`); earlier the same day runs took 2.1–6.3 s. Decision 2026-10-10 put off snapshots at the stop and a persistent translation cache "until a frontier run takes minutes (today 5.8 s)". That condition is met. Each rerun replays 57 million instructions and lifts every block again in Python.

## 4. SDD adherence

### K1 — A user decision deferred by the agent

Decision 2026-10-10 (user): "From Sprint 049 on … (2) independent families run in parallel, one agent and one git worktree per family." Deferred entry 2026-10-11: "Sprint 049 ran its families one after another … Re-visit when a sprint opens three or more independent families at once." That deferral sits in `## Deferred`, not in Decisions with grounds. Sprint 054 then opened six families (version, display, windows, time, registry, file_search) and ran them in one session between 13:38 and 14:08; the deferral was not revisited. Sprint 060's plan row lists more than twenty functions across several families. `git worktree list` shows one worktree. AGENTS.md: "a user decision supersedes any agent-resolved one", and no decision is silent. This was neither overruled nor carried out.

### K2 — Vocabulary bent instead of grown

The frontier reports a lift refusal (Sprint 049, CVTDQ2PS) and an unresolved API set (Sprint 059) as EXECUTION_COMPLETED `faulted`, with the stop written into `unobserved`. Vocabulary 0.4 (P13) defines that field as the place to name what a run did not observe. The Drift watchlist set its own trigger: "if two sprints plan from a lift-refusal stop, propose a tag". Sprint 050 was planned from 049's refusal, and Sprint 060 from 059's API-set stop. Sprint 059's Rubber Duck Pass recorded "no new tag". Hard rule 2 requires a typed proposal here. The vocabulary has been at 0.7 since Sprint 039.

### K3 — The working agreement is stale (hard rule 7)

`WORKING_AGREEMENT.md` is stale in four ways:
- **Vocabulary version.** It names vocabulary 0.5 in three places: the registry row, the vocabulary location and the `json.load(open('signals/0.5.json'))` build command. `signals/current.json` says 0.7.
- **Substrate pin.** The permitted-dependency line pins `substrate-kernel[openai-compat]==1.1.1`. `pyproject.toml` installs the editable clone on branch `w2m` at 3c0542a.
- **Missing registry rows.** The canonical home registry has no row for `kernel.py`, `scheduler.py`, `file_bridge.py`, `family_run.py`, `work_list.py`, `specs/work_list.json` or `store/game-runs/`. These are the modules most sprints since 048 touch.
- **Reference runs.** The observation environment still calls reference runs optional. The build section requires green CI on every closing commit.

The plan is stale in the same way:
- the §6.3 row puts the function pool at "045", which is the transport;
- the rows "— exceptions and threads (was 047)" and "— memory and files (was 048)" stand unnumbered after Sprints 051 and 053 did most of that work.

### K4 — What the kit is doing right here

Every sprint since 046 closes on green CI on its closing commit. Rubber Duck Passes carry real observations with real dispositions (059: an empty environment counted as two NULs, resolved). Planted faults name their reach. Sprint 057 found six plants whose anchor text had gone and gated the anchors. KIT_DIARY #42–#49 are genuine findings about evidence, among them #44 (same inputs for a reference trace) and #49 (a plant patched into a module the run had copied names from). These are the parts of SDD that catch defects, and they caught them.

## 5. Substrate

### S1 — Records, replay and fan-out are used correctly

`run_with_provenance` stamps both commits into every baseline. 7,048 records under `store/runs/` pass the integrity check. The family topology fans out one COMPARISON_COMPLETED producer per runner binary on CANDIDATE_GENERATED and fans in to one acceptance (`family_run.py`, topology). That is the shape substrate is built for.

### S2 — The orchestration lives outside the log

Substrate's README: "Every event and every runtime decision lands on the log … Nothing consequential lives in memory or in control flow." v0.2 §11.4: the harness "starts and coordinates workers … The record preserves why work started". In w2m, the reason each family started is a BLACKBOARD Decision written in prose. The loop from frontier stop to work list to family to frontier rerun runs by hand in the agent's session.

Decision 2026-10-09 (breadth work) chose otherwise: "the frontier run is one topology that starts item runs and resumes on their acceptances". `run_frontier.py` still has three producers (harness, runner, closer) and no trigger on SERVICE_UNIMPLEMENTED.

Across `src/` and `scripts/`:
- **Routes:** 0 (`TopologyBuilder.route`).
- **Composition:** 0 uses of `embedded_substrate`, the primitive for one run that starts others.
- **Predicates:** 23 of 43 are `lambda c: True`, so most topologies are fixed chains, not conditions over the log.

### S3 — No repair path on the record

In `family_run.py` the `implementation_source` producer emits CANDIDATE_GENERATED for the C file already on disk. A rejection ends the run, and the repair happens in the agent's editor before a new run. v0.2 §11.3 puts implementation repair and contract revision on the record as two separate paths. Substrate's `route` exists for exactly this: `docs/api.md` line 281 gives "carrying context — e.g. a failure reason — forward into the Producer it starts".

### S4 — `first_divergence` is never called

The name appears in w2m only as an event field. Substrate's function is never called, although Decision 2026-10-10 named it for comparing w2m's call stream with Windows's. A 131-second frontier run leaves 15 events and no call stream. The guest's call ring exists (`RING_SIZE`, `vm_backend.py` line 102), and the Windows side's calls are on disk (G2).

## 6. What to change

1. **Build the frontier topology Decision 2026-10-09 chose.** A trigger on SERVICE_UNIMPLEMENTED starts the next family's item run from `specs/work_list.json` through `embedded_substrate`. A rejection routes its first divergence into a repair producer. Acceptances resume the frontier. The reason each piece of work started then lands on the log (S2, S3).
2. **Record the frontier's call stream** and compare it, with arguments and returns, against `jit_capture.bin.trace.tsv` through `first_divergence`. Accept a family only when the game's own calls agree as well as the probe's rows. This also closes F2 (G2, S4).
3. **Carry out the user decision of 2026-10-10:** run families in parallel worktrees from Sprint 060, or record why not as a Decision with grounds (K1).
4. **Snapshot at the stop and a persistent translation cache**, since the condition in Decision 2026-10-10 is met (W3).
5. **Write a typed vocabulary proposal** for the frontier's stop kinds (lift refusal, unresolved API set), and stop using `unobserved` to carry them (K2).
6. **Write one Decision that states the architecture as built** — waits, scheduling and shared objects in the host — superseding item (3) of 2026-10-09. Then record host time by op in each frontier record (G3).
7. **Bring the documents up to date.** Refresh `WORKING_AGREEMENT.md` (vocabulary, substrate pin, registry rows). Clean the plan's stale rows. Add a plan row for the §12.1 product slice and the `w2m` command (K3, G1).

## Sources

- Repository at bae2a4da: `docs/plan.md`; `WORKING_AGREEMENT.md`; BLACKBOARD Decisions 2026-10-09 (runtime architecture, threading, breadth work), 2026-10-10 (user: work list; F1; feasibility dispositions), 2026-10-11 (scheduler, second process); BLACKBOARD Deferred 2026-10-11; Drift watchlist 2026-10-11; Sprint tail 056–059; KIT_DIARY #42–#49; `scripts/run_frontier.py`; `scripts/family_run.py` lines 1–12 and its topology; `scripts/work_list.py`; `src/w2m/kernel.py` lines 1–35; `src/w2m/vm_backend.py` lines 102, 166, 565, 2081; `tools/guest_runtime/threads.c` lines 17–18; `tools/guest_runtime/heap.c`; `signals/current.json`; `pyproject.toml`.
- Records: `store/runs/frontier-643a6283510c-*` (elapsed_seconds); `store/game-runs/38113342549/jit_capture.bin.trace.tsv`; `store/game-runs/38006665419/jit_capture.bin.imports.tsv`.
- CI: runs 38195353233 (success, 4a27afe9) and the latest `ci` run (success, bae2a4da).
- `windows_to_macos_research_direction_v0_2.md` §1, §4.3, §11.2, §11.3, §11.4, §12.1.
- substrate (`./substrate` at 3c0542a): `README.md` ("How it works"); `docs/api.md` lines 281 (route), 490–525 (embedded_substrate), 755–790 (replay).
- `sdd-kit-3/AGENTS.md`: Decisions protocol, hard rules 2 and 7, Rubber Duck Pass.
