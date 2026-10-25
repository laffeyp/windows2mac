# Sprint 139 — The host's compile path, designed before the skeleton runs a guest

```yaml
---
id: 139
status: done
phase: 6
pass_kind: architecture
---
```

## scope

The walking skeleton (step 3 of `docs/rust-port-plan-2026-10-24.md`, lines 114-120) serves a `tests/data/timed_waits`
guest's exits, and a dispatch miss the C compiler in the VM declines reaches the host as `hvc #1`. Measured after
Sprint 138 on the Python host: `sleeper` sent 5 blocks to the host's compiler, `qpc` 2 and `twoready` 10 (the C
compiler counting "not a code page" 9, 4 and 9, "no stencil" 4, 2 and 10). The plan ports the translator in step 4,
after the skeleton. `docs/design-host-compile-path-2026-10-24.md` states the miss path against the goal's figures and
against QEMU, crosvm, Firecracker, Wine, Rosetta 2 and FEX, with sources; weighs three answers (the translator ported
before the skeleton's run loop; the frozen Python compiler called from the Rust host; the stencil table widened until
the skeleton's guests need no host compile); and decides one, with the plan amended to match. No code.

Amended after the reviewer's small review, before the result: the measurement widened to all 14 guests; three more
answers weighed (a guest the table covers whole; the declined blocks' words as a fixture; the run loop first, to the
first decline), and the third chosen; the translator's judges named for the blocks the C emitter declines. The date
in the design's name follows the local clock (2026-10-24 PDT), not UTC.

## prerequisites

- 138 (closed)

## context_files

- `docs/rust-port-plan-2026-10-24.md` 114-126; `docs/design-rust-host-2026-10-24.md` (change 3)
- BLACKBOARD Decisions 2026-10-14 (copy-and-patch; the stencil key) and 2026-10-15 (thin samples; the inferred
  compiler put in the VM, its Revisit-if)
- `tools/vm_compiler/vm_compiler.c` 216-270 (`lookup`, `vmc_decode`, `vmc_emit`); `src/w2m/vm_backend.py` 1417
  (`compile_insn`), 4588 (`_compile`)
- primary sources: QEMU `docs/devel/tcg.rst`; FEX `Dispatcher.cpp`; the FEX wiki's ARM64EC page; WWDC20 10686;
  crosvm `ARCHITECTURE.md`; Firecracker `docs/design.md`

## signal contract

### Emits

None. A document.

### Consumes

The measurement above (the Python host on three guests, one vCPU, 2026-10-24; widened to all 14 after the review).

### Invariants

- No code changes. No Python file added or changed.

## artifact contract

### Files created

- `docs/design-host-compile-path-2026-10-24.md`

### Files modified

- `docs/rust-port-plan-2026-10-24.md` (the steps' order, if the decision moves a step), `BLACKBOARD.md` (the
  Decision, marked `Architecture:`; Sprint 138's CI run)

### Content assertions

- The design has "## Against the goal" and "## Against the systems that solved it", each of the six systems sourced.

### Command exit codes

- `target/release/design_check` returns 0; `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `design_check` passes the design and the amended plan; the Decision names the design.
- **Planted fault:** the design with FEX's source removed: `design_check` names FEX.

## done criteria

The miss path the skeleton needs is decided against the goal and the systems that solved it, and the plan says
where it is built.

## self-review checklist

- [x] Scope names one verifiable property: the miss path decided, checked and in the plan.
- [x] context_files cite the measurement, the Decisions and the sources.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values and a planted fault.
- [x] One concept: the host's compile path.

## result

- `docs/design-host-compile-path-2026-10-24.md`: the measurement of all 14 `timed_waits` guests on the Python host
  (2 to 54 host compiles each, 22 to 56% of compiled blocks), the three places the C compiler declines, six answers
  and a seventh (QEMU's one translator) weighed against the goal and the six systems, each with its own source; the
  decision: the run loop first, stopping `qpc` by name at its first declined block, then the translator's parts, each
  judged by the C emitter's words where it has a stencil and by its classes' cases through the run loop. The plan's
  steps 3 and 4 amended to match; the Decision marked `Architecture:` with project, SDD and practice grounds.
- Expected values: `design_check` passes 3 plans and designs; the Decision names the design; `scripts/gates.sh`
  exit 0 on the final tree.
- Planted fault: both FEX URLs removed: "names FEX with no source", exit 1; restored, exit 0. The first plant's
  pattern missed and the check passed with FEX still sourced; the plant was checked by a count of FEX URLs (0) and run
  again. The first draft failed the check itself: its Wine item cited only FEX's wiki, which the check took as FEX's
  source; Wine 9.0's release notes became Wine's.
- Review: the reviewer's small review found the first draft's judge blind to the blocks that motivate it, the
  skeleton's first run pushed behind five or more sprints, the crosvm sentence false by its own source, the "not a
  code page" declines misread (they are import thunks the bridges answer), no SDD ground, and no provenance for the
  measurement. Each is answered in the design or the Decision. One of its figures did not hold: it gave 3,365 of
  4,166 stencils as guarded; counted from `stencil_table.h`, 3,564 of 21,239 key choices carry guards, touching 1,227
  of the 4,166 stencils.
- Re-review: every finding answered; the reviewer confirmed the guard count (its 3,365 came from Sprint 079's tail,
  an older table). One medium finding changed the design: the class cases need a case runner (lockstep, a case's
  state in and out, the fault path; `scripts/vm_oracle.py` 184, 236, 243-246) that the first link does not build, so
  the case runner is a link of its own after the decoder, and the model's classes (`hvc #10`, `#11`) wait for step 4.
  Its other medium: the inline `python -c` wrote Python though no file; the user's Decision of 2026-10-23 reads "no
  new Python is written", so the question goes to the user, and until answered further oracle counts go through
  existing entry points only.
- The measurement ran through an inline `python -c` that wraps `VM._compile`; no file was added. The frozen host is
  the only thing that can say how many blocks it compiles, and its counts are in the design's table.
