# Sprint 230 — The family row rules

```yaml
---
id: 230
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 214" as annotated at Sprint 222 (reviewer-3's Medium 1 of its refreshed project
review and Medium 2 of its review at 222), and plan step 4's gate item "68 of 68 families".

- **Where it stands, by lookup (2026-10-30):**
  - Of the family index's 273 runs, 143 conform on Rust. 80 end at ExitProcess and are held in
    `store/family_runs/sweep-held-2026-10-29.tsv` only by their row rule's name and the sha256 of their printed rows.
    51 stop before the end.
  - `family_runs.rs` carries two rules, `raw` and `module_names_ansi_rows`, and only with no row excluded.
  - The 80 wait on 23 rules, each a function or lambda of `scripts/family_run.py` at the tag
    frozen-python-2026-10-29, read with `git show`:
    - `loader_rows` 16 runs (218-228), `code_page_rows` 16 (163-165), `process_query_rows` 5 (143-153), `time_rows`
      4 (399-408);
    - two runs each: `virtual_memory_rows` (231-239), `version_rows` (242-246), `topology_rows` (443-458),
      `thread_stack_rows` (1929-1931), `temp_path_rows` (1733-1734), `slist_rows` (461-473), `processes_rows`
      (276-281), `paths_rows` (476-490), `parameters_rows` (284-308), `no_steps` (834-837), `integrity_level_rows`
      (1786-1787), `heap_query_rows` (2043-2046), `format_message_rows` (2217), `files_more_rows` (2082-2083),
      `file_mapping_rows` (2179-2180), `display_rows` (249-273), `console_rows` (156-160);
    - `thread_stacks_rows` 1 (1966-1969);
    - `raw` with rows excluded, 4 runs.
  - Each contract records its rule's name and description (`acceptance.derivation`), and `family_known` already reads
    each run's contract and version from its family record (Sprint 221) and its rule from the contract.
  - An excluded row is not among its contract's acceptance rows (checked for `windows.process.queries`,
    `windows.file.attributes` and `windows.memory.local_free`), and the test compares the contract's rows only, so a
    rule with exclusions judges as the same rule without them.
- **The work:**
  - The 23 rules, ported into `family_runs.rs`'s `derive` as the frozen code writes them, each with its source lines
    cited. A rule is chosen by the name in the lookup's column 8; `time_rows`' start by the family
    (`time_and_error_mode_child` keeps only `error_mode.child_inherited`).
  - An unknown rule name fails the test, naming it; no run is held by a rule's name alone.
  - Each of the 80 then conforms or is held by its first differing row: "ended; row <s> differs from
    <contract>@<version>".
- **If the Python was wrong, its wrong parts are not reproduced:** a row the contract rejects on Rust is read against
  the frozen record's verdict and the Windows runs before it is called a Rust defect.
- **Not here:** `windows_rows` and `child_rows`, whose runs stop before the end; the 51 stops.

## prerequisites

- 229 (closed)

## context_files

- `scripts/family_run.py` at the tag (each rule's lines above)
- `crates/w2m_vm/tests/family_runs.rs`; `store/family_runs/known-2026-10-29.tsv`,
  `store/family_runs/sweep-held-2026-10-29.tsv`; `contracts/windows.*/` (each run's version);
  `docs/step-4-census-2026-10-28.md` (row 6)

## signal contract

### Emits

- None: the families' runs write no record on the Rust host (Deferred, 2026-10-17); the held file and census row 6
  are the artifacts.

### Invariants

- The 143 runs that conform still conform; the 51 stops are unchanged.

## artifact contract

### Files modified

- `crates/w2m_vm/tests/family_runs.rs`, `store/family_runs/sweep-held-2026-10-29.tsv`,
  `docs/step-4-census-2026-10-28.md` (row 6)

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** of the 80, how many conform, and each other with its first differing row; runs conforming of
  273, against 143; families conforming in every run, against 32.
- **Planted faults:**
  1. `loader_rows`' drop of `apphelp.dll` removed: no loader run changes (none prints it), and the rule's unit test,
     a row naming it, fails.
  2. `code_page_rows` applied under another rule's name: the code-page runs fail, naming a `bestfit.<u>` row.
  3. An unknown rule name in a scratch copy of the lookup: the test names the rule.

## done criteria

Every family run that ends is judged against its contract version's rows under its own row rule on Rust. None is held
by a rule's name.

## self-review checklist

- [x] Scope names one verifiable property: every ended family run judged under its rule.
- [x] context_files cite the rules' lines, the test, the lookup and the contracts.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the families' row rules.

## result
