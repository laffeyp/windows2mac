# Sprint 149 — The case runner bound: no vacuous pass, a pinned fixture, and link 3's revisit measured

```yaml
---
id: 149
status: active
phase: 6
pass_kind: functional
---
```

## scope

The large review of link 3 (reviewer-2, 2026-10-25) found two blocking faults and five medium ones. This sprint
fixes the two blocking ones and four of the medium ones, before link 4 begins. Medium 5 (no emit site) is Sprint
150's.

- **Blocking 1, a vacuous pass.** `tests/case_runner.rs` returns passing with no clang, no results, or no fixture, and
  without the fixture it falls back to the C emitter, which Sprint 145 measured compiles none of the cases. Fixed:
  - the C emitter path is removed from the test; a missing fixture fails;
  - on the fixture path a refusal fails;
  - each run's counts (cases, judged whole, Windows faults, faults judged, model, undefined memory) are committed in
    `store/case_runner/expected.tsv` and asserted exactly;
  - `scripts/gates.sh` sets `W2M_REQUIRE_CASE_RUNS=1`, under which missing inputs fail rather than skip;
  - each close names the judges CI does not run (CI is Linux; the case runner needs Hypervisor.framework and the
    downloaded results).
- **Blocking 2, the revisit not measured.** The design's revisit condition for link 3: "after the case runner, the
  cases that can run at step 3 reach fewer than half the 254 rule files: then the translator's judges are too thin to
  port the rules in groups, and the judges come first". The test maps the iclasses of the cases judged whole to rule
  modules through the frozen registry's map (`store/rules/registry.tsv`, 460 iclasses to 249 modules, made by running
  `w2m.rules.registry()`), reports the modules reached out of 249 (the public modules; 254 counts `__init__.py` and the
  four shared `_` modules too), and the condition is applied in the result.
- **Medium 1, the fixture's layout.** The fixture's words embed the runtime's symbols and the process area's
  addresses, not only the block base and exit. The fixture gains a header (base, exit, slice stub, the sha256 of the
  runtime symbol table as `name=address` lines sorted, the commit of `src/w2m`), the test asserts each against the
  Rust guest, and the fixture's sha256 is committed in `store/case_runner/python-words.sha256`. The generator command
  is named in `WORKING_AGREEMENT.md`; it runs the frozen Python inline.
- **Medium 2, the arena off CI.** `VIRTUAL_VA` and `VIRTUAL_SIZE` move to an ungated module, and
  `tests/virtual_memory.rs` loses its macOS gate, so CI runs the arena's replay.
- **Medium 3, a claim and two counts.** The tail entry corrects Sprint 148's "judges everything vm_oracle judges"
  (the 116,718 model cases are not judged), and gives the distinct count (set 039 runs twice) and each set's judged
  share.
- **Medium 4, the give-back.** `_unrun_after` and the give-back become one function in `run.rs` with the code-cache
  guard and the count's write (vm_backend 5198-5208), from the block's tails (`tail_hosts`, `tail_left`), used by
  `run_cases`.
- **Low.** KIT_DIARY gains the three lessons of Sprint 145; this card is committed alone before its code.

## prerequisites

- 148 (closed)

## context_files

- reviewer-2's link-3 review (2026-10-25); `docs/design-host-compile-path-2026-10-24.md` 168, 181-185
- `src/w2m/vm_backend.py` 5195-5208, `_unrun_after`, `_in_code_cache`, the block assembly's tails;
  `src/w2m/rules/__init__.py` (read in full)
- `crates/w2m_vm` (`cases.rs`, `run.rs`, `guest.rs`, `lib.rs`, `tests/case_runner.rs`, `tests/virtual_memory.rs`),
  `scripts/gates.sh`, `KIT_DIARY.md`

## signal contract

### Emits

None this sprint; Sprint 150 binds the runner's verdicts to records (review Medium 5).

### Consumes

The case files, the downloaded Windows results, the fixture.

### Invariants

- No Python file added or changed.

## artifact contract

### Files created

- `store/rules/registry.tsv`, `store/case_runner/expected.tsv`, `store/case_runner/python-words.sha256`

### Files modified

- `crates/w2m_vm/` (`cases.rs`, `run.rs`, `guest.rs`, `lib.rs`, `tests/case_runner.rs`, `tests/virtual_memory.rs`),
  `scripts/gates.sh`, `WORKING_AGREEMENT.md`, `KIT_DIARY.md`, `BLACKBOARD.md` (Sprint 148's CI run, in the tail entry)

### Content assertions

- Each function cites the Python lines it carries.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the case runner's counts per run equal `expected.tsv` (from the run of 2026-10-25, before this
  sprint's changes), zero difference classes, zero refusals; the arena's replay runs without the macOS gate.
- **Planted faults:**
  - the fixture moved aside: the gated test fails, naming the fixture;
  - one run's expected judged count changed by one: the test names that run;
  - one header field changed (the exit address): the test refuses the fixture.
- **The revisit:** the modules reached, out of 249, against the condition's half (125).

## done criteria

The case runner cannot pass in the gates without judging the committed counts, its fixture is pinned to the layout it
was made for, and link 3's revisit condition is measured and applied.

## self-review checklist

- [x] Scope names verifiable properties, each from a review finding.
- [x] context_files cite the Python carried.
- [x] Emits: none this sprint, with the sprint that adds them named.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values, planted faults, the revisit's measure.
- [x] One concept: the case runner's judges made binding.

## result
