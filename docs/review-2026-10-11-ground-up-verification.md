# Verification 2026-10-11 — the ground-up review

Each factual claim of `docs/review-2026-10-11-ground-up.md` checked against the repository at bae2a4da, with the
command or file that decided it.

## Missed by the review

- **Sprint 058's and 059's frontier runs are not in `store/`.** `scripts/run_frontier.py:53` defaults `--runs` to
  `ROOT/runs`, and `.gitignore:17` ignores `/runs/`. `runs/` holds 500 records, 44 of them frontier runs; Sprint 059's
  stop is `runs/frontier-643a6283510c-1791770815935550000`. The newest frontier record in `store/runs/`
  (`…1791757748625534000`) is Sprint 055's, stopped at GetCurrentDirectoryA after 57,411,545 instructions, and Sprint
  059's card names it as its start (`sprints/sprint-059-process-parameters.md:43`). The review's W3 and S4 figures
  describe 055's run.
- **15 of the 20 frontier records in `store/runs/` name an uncommitted tree** (`w2m_commit` ending `-dirty`); all six
  131-second runs are `d73a8eba…-dirty`, and Sprint 059's run in `runs/` is `cfc90ece-dirty`. S1's "stamps both
  commits into every baseline" holds, but a dirty stamp does not name the code that ran.
- **The API-set stop has a tag.** Sprint 059's run reported it as EXECUTION_COMPLETED `faulted` with the reason in
  `unobserved` and emitted no SERVICE_UNIMPLEMENTED, the tag for a service w2m does not serve, which the frontier
  topology already subscribes to. K2 then needs a proposal for the lift refusal only.

## Corrected

| Claim | Finding |
|---|---|
| W1: 44 C files, 7,374 lines | The 44 C files hold 7,060 lines; 7,374 counts the headers too. |
| W2 F4: nothing flushes the code cache | `flush_translations` exists (`vm_backend.py:1084`); only `scripts/vm_oracle.py:206` calls it. A full cache in a frontier run still raises CompileError. |
| W3: earlier the same day runs took 2.1–6.3 s | The store also holds runs of 62.0–66.0 s, the first at 02:14 PDT on 2026-10-11, about 13 hours before the 131-second runs; the deferral's condition was met then. |
| S2: 23 of 43 predicates are `lambda c: True` | 25 of 43. |
| S3: no repair path on the record | `family_run.py:2635–2643` emits DISCREPANCY_REPORTED and DISCREPANCY_ROUTED on a rejection, with `repair_path` fixed at `implementation_repair`; no producer acts on it. `src/w2m/differential.py:240–275` has a router and resolver on triggers for the CPU half. |
| K2: the watchlist trigger is met | Only Sprint 050 planned from a lift refusal (049's). Sprint 060's plan row comes from an API-set stop. |

## Holds

- **G1:** no `[project.scripts]` in `pyproject.toml`, no command-line module in `src/w2m/`, no plan row for
  `w2m steam run`.
- **G2:** the Sprint 054 review's eight defects (Sprint 056's table) and KIT_DIARY #47 as quoted; `trace.tsv` holds
  3,495 calls; `run_frontier.py:108` writes `trace_sha256=EMPTY, guest_line_count=0`; F2 open (BLACKBOARD line 123).
- **G3:** "never leave the VM" at BLACKBOARD line 88; `HVC_SCHEDULE = 12`, `HOST_KERNEL = 33`, host requests 19–22 are
  VirtualAlloc, VirtualFree, VirtualProtect and VirtualQuery; 10.3–10.5 µs through the pipe (Sprint 048);
  `vm.host_requests` counts by op (`vm_backend.py:1022`); no plan row measures the host share.
- **K1:** the Deferred entry as quoted; Sprint 054's commits run 13:33–14:08 for six families; `git worktree list`
  shows one.
- **K3:** WORKING_AGREEMENT names vocabulary 0.5 at lines 30, 95 and 103 and pins `==1.1.1` at line 117; no registry
  rows for `kernel.py`, `scheduler.py`, `file_bridge.py`, `family_run.py`, `work_list.py`; plan lines 59–60 unnumbered.
- **S2:** 0 `route` calls, 0 `embedded_substrate` uses; substrate README line 47 as quoted.
- **S4:** substrate's `first_divergence` (`substrate/projections/inspect.py:217`) is never called; w2m uses the name as
  an event field only; a frontier record holds 15 events.

## Order of work

1. Frontier records in `store/runs/` from a committed tree; the API-set stop as SERVICE_UNIMPLEMENTED.
2. The frontier's call stream compared with `store/game-runs/38113342549/jit_capture.bin.trace.tsv` through
   `first_divergence` (the review's item 2; closes F2).
3. A snapshot at the stop and a persistent translation cache (item 4).
4. A Decision on parallel worktrees with grounds (K1); a vocabulary proposal for the lift refusal (K2); one Decision
   for the host-side architecture as built (G3); WORKING_AGREEMENT and the plan brought current, with a row for the
   §12.1 slice (K3, G1).
5. The frontier topology through `embedded_substrate` and routed repair (item 1), after the call-stream judge exists.
