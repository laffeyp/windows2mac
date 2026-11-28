# Sprint 379 — The version and system-info answers as the laptop gives them

```yaml
---
id: 379
status: closed-partial
phase: 6
pass_kind: functional
---
```

## scope

One path from input to verdict (`WORKING_AGREEMENT.md` 301): w2m's version and system-info answers re-taken from the
laptop, and the three families that read them held to the laptop's rows. Sprint 378 built the route
(`store/family_runs/retaken-2026-11-27.tsv`) and wrote, in its card's result, the laptop runs and rows this card needs:
`version` (`lap-20261013T231059422Z-136bec80-186`, `-187`), `system_info` (`-156`, `-157`), `files_more` (`-084`, `-085`),
16 runs, all build 26200.7840; the rows by which the laptop differs from the runner (product type 1, suite 768, a build
above 26100, shell32's revision 7840 reduced to `version_low_set`, the Windows directory `C:\windows`); and the one
value no stored row holds, GetSystemDirectoryW's string (length 19, not `C:\Windows\system32`).

- **The measurement.** `tools/windows_machine/profile.ps1` records `system_directory` and `windows_directory` in
  `profile.json`, by a call whose path to GetSystemDirectoryW and GetWindowsDirectoryW is cited from Microsoft's
  documentation or .NET's source; one laptop job of the `system_info` probe records them and stores a 26200.9974 run
  of the family. This is a laptop path, committed by card amendment (Sprint 377's rule).
- **The tables.** `tools/guest_runtime/version.c` and `system_info.c` give the laptop's values, each cited to its row;
  physical memory stays 16 GiB (the Decision "The order after Sprint 378").
- **The families.** `contracts/windows.version.version/0.2.0.json`, `windows.system.information/0.2.0.json` and
  `windows.file.files_more/0.2.0.json`, each 0.1.0's with the laptop run's rows and evidence (0.1.0 kept unchanged),
  named in `retaken-2026-11-27.tsv`; `store/family_runs/known-2026-10-29.tsv` regenerated; the two files' entries in
  `crates/guest_runtime/tests/python_image.rs` DEPARTURES, each with its reason.
- **The readers.** Every reader of the family tables reads the new table: `reference_machine.rs` `FAMILY_TABLES`
  first, and the others a search for `joined-2026-11-27.tsv` finds (`family_runs.rs`, `family_known.rs`,
  `runner_families_on_the_laptop.rs`, `windows_machine`'s `lib.rs` and `windows_machine.rs`).
- **Not here:** the spec tables (cards 380, 381). No API id is added.

## prerequisites

- 378 (closed)

## context_files

- `sprints/sprint-378-version-and-system-info-from-the-laptop.md` (its result, "For cards 379-380").
- `tools/guest_runtime/version.c`, `system_info.c`; `probes/version/`, `probes/system_info/`, `probes/files_more/`.
- `contracts/windows.version.version/0.1.0.json`, `windows.system.information/0.1.0.json`, `windows.file.files_more/0.1.0.json`.
- `crates/w2m_vm/src/bin/family_known.rs`, `crates/w2m_vm/tests/family_runs.rs`, `crates/guest_runtime/tests/python_image.rs`.
- `tools/windows_machine/profile.ps1`; `crates/windows_machine/tests/reference_machine.rs`.
- `grep -n "The order after Sprint 378\|Which Windows w2m answers as" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266 and 301.

## signal contract

### Emits

- None declared: the family runs the judge holds are the gates' existing runs.

### Invariants

- Every run as held; G3's batch runs conform; no Windows run on GitHub's runner; `crates/guest_runtime/src/api_ids.txt`
  unchanged; every 0.1.0 contract byte-equal to HEAD's.

## artifact contract

### Files created

- `contracts/windows.version.version/0.2.0.json`, `contracts/windows.system.information/0.2.0.json`,
  `contracts/windows.file.files_more/0.2.0.json`; the laptop job's `store/probe-runs/<run>/`.

### Files modified

- `tools/guest_runtime/version.c`, `system_info.c`, `tools/windows_machine/profile.ps1` (by amendment),
  `store/family_runs/retaken-2026-11-27.tsv`, `known-2026-10-29.tsv`, `crates/guest_runtime/tests/python_image.rs`,
  the family-table readers; file count recorded (`WORKING_AGREEMENT.md` 301).

### Command exit codes

- `target/release/windows_machine probe system_info` returns 0, its `profile.json` carrying `system_directory`
- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The value table: call, field, w2m before, laptop 7840, laptop 9974 (the new run), w2m after, the row cited.
  - The 16 runs of `version`, `system_info` and `files_more`, each conforming to its 0.2.0 contract.
  - The new `system_info` run's rows against `-156`'s, by Sprint 374's four outcomes.
- **Planted faults:**
  - (1) The product type set back to 3: the `version` and `system_info` judges fail naming the row.
  - (2) A route row removed: the family is held to 0.1.0 and fails naming the row.
  - (3) A laptop run cited only in `retaken-2026-11-27.tsv` changed to a runner's: the cited-runs walk fails naming it.
  - (4) One cell changed in a row of the batch run's call stream before the first re-taken call: the frozen-host test
    fails naming the row.

## done criteria

w2m answers the version and system-info calls as the laptop does, each value cited to a laptop row, and the three
families that read them conform to contracts holding the laptop's rows.

## self-review checklist

- [x] Scope names one verifiable property: the three families conform to the laptop's rows.
- [x] context_files cite Sprint 378's facts, the tables, the contracts, the route and the readers.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command; file count recorded.
- [x] Observation contract: the value table, the runs, the new laptop run; numbered plants.
- [x] One concept: one path from the laptop's rows to the families' verdicts.

## result

- **The laptop job failed; nothing of it is stored.** `tools/windows_machine/profile.ps1` 24-45 and 54-55 (committed by
  the card amendment 4c0653e26) call kernel32's GetSystemDirectoryW and GetWindowsDirectoryW themselves (DllImport by
  Add-Type, as `cpusets.ps1` calls GetSystemCpuSetInformation; the pages
  learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getsystemdirectoryw and
  .../nf-sysinfoapi-getwindowsdirectoryw, updated 2025-07-01) and write `system_directory` and `windows_directory` to
  `profile.json`. `target/release/windows_machine probe system_info` exit 1: job `lap-20261128T042841141Z-4c0653e2`
  (04:28:41-04:29:32 UTC) built probe.exe on the laptop, byte for byte binary 3a8a76e2... (-156's), and execution 1
  exited 4551, "'D:\a\windows2mac\windows2mac\probe.exe' was blocked by your organization's Device Guard policy."
  `windows_machine binary system_info`, job `lap-20261128T042957607Z-4c0653e2` (04:29:57-04:30:36 UTC), sent the stored
  copy: the same refusal, exit 1. Nothing was stored as a run. The profile ran in both before the probe:
  `profile.json` holds `"system_directory": "C:\\windows\\system32"`, `"windows_directory": "C:\\windows"`, build 26200,
  UBR 9974, `memory_bytes` 33970921472, taken 04:29:20.69Z and 04:30:29.96Z. The first is kept, a measurement of the
  reference machine and not of a probe, as `store/reference_machine/profile-lap-20261128T042841141Z-4c0653e2.json`,
  copied byte for byte (`cmp` equal; sha256 a4bdf40424dc75fbee367edb9ed1a53708930a41dacd2ed4e724e0952aedb476; user
  w2m, no password or Steam value). Each job's CodeIntegrity window read `{"blocks": []}` (04:29:11.077Z-04:29:27.744Z,
  04:30:22.518Z-04:30:31.896Z), and a later query of the laptop's log shows event 3077 at 2026-11-28T04:29:27.5742264Z,
  cmd.exe loading probe.exe, with 3033, 3089 and 3118 at the same instant: inside the first window, 170 ms before its
  end, and not returned to `run-job.ps1`'s reader when it ran (Sprint 377's window; not changed here). Smart App
  Control is the user's decision and stays as it was; no further laptop job was sent. So no 26200.9974 run of the
  family is stored, and the artifact contract's run and the command's exit 0 are not met: the close is partial,
  blocked on the user. Deferred: a 26200.9974 `system_info` run, once Windows admits the probe binary.
- **The values**, each from a stored laptop row of 26200.7840 (`lap-20261013T231059422Z-136bec80-<n>`, Sprint 378's
  result) but the system directory's string:

  | call | field | w2m before | laptop 7840 | laptop 9974 | w2m after | row |
  |---|---|---|---|---|---|---|
  | GetVersionExW, manifest | build | 26100 | 26200 | 26200 (profile) | 26200 | -186 92, `build_at_most_26100` false; profile.txt |
  | GetVersionExW, manifest | suite | 400 | 768 | not run | 768 | -186 92 |
  | GetVersionExW, manifest | product | 3 | 1 | not run | 1 | -186 92 |
  | GetVersionExW, none | product_type | 3 | 1 | not run | 1 | -156 4 |
  | RtlVerifyVersionInfo, VerifyVersionInfoW | product type | 3 | 1 | not run | 1 | -186 88-89 |
  | RtlVerifyVersionInfo, VerifyVersionInfoW | build | 26100 | 26200 | 26200 (profile) | 26200 | profile.txt |
  | DllGetVersion, DLLVERSIONINFO2 | version_low | 33438 | 7840 | not run | 7840 | -186 96 (`version_low_set`) |
  | GetWindowsDirectoryW | path | C:\Windows | C:\windows | C:\windows (profile) | C:\windows | -084 54-55 |
  | GetSystemDirectoryW | path | C:\Windows\system32 | 19 units, not C:\Windows\system32 | C:\windows\system32 (profile) | C:\windows\system32 | -156 9; the stored profile |
  | GlobalMemoryStatusEx | ullTotalPhys | 16 GiB | 33970921472 (profile.json) | 33970921472 (profile) | 33970921472 | the stored profile's `memory_bytes`; rows relations |

  `tools/guest_runtime/version.c` 15-24 and 49-54 (BUILD 26200, PRODUCT_TYPE 1) and 140-146 (SHELL32_REVISION 7840);
  `tools/guest_runtime/system_info.c` 6-24, 59-80 (GetVersionExW: 26200, suite 768, product 1), 89-106 (ullTotalPhys
  33970921472, the one amount measured, cited to the stored profile's `memory_bytes`, Win32_ComputerSystem.
  TotalPhysicalMemory), 113-123 (C:\windows), 125-138 (C:\windows\system32, the comment citing the stored profile and
  its sha256). The card's scope kept 16 GiB (the Decision "The order after Sprint 378"); after reviewer-4's
  fifty-third review the total is the laptop's by the user's rule "Match the reference" (BLACKBOARD 160), UnityPlayer.dll
  calling GlobalMemoryStatusEx twice in each game run (c8814547-01, 38216081440, 33b6423b-01) and 16 GiB being no
  figure of the VM either (its heap is 256 MiB, guest_layout HEAP_SIZE). The relations hold: available 12 GiB at most
  the total, the total at least 1 GiB, load 25 at most 100. Not measured, kept as w2m's figures: ullAvailPhys (12 GiB;
  -156 line 6 holds only `avail_phys_le_total`), ullTotalPageFile (32 GiB) and ullAvailPageFile (28 GiB), which no row
  records; with the laptop's total the 32 GiB commit limit implies a paging file of about 371 MiB that no reference
  shows (Microsoft's MEMORYSTATUSEX page, learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/
  ns-sysinfoapi-memorystatusex, updated 2024-02-22: ullTotalPageFile "The current committed memory limit for the system
  or the current process, whichever is smaller, in bytes"; cited in the comment too, written before this gates run). ullTotalVirtual 140737488224256
  and ullAvailExtendedVirtual 0 are -156 line 6's values, unchanged.
- **The families.** `contracts/windows.version.version/0.2.0.json`, `windows.system.information/0.2.0.json`,
  `windows.file.files_more/0.2.0.json`: 0.1.0's with the laptop runs' rows (version_rows and files_more_rows applied,
  system_info raw), the laptop runs as evidence (-186 and -187; -156 and -157; -084 and -085, each four bodies alike),
  the runner runs in a `history` field, as `windows.sync.named_objects` 0.2.0 has it; system_info's evidence also
  `laptop-profile`, the stored profile and its sha256. Rows differing from 0.1.0:
  `verify.product_server`, `verify.product_workstation`, `get_version.ex`, `get_version.plain`; `version_ex`,
  `system_directory`; `windows.full`, `windows.exact`. Each 0.1.0 `git diff --quiet HEAD`, exit 0.
  `store/family_runs/retaken-2026-11-27.tsv` names version (-186), files_more (-084) and system_info (-156);
  `store/family_runs/known-2026-10-29.tsv` regenerated by `target/release/family_known .`, 16 lines changed, column 4
  alone. `crates/guest_runtime/tests/python_image.rs` DEPARTURES 16, `system_info.c` and `version.c` with their reasons.
- **The 16 runs under 0.2.0.** `scripts/gates.sh`' family-runs step run alone (its lines 242-263 in a scratchpad
  script, after `cargo build --release -p w2m_vm`), then `W2M_REQUIRE_CASE_RUNS=1 cargo nextest run -p w2m_vm --test all -E
  'test(/family_runs::every_family_run/)'`: "274 of 274 family runs conform, 69 of 69 families in every run", "on two
  vCPUs: 274 of 274 held as on one vCPU, 274 of them conforming"; the 2 version, 12 system_info and 2 files_more runs
  among them.
- **The new system_info run against -156:** none stored. Of Sprint 374's four outcomes it is the fourth, no laptop rows,
  Windows refusing the binary; -156 and -157 stay the family's laptop rows.
- **The readers.** `crates/windows_machine/tests/reference_machine.rs` `FAMILY_TABLES` (table, run column, laptop runs
  alone): the re-taken table's third column, a run there held as the laptop's; and a cited laptop job with no stored
  run passes only by its profile under `store/reference_machine/` naming the laptop and build 26200
  (`every_cited_run_is_held_to_the_reference_rule`, "732 citations, 278 by the family tables and 454 by 108 contract
  files: {"Laptop": 60, "LaptopProfileNoRun": 1, ...}; failing 0"). No other reader walks `store/reference_machine/`:
  `reference_machine.rs`, `runner_families_on_the_laptop.rs` and `run_loop.rs` read its one list by name.
  `crates/windows_machine/src/lib.rs` `RETAKEN` and
  `batch_items`: a re-taken family's laptop run joins the item of its binary, or is an item of its own, refused for a
  DLL probe; `tests/windows_machine.rs` `the_batch_reads_the_retaken_table`, and the joined table's test writes the
  table's header. `crates/windows_machine/tests/runner_families_on_the_laptop.rs`
  `each_retaken_familys_laptop_rows_differ_from_the_runners_as_named`: version 4 pairs, files_more 4, system_info 24,
  each differing in the rows Sprint 378 named and no other. `crates/w2m_vm/tests/family_runs.rs`:
  `a_retaken_row_moves_its_familys_contract_and_nothing_else` takes the stored rows and the fixture's after them;
  `each_retaken_contract_holds_its_laptop_runs_rows`: version and files_more 4 bodies each, system_info 4, conforming.
  `family_known.rs` reads the table already (Sprint 378). `tools/windows_machine/profile.ps1`'s text test,
  `the_profile_records_the_system_and_windows_directories`. `cargo nextest run -p windows_machine -p guest_runtime`:
  62 passed.
- **File count** (`WORKING_AGREEMENT.md` 301): 4 created (the three contracts and the stored profile), 13 modified
  (profile.ps1 by the amendment, and from the batch run's measurement `batch_run.rs` and the held stop file) and the
  card; over 10 because the Decision "The
  order after Sprint 378" makes this one path from the laptop's rows to the families' verdicts, every reader of the
  table in it.
- **Plant 1** "system_info	38049865817	ended; row \"version_ex\": product_type reference 1, target 3
  (windows.system.information@0.2.0)" and "version	38173733146	ended; row \"get_version.ex\": product reference 1,
  target 3 (windows.version.version@0.2.0)", `W2M_REQUIRE_CASE_RUNS=1 cargo nextest run -p w2m_vm --test all -E
  'test(/family_runs::every_family_run/)'` exit 100; live 04:35:01-04:36:27 UTC:
  - backup `scratchpad/plants/379/system_info.c.backup`; `p[282] = 1` set to 3 by the Edit tool (GetVersionExW's
    product type, which both families print); landed (grep, line 76); the family-runs step run; 14 runs named, 12
    system_info and 2 version; restored by `cp`, `cmp` equal; the step run again, 274 of 274 conforming.
- **Plant 2** "version	38173733146	ended; row \"verify.product_server\": err reference 57005, target 1150
  (windows.version.version@0.1.0)", the same judge, exit 100; live 04:37:43-04:37:58 UTC:
  - backups `scratchpad/plants/379/retaken-2026-11-27.tsv.backup` and `known-2026-10-29.tsv.backup`; version's row
    removed by the Edit tool (`grep -c ^version`: 0); the lookup regenerated by `family_known`, version's two runs
    naming `windows.version.version@0.1.0`; restored by `cp`, both `cmp` equal.
- **Plant 3** "store/family_runs/retaken-2026-11-27.tsv cites run 38119625685: Some(Ok(RunnerBeforeTheLaptop)), where
  the table holds stored laptop runs (laptop-x64-10.0.26200.<revision>-Client) alone", `cargo nextest run -p
  windows_machine -E 'test(every_cited_run_is_held_to_the_reference_rule)'` exit 100; `family_known .` exit 1,
  "files_more's run 38119625685 is no stored laptop run"; live 04:38:11-04:38:11 UTC:
  - backup `scratchpad/plants/379/retaken-plant3.tsv.backup`; files_more's run -084 set to its runner run 38119625685
    by the Edit tool (grep shows it); restored by `cp`, `cmp` equal.
- **The batch run against the frozen host.** The gates run of 04:51:40Z failed three frozen-host tests:
  `batch_run::the_batch_runs_calls_are_the_python_runs_up_to_the_stop` ("row 6457 of the call stream differs":
  mono-2.0-bdwgc.dll ResumeThread 20b8, 0-based column 12, `15381455` where the frozen host has `1538143f`),
  `batch_run::the_batch_run_and_the_guests_stop_where_held` and
  `batch_record::the_batch_runs_record_is_the_frozen_hosts_in_form_and_its_own_stop_in_count` (hvc 8 38939 where the
  held file has 38881). Measured, one batch run at a time (the gates' step, gates.sh 168-180, then the call-stream
  test), each change reverted from a backup and restored by `cp`: the directories back to `C:\Windows`
  (05:03:57-05:05:35), PASS; ullTotalPhys back to 16 GiB (05:05:47-05:07:25) and the version answers back to Server
  2025's (05:07:40-05:09:18), each FAIL at the same row. So the lower-case directories move the game. The stream's
  columns are seq, tid, label, DLL, function, a0-a7, return, last error (`crates/w2m_vm/src/guest.rs` 1588-1590):
  the first cells past GetWindowsDirectoryW that differ lie in argument slots past the function's parameters in
  `specs/signatures.json` (ResumeThread's a7; WaitForSingleObjectEx's a6 at 6719 and 6722; ReleaseSemaphore's a7 at
  6717 and a4 at 6768), and from row 6881 the calls' order changes (an extra SwitchToThread, SleepEx moved; Sleep 97
  calls to 100), 10505 rows on both sides. G3 in that run: `cargo nextest run -p call_compare`, 23 passed, "the Rust
  host's 10505 calls: conforms, 0 defects, 0 past the cut ..., 25 values Windows varies" and on two vCPUs "10468
  calls ...: conforms, 0 defects". The rule now (`crates/w2m_vm/tests/batch_run.rs` `FIRST_RETAKEN_CALL`): the frozen
  stream is the reference row for row up to and including the first call whose answer a card re-took from the
  laptop and which moved the game, GetWindowsDirectoryW, found in the stream by name (row 6364, seq 6597); past it
  the rows are judged against Windows alone, as past the stream's end. The answers re-taken earlier in the stream
  (GlobalMemoryStatusEx at rows 1436 and 2572, RtlVerifyVersionInfo at 2380, DllGetVersion at 2531) leave every row to the cut
  equal, and the test holds them so. `store/batch_run/first-stop-2026-10-28.tsv` rewritten from this run's stop, as
  its header says: hvc 8 38881 to 38939, compiled 274830 to 274778, blocks the host compiled 275608 to 275556 (three
  lower-case runs alike); no test changed for the stop. On a fresh batch run of the tree the call-stream test and
  `batch_record`'s pass.
- **Plant 4** "row 6000 of the call stream differs from the Python batch run's", left `[..., "ReleaseSRWLockExclusive",
  "20247c321", "40", ...]`, right `[..., "20247c320", "40", ...]`, `W2M_REQUIRE_CASE_RUNS=1 cargo nextest run -p w2m_vm
  --test all -E 'test(/batch_run::the_batch_runs_calls/)'` exit 100; live 05:17:15-05:17:16 UTC:
  - backup `scratchpad/plants/379/first-stop.calls.tsv.backup`; in `build/batch_run/first-stop.calls.tsv` row 6000's
    a0 (seq 6219, ReleaseSRWLockExclusive) `20247c320` set to `20247c321` by the Edit tool, `diff` showing that one
    cell; restored by `cp`, `cmp` equal. A first attempt, 05:16:56-05:17:05, also lost the tab after the cell and
    failed at the same row; it was restored and made again.
- **For later cards:** `tools/guest_runtime/environment.c` 216 writes `C:\Windows\` and `crates/w2m_vm/src/process.rs`
  577 `C:\Windows\System32\`, against the laptop's `C:\windows`; `known_folders.c` 29's profile is the runner's
  `C:\Users\runneradmin`. The CodeIntegrity window returned no block for a refusal its log holds inside the window.
- **Gates:** `exit 0 in 603 s	default	2026-11-28T05:29:25Z	sprint 379	4c0653e266c64ec58fbd11aee59d66e197419071	890d7c4605b012861eb7792947a369b6bb7b97381edfb6223e579f95a80fbcab	root=2f58239c2e25d9413c613549fd1f2a759e4ad0bf	rust=all python=0 samples=0	base=72edfff0c8d9d328efc897697883087a452826c2`
- **G3's two-vCPU batch run in these gates:** `run_seconds` 129.261997748 (variant vm,
  `build/batch_run/record-2-vcpus-1795843270000000000`, verdict conforms); `load_at_start` "load averages 2.49 2.31
  2.30; busy: mds_stores 91%; other w2m: none".
