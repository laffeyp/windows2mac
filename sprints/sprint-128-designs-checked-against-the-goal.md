# Sprint 128 — Designs checked against the goal

```yaml
---
id: 128
status: done
phase: 6
pass_kind: architecture
---
```

## scope

Fix (a) of the direction of 2026-10-24 (BLACKBOARD Decision 2026-10-24 user): one check before every design or
decision lands. A Rust binary, `design_check`, reads every plan and design under `docs/` dated 2026-10-24 or later (a
file `design-*-YYYY-MM-DD.md`, or one whose name holds `plan-` and ends in a date) and fails unless it has two
sections. `## Against the goal` must state the goal's figures: 37,600 (system calls a second in play), 41 million
(instructions a frame), six threads, and the first scene. `## Against the systems that solved it` must name QEMU,
crosvm, Firecracker, Wine, Rosetta 2 and FEX, each on a line with a source (`http`). The gates and CI run it, so the
plan and the design that follow (Sprints 129 and 130) cannot land without both sections. The binary checks that the
figures and the sources are there; whether the argument holds is the reviewer's, as before.

## prerequisites

- 127 (closed)

## context_files

- `BLACKBOARD.md` Decision 2026-10-24 user (the re-review and its five fixes)
- `docs/plan-2026-10-09.md` (the goal, milestones A to C, 37,600 a second); `docs/recentering-2026-10-14.md` 35-36
  (41 million a frame, six busy threads, the first scene at 62 s)
- `crates/signal_report/` (the pattern: a `--check` mode, `--root`, scratch-root tests)
- `scripts/gates.sh`, `.github/workflows/ci.yml`, `WORKING_AGREEMENT.md` (gate commands)

## signal contract

### Emits

None. The binary reads Markdown and exits; it puts nothing on a record.

### Consumes

The plan and design documents under `docs/`.

### Invariants

- No Python file is added to w2m.
- A document dated before 2026-10-24 is not checked: the plans and designs before the check stay as history.

## artifact contract

### Files created

- `crates/design_check/` (the binary and its tests)

### Files modified

- `Cargo.toml` (member), `Cargo.lock`, `scripts/gates.sh`, `.github/workflows/ci.yml`, `WORKING_AGREEMENT.md`

### Content assertions

- `crates/design_check/src/main.rs` names the four figures and the six systems as constants, each cited to its
  source document or to the Decision.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `target/release/design_check` returns 0 on the tree (no document dated 2026-10-24 or later exists yet)
- `scripts/gates.sh` returns 0

## observation contract

- **Expected value:** a document with both sections, every figure and every system with a source, passes; on the
  tree the binary reports the number of documents checked.
- **Planted fault 1:** a plan dated 2026-10-24 with no `## Against the goal`: exit 1, naming the file and the section.
- **Planted fault 2:** a design that states every figure but 37,600: exit 1, naming 37,600.
- **Planted fault 3:** a design whose Rosetta 2 line has no source: exit 1, naming Rosetta 2.
- **Planted fault 4 (the exemption):** the same faulty design dated 2026-10-23 passes, and a review dated 2026-10-24
  (not a plan or design) passes.

## done criteria

No plan or design dated 2026-10-24 or later can pass the gates without stating the goal's figures and naming the
systems that solved the same problem, with sources.

## self-review checklist

- [x] Scope names one verifiable property: a plan or design that omits the goal's figures or the prior systems fails.
- [x] context_files name the Decision, the documents the figures come from, and the pattern followed.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: an expected value and four planted faults.
- [x] One concept: the check.

## result

- `crates/design_check` (no dependency) checks every file under `docs/` whose name begins `design-`, `solution-`,
  `recentering-` or `architecture-`, or holds `plan-`, except a closed list of the seven written before the check;
  each must have `## Against the goal` stating 37,600, 41 million, six threads and the first scene, and
  `## Against the systems that solved it` naming QEMU, crosvm, Firecracker, Wine, Rosetta 2 and FEX, each in an item
  (a list entry or a paragraph, its wrapped lines joined) that holds a source. Every `## Decisions` entry from
  2026-10-24 marked `Architecture:` must name a checked document that exists.
- Expected value: on the tree, 0 documents checked, exit 0; on the drafts of Sprints 129 and 130 in a scratch root,
  2 checked, exit 0.
- Planted faults, each exit 1 naming the file and what is missing: no goal section; a missing 37,600; Rosetta 2 with
  no source; a new design whose name carries an old date (2026-10-20); a marked Decision naming only history, and one
  naming a file that does not exist; a tree with no BLACKBOARD.md. History, a review and an unmarked Decision pass
  unchecked.
- Changed on the way, by the reviewer's small review and a failed first run:
  - the first version matched a system and its source on one physical line, and failed both drafts, whose bullets
    wrap the name and the URL onto different lines; the check now reads items;
  - the first version exempted documents by the date in their name, which the author writes; history is now a closed
    list;
  - the first version read only `design-` and `plan-` names, while fix (a) says "every design or decision": three more
    prefixes, and the `Architecture:` marker on Decisions.
- Tests: 13 (5 unit, 8 planted); the gates and CI run `target/release/design_check`.
