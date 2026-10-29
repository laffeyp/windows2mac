# Sprint 220 — Milestones D and E in the plan

```yaml
---
id: 220
status: active
phase: 6
pass_kind: functional
---
```

## scope

The user's Decision of 2026-10-29, "Milestones for the product slice, after milestone C". `docs/rust-port-plan-2026-10-24.md`
runs steps 1-10 to milestone C, and has no step for v0.2's product (§1, §4.1-4.3, §12.1). The Decision gives two
milestones after C, and this card writes them into the plan.

- **The plan's goal section** names D and E beside A, B and C.
- **Milestone D, the product slice:** `w2m steam run <appid>` from the Mac command line. Its step covers:
  - the depot acquired under the user's account, in Rust;
  - the drive and the registry kept across runs, where the file bridge removes its drive today (`file_bridge.rs` 573)
    and `registry.c` 30 keeps nothing;
  - the output a Mac application bundle (§4.3);
  - run, close and relaunch, and update and removal under the same lifecycle (§12.1).
  Its gate is §12.1's slice run end to end on this Mac.
- **Milestone E, the Steam session:** the guest `steamclient64.dll`'s 17 interface versions (the Decision lists
  them), generated from the Steamworks headers as `docs/solution-2026-10-16.md` §4.7 designs, forwarding to the Mac
  Steam client. Its gate is `SteamAPI_Init` succeeding and the session's calls conforming to a Windows run with Steam.
- **Each step's knowable facts** are named from the records, not left open: the 17 versions, the state's two homes,
  the seam step 8b builds.
- **The amendment is dated,** and `design_check` passes on the plan.

## prerequisites

- 219 (closed)

## context_files

- `docs/rust-port-plan-2026-10-24.md`; `windows_to_macos_research_direction_v0_2.md` §1, §4.1-4.3, §12.1;
  `docs/solution-2026-10-16.md` §4.7; BLACKBOARD Decision 2026-10-29 user "Milestones for the product slice"

## signal contract

### Emits

- None: a documents card. The judge is `design_check`, with `signal_report --check` for the bookkeeping.

### Invariants

- Steps 1-10 keep their text; the amendment adds steps and names, and changes no gate.

## artifact contract

### Files modified

- `docs/rust-port-plan-2026-10-24.md`, `docs/step-4-census-2026-10-28.md` if a row names the milestones

### Command exit codes

- `design_check`, `signal_report --check` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the plan names milestones A to E, and steps 11 and 12 with gates. Each step's facts cite the
  file and line they come from.
- **Planted faults:** (1) the amendment without its date: `design_check` names it; (2) step 11 without a gate:
  `design_check` names it, or the card's result says the check does not read gates and the plant reached no judge.

## done criteria

The plan carries milestones D and E after C, each with its step, its facts and its gate, as the user's Decision says.

## self-review checklist

- [x] Scope names one verifiable property: the plan reaches v0.2's product.
- [x] context_files cite the plan, v0.2's sections, the solution's Steam design and the Decision.
- [x] Emits: none, a documents card.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the plan's milestones past C.

## result

- **The plan:** `docs/rust-port-plan-2026-10-24.md` names milestones D and E in its goal section, in an amendment dated
  2026-10-29. Its order gains step 11 (milestone D, the product slice) and step 12 (milestone E, the Steam session),
  after step 10, each with its facts and its gate; steps 1 to 10 keep their text. Step 11 cites the state's two homes
  (`file_bridge.rs` 573, `registry.c` 30), SteamCMD's acquisition (WORKING_AGREEMENT 120) and v0.2 §4.3 and §12.1.
  Step 12 cites the bridge's design (`docs/solution-2026-10-16.md` §4.7) and the 17 interface versions, and makes its
  reference a laptop run with the Steam client, which the runner cannot make. Each names when it moves ahead of C.
- **The check:** before this card, `design_check` read the goal's figures and the systems' sources only.
  `amendment_faults` adds two rules. Each amendment marker (`*Amended `, `(amended `) names its date next. A step of
  `## Order` added by amendment states its `Gate:`. The unit test `an_amendment_is_dated_and_an_added_step_has_a_gate`
  holds both. Its first form read every "amended", and named a sentence in `docs/design-rust-host-2026-10-24.md` 21
  ("amended by the rows below"), which is no marker; it reads the two markers since. The tree's three checked
  documents pass.
- **Plant 1** (the goal section's amendment undated): before the check changed, `design_check` passed it, and the plant
  reached no judge. Run again on the new check: "docs/rust-port-plan-2026-10-24.md:23: an amendment with no date after
  it". Restored byte for byte.
- **Plant 2** (step 11's "Gate:" removed): before, it reached no judge. Run again: "docs/rust-port-plan-2026-10-24.md:
  \"## Order\" step 11, added by amendment, states no gate". Restored byte for byte.
- **Departure:** `crates/design_check/src/main.rs`, outside the card's file list. The plants found the gap, which
  Sprint 208 had met too (its plant 2), a rule recurring; so it goes into code here.
