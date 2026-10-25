# Sprint 129 — The plan reaches a frame

```yaml
---
id: 129
status: done
phase: 6
pass_kind: docs
---
```

## scope

Fixes (c) and (d) of the direction of 2026-10-24 (BLACKBOARD Decision 2026-10-24 user), and the first document the
design check (Sprint 128) judges. `docs/rust-port-plan-2026-10-24.md` replaces `docs/rust-port-plan-2026-10-21.md` as
the order of work. It states its endpoint (milestone B, then the first scene) and holds every step to the goal's
figures: 37,600 system calls a second, 41 million instructions a frame over six threads, the first scene at 325.4
billion instructions and 62 s. It names the systems that solved each problem the port faces, with sources. Its order:
the host redesigned; vocabulary 0.10; a walking skeleton on the existing C guest runtime, with each judge moving to
Rust in the sprint whose code it judges; one vCPU to the batch run; the PE images ahead of time; several vCPUs; the NT
edge inside the VM; milestone B as three steps (the window on AppKit's main thread, winemetal served by the host, the
first frame and first scene); play measured; and only then the C runtime and the in-VM compiler ported. The
superseded plan and `docs/plan-2026-10-09.md` point to it.

## prerequisites

- 128 (closed)

## context_files

- `docs/rust-port-plan-2026-10-21.md`, `docs/design-rust-host-2026-10-23.md` (what is replaced, and why)
- `docs/plan-2026-10-09.md` (goal, milestones, the 37,600), `docs/recentering-2026-10-14.md` 35-36,
  `docs/solution-2026-10-16.md` 4.3 to 4.9 and 5 (the NT edge, every system call inside the VM),
  `docs/milestone-playable-slice-2026-10-13.md` 53-84 (DXMT and winemetal's seam), `docs/postmortem-2026-10-21.md`
- BLACKBOARD Decisions 2026-10-14 (the ahead-of-time cache's scope), 2026-10-15 (the in-VM compiler's revisit
  condition), 2026-10-16 user (Microsoft's DLLs), 2026-10-24 user
- primary sources for QEMU, crosvm, Firecracker, Wine, Rosetta 2 and FEX, quoted in the plan

## signal contract

### Emits

None. A document; it puts nothing on a record.

### Consumes

The documents and Decisions above.

### Invariants

- The superseded plan stays in place as history (no file deleted); only a pointer line is added to its head.
- No Python file is added to w2m.

## artifact contract

### Files created

- `docs/rust-port-plan-2026-10-24.md`

### Files modified

- `docs/rust-port-plan-2026-10-21.md`, `docs/plan-2026-10-09.md` (a pointer line each); `BLACKBOARD.md`;
  `crates/design_check/src/main.rs` (found by the planted fault, below)

### Content assertions

- The plan has `## Against the goal` and `## Against the systems that solved it`, and every figure it states cites
  the document and line it comes from, or says it is derived.
- Its order puts several vCPUs and the NT edge before milestone B, milestone B as three steps of its own, and the
  ports of the C guest runtime and the in-VM compiler after it.

### Command exit codes

- `target/release/design_check` returns 0 with the plan in `docs/` (1 document checked)
- `scripts/gates.sh` returns 0

## observation contract

- **Expected value:** `design_check` reports 1 plan or design checked, and passes.
- **Planted fault:** the plan with its Rosetta 2 source removed fails `design_check`, naming Rosetta 2.

## done criteria

The order of work ends at a frame and the first scene, and each step names the goal's figure it moves.

## self-review checklist

- [x] Scope names one verifiable property: a plan that reaches milestone B and passes the design check.
- [x] context_files name every document a figure or step comes from.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command and by reading the order.
- [x] Observation contract: the check's expected value and one planted fault.
- [x] One concept: the order of work.

## result

- `docs/rust-port-plan-2026-10-24.md`: the order of work, ten steps to milestone B, the first scene and play
  measured, then the ports and Microsoft's DLLs off the critical path. Its figures cite their documents and lines;
  the 6,100 crossings a second is marked derived. The two older plans point to it. BLACKBOARD Decision 2026-10-24
  agent-resolved, marked `Architecture:`, names it, and `design_check` reads that pointer.
- The reviewer's two passes changed the draft: Microsoft's DLLs over a w2m kernel moved from before milestone B to
  step 10, by Decision 2026-10-16 user ("off the critical path"); the play path to the first DXGI or Direct3D 11 call
  became a step; DXMT's imports, and msvcrt's and ucrtbase's own, are listed before choosing how to serve them; two
  figures were corrected (milestone A at Sprint 068; Sprint 108's 2,095,145,119 instructions a second, where the
  draft cited a figure no record holds).
- Expected value: `design_check` reports 1 plan or design and passes.
- Planted fault: the Rosetta 2 source removed. The first run passed it, because Rosetta 2 shares its bullet with FEX,
  whose URL counted for both. The check now requires a source after each system's name and before the next system
  named in the item (`sourced`, a unit test); the planted plan then fails, naming Rosetta 2.
- Tests: design_check 14 (6 unit, 8 planted).
