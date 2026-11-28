# Sprint 378 — A frozen family held to a contract re-taken on the laptop

```yaml
---
id: 378
status: done
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Which Windows w2m answers as, the reference being the laptop" (Sprint 374) sets w2m to answer as the
laptop, and names tables still taken from GitHub's Server runner, `tools/guest_runtime/version.c` and `system_info.c`
first. Re-taking a table moves the families that read it (`version`, `system_info`, `files_more`): their judge,
`crates/w2m_vm/tests/family_runs.rs`, holds each run to the contract version in
`store/family_runs/known-2026-10-29.tsv`, and for a frozen family `crates/w2m_vm/src/bin/family_known.rs` (`contract`,
165-193) takes that version from the family record's first COMPARISON_COMPLETED, the runner's 0.1.0. A joined family
already moves by its row in `store/family_runs/joined-2026-11-27.tsv` (`named_sync_objects` at 0.2.0, Sprint 374); a
frozen family has no such route.

- **The route.** `store/family_runs/retaken-2026-11-27.tsv` (family, contract@version, the laptop run its rows come
  from), read by `family_known`: a family named there is held to that contract version, the rest as before.
  `known-2026-10-29.tsv` regenerated, its contract column alone changing, and only for families the new table names.
- **Its test.** A row in the new table moves that family's contract in `family_known`'s output and nothing else; a
  row naming a family the index lacks, or a contract file that does not exist, is refused naming it.
- **This card moves no family.** The new table ships empty of families, or with one only if a card's contract exists
  for it; the table re-takes are cards 379 (`version`: `version.c`), 380 (`system_info` and `files_more`:
  `system_info.c`, the system directory measured on the laptop), 381 (`system_messages.json`, `windows_exports.json`),
  382 (`recorded_folders.json`, the display calls). No API id is added.

## prerequisites

- 377 (closed)

## context_files

- `crates/w2m_vm/src/bin/family_known.rs`, `crates/w2m_vm/tests/family_runs.rs`.
- `store/family_runs/known-2026-10-29.tsv`, `store/family_runs/joined-2026-11-27.tsv` (their first lines).
- `grep -n "Which Windows w2m answers as\|The order after Sprint 377" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266.

## signal contract

### Emits

- None: the route changes which contract version a judge reads; no run is recorded.

### Invariants

- Every run as held; `known-2026-10-29.tsv` byte-equal to HEAD's while the new table names no family; G3 conforms;
  `crates/guest_runtime/src/api_ids.txt` unchanged.

## artifact contract

### Files created

- `store/family_runs/retaken-2026-11-27.tsv`

### Files modified

- `crates/w2m_vm/src/bin/family_known.rs`, with its tests and others only where the class requires, reported to main
  when found. One concept: the route by which a frozen family is held to a re-taken contract.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - `family_known`'s output byte-equal to `known-2026-10-29.tsv` with the new table empty.
  - A test fixture row moving one frozen family's contract column, and only that row.
- **Planted faults:**
  - (1) The route applied to every family: the known table's test fails naming a moved row.
  - (2) A table row naming a contract file that does not exist: `family_known` refuses it naming the file.

## done criteria

A frozen family can be held to a contract version re-taken on the laptop by one row of a table, and no family moves
until a card names it.

## self-review checklist

- [x] Scope names one verifiable property: the route, tested, moving nothing yet.
- [x] context_files cite the judge, the generator, the tables and the Decisions.
- [x] Emits: none, with the reason.
- [x] Artifact contract: one file created, one modified.
- [x] Observation contract: the unchanged output, the fixture; numbered plants.
- [x] One concept.

## result

- **The table.** `store/family_runs/retaken-2026-11-27.tsv`, new: one comment line naming its three columns (family,
  contract@version, the laptop run whose rows that contract holds), its reader and its refusals; no family. No card has
  re-taken a contract yet: 379 takes `version`, 380 `system_info` and `files_more`.
- **The route.** `crates/w2m_vm/src/bin/family_known.rs` 219-265, `RETAKEN` and `retaken`. Each row is checked against
  the frozen index's families and the contract each one's record names. A row is refused naming what it names wrongly:
  a family the index lacks ("no family of the frozen index", a joined family among them, since a joined family moves
  by its own row of `joined-2026-11-27.tsv`), a contract file that does not exist ("contracts/<id>/<version>.json does
  not exist"), a contract other than the record's ("re-taken under <id>, where its record names <contract>"), a run
  whose `store/probe-runs/<run>/profile.txt` does not begin "laptop-", or a family already named. `run` (306-318) now
  reads every frozen family's recorded contract first, then the table, and prints a named family's runs with the
  re-taken contract and the row rule that contract states (`rule`, unchanged). `family_known ROOT TABLE` reads a
  fixture's table in place of the stored one (doc 3 and 22-25). With the stored table empty,
  `store/family_runs/known-2026-10-29.tsv` is byte-equal to HEAD's (`git diff --quiet HEAD --` it, exit 0), and
  `the_stored_lookup_is_family_knowns_output` passes on it unchanged.
- **The tests** (`crates/w2m_vm/tests/family_runs.rs` 1300-1422, beside the stored-lookup test, Apple silicon only as it
  is). `a_retaken_row_moves_its_familys_contract_and_nothing_else` checks first that the stored table names no family.
  It then runs `family_known` with a fixture written under `CARGO_TARGET_TMPDIR`:
  `code_rewrite	windows.memory.code_rewrite@0.1.0	lap-20261014T044509652Z-627bbd99`. Every one of the 274 output
  lines is the stored lookup's except one, "code_rewrite run lap-20261014T044509652Z-627bbd99: columns [4] moved". The
  fixture goes back a version because every frozen record already names its contract's newest version: of its 273 rows, 44
  have a contract with several versions on disk, and all 44 name the newest. `code_rewrite` is a frozen family of one run, raw in
  both versions, so only the contract column can move. `a_retaken_row_is_refused_by_what_it_names` tries six fixtures:
  `no_such_family`, `named_sync_objects` (joined), `code_rewrite@0.9.0`, `windows.memory.virtual@0.1.0` for
  `code_rewrite`, run 38173733146 (a runner's), and a row given twice. Each exits 1, its stderr naming the fault.
  `cargo nextest run -p w2m_vm --test all -E 'test(/family_runs::(a_retaken|the_stored_lookup)/)'`: 3 passed.
- **Plant 1** "assertion `left == right` failed: the rows the fixture moves ... left: [\"code_pages run 38044061149:
  columns [4, 8] moved\", \"code_pages run 38044062619: columns [4, 8] moved\", ... right: [\"code_rewrite run
  lap-20261014T044509652Z-627bbd99: columns [4] moved\"]", `cargo nextest run -p w2m_vm --test all -E
  'test(/family_runs::(a_retaken|the_stored_lookup)/)'` exit 100
  (`a_retaken_row_moves_its_familys_contract_and_nothing_else` failed, the other two passed); live 03:56:28-03:56:42
  UTC:
  - backup `scratchpad/plants/378/family_known.rs.backup`; `run`'s `moved.get(family)` set by the Edit tool to
    `moved.values().next()`, the route applied to every family; landed (grep, line 318); restored by `cp`, `cmp` equal.
- **Plant 2** "family_known: /Users/peterlaffey/port/store/family_runs/retaken-2026-11-27.tsv: version's contract
  windows.version.version@0.2.0: contracts/windows.version.version/0.2.0.json does not exist",
  `target/release/family_known .` exit 1, `cargo nextest run -p w2m_vm --test all -E
  'test(/family_runs::the_stored_lookup/)'` exit 100; live 03:56:53-03:57:01 UTC:
  - backup `scratchpad/plants/378/retaken-2026-11-27.tsv.backup`; the row
    `version	windows.version.version@0.2.0	lap-20261013T231059422Z-136bec80-186` added by the Edit tool, `tail -1`
    showing it and `ls contracts/windows.version.version/` showing 0.1.0.json alone; restored by `cp`, `cmp` equal.
- **For cards 379-380** (measured here from the stored rows, so they are not looked up again):
  - The laptop runs of the families' binaries, all build 26200.7840 (`profile.txt`). `version`: -186 (runner
    38173733146's binary, 3a49efba...) and -187 (38173735116's, a51dc8c5...), the two bodies of each equal and -186's
    equal to -187's. `system_info`: -156 (3a8a76e2..., the six 2025 runner runs) and -157 (d2530829..., the six 2022 runs).
    `files_more`: -084 (38119625685's) and -085 (38119627458's). Each run is `lap-20261013T231059422Z-136bec80-<n>`.
    No 26200.9974 run of these six binaries is stored. The judge holds 16 runs: 2 version, 12 system_info, 2 files_more.
  - The rows by which the laptop differs from the runner, nothing else differing. `version` -186: lines 88-89
    (`verify.product_server` mismatch, `verify.product_workstation` success: product type 1); 92 (`get_version.ex`:
    `build_at_most_26100` false, `build_is_a_runner_build` false, suite 768, product 1; the build 26200 itself is from
    `profile.txt`); 93 (`get_version.plain`, the same build flags); 96 (`dll_get_version.info2` `version_low` 7840,
    runner 33438 and 5499, which the `version_rows` rule reduces to `version_low_set`, `family_runs.rs` 137-145).
    `system_info` -156/-157: line 4 (`version_ex` `product_type` 1) and line 9 (`system_directory` length 19,
    `is_c_windows_system32` false: the string is held by no stored laptop row, the probe comparing against
    `L"C:\\Windows\\system32"`, `probes/system_info/system_info.c` 124). `files_more` -084/-085: lines 54-55
    (`windows.full`, `windows.exact`: `C:\windows`).
  - Physical memory: GlobalMemoryStatusEx's rows are relations; the laptop's `profile.json` `memory_bytes` is
    33970921472 on both builds (`Win32_ComputerSystem.TotalPhysicalMemory`, `tools/windows_machine/profile.ps1` 36);
    w2m keeps 16 GiB, a machine-decided value not re-taken.
  - G3: in the 14 stored game runs (runner 38216081440, `lap-20261014T032442034Z-c8814547-01` to `-10` at 7840,
    `lap-20261128T014720644Z-33b6423b-01` to `-03` at 9974), every call these tables answer has the same return and
    last error in all 14. The calls: VerSetConditionMask (5, masks 0x8000000000000018 to 0x800000000001b21b), RtlVerifyVersionInfo and
    DllGetVersion through GetProcAddress (0), GetSystemInfo (4, mono, error 203), GetNativeSystemInfo (32),
    GlobalMemoryStatusEx (2, 1) and GetWindowsDirectoryW (1, 10). The game calls no GetVersionExW, VerifyVersionInfoW
    or GetSystemDirectoryW. So no value of these tables differs between builds 7840 and 9974 in G3's stream, and
    runner run 38216081440 holds no machine-decided row for them.
- **Gates:** `exit 0 in 601 s	default	2026-11-28T04:08:58Z	sprint 378	750e5e35e182467fbbbfb10fa39a36fefc4c62c3	c71fb63ea204be727929bda79ec9db53d9bacd026d71d9f3148246b337603993	root=3f98c468263c535f25b05ad2de7ed95994358f81	rust=crates python=0 samples=0	base=1904e61a08e1b4a62dee5864dff3ee2992cd2a1f`
- **G3's two-vCPU batch run in these gates:** `run_seconds` 129.160279408 (variant vm,
  `build/batch_run/record-2-vcpus-1795838451000000000`, verdict conforms); `load_at_start` "load averages 2.21 2.61
  2.56; busy: mds_stores 81%; other w2m: none".
