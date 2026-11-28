# Sprint 381 — The runner's values left in w2m re-taken from stored laptop evidence

```yaml
---
id: 381
status: done
phase: 6
pass_kind: functional
---
```

## scope

Sprints 379 and 380 re-took the version, system-info and message tables. A class search at Sprint 379 found values
still taken from GitHub's Server runner, and the Decision "Which machine-decided values w2m answers as" names the
display. This card re-takes each of them from stored laptop evidence (the user's rule "Match the reference",
BLACKBOARD 160), one concept: no value w2m gives a program is the runner's where a stored laptop row or profile gives
the laptop's.

- **The display.** The game's display calls (GetSystemMetrics 0x4c-0x4f, GetMonitorInfoA/W, GetDpiForMonitor,
  EnumDisplaySettingsW and any other the class search finds) read across the 14 reference game runs in
  `store/game-runs/` (runner 38216081440, the ten `lap-20261014T032442034Z-c8814547-*`, the three
  `lap-20261128T014720644Z-33b6423b-*`): for each, w2m's answer, the laptop's, the runner's, and whether the laptop's
  runs agree among themselves, written as a measurement: the re-take of the display, `windows` and `dxgi` (the
  laptop's screen at 120 DPI) is card 383, by the Decision "The order after Sprint 378" as amended at this card's
  close; the contents of display adapters 3 and 4, which the game asks for on the laptop, are in no stored row
  (card amendment, Sprint 381).
- **The profile folder.** `tools/guest_runtime/known_folders.c` 29, `C:\Users\runneradmin`: the laptop's
  `C:\Users\w2m` (`store/reference_machine/profile-lap-20261128T042841141Z-4c0653e2.json` `user.profile`), and every
  other place the class search finds the runner's user name or profile path.
- **The temp-path fallback.** `tools/guest_runtime/environment.c` 216, `C:\Windows\`: the Windows directory Sprint 379
  measured, `C:\windows`, if Microsoft's GetTempPath2W/GetTempPathW page documents that fallback (cite it; if it does
  not, name the value as not measured).
- **The loader's system path.** `crates/w2m_vm/src/process.rs` 577, `C:\Windows\System32\`: re-taken only if a stored
  laptop row shows how Windows writes a module's path there (a GetModuleFileNameW of a system DLL, a strings row);
  otherwise named as not measured.
- Each re-taken answer that moves the game is measured as Sprint 379 measured, and the frozen stream's cut follows the
  Decision "The system messages re-taken, and where the frozen stream's cut falls".
- **Not here:** `specs/recorded_folders.json` and `specs/windows_exports.json`: their probes (`system_fonts`,
  `exports`) have no stored laptop run, and the laptop refuses new probe runs (blocked on the user). No API id is
  added.

## prerequisites

- 380 (closed)

## context_files

- `tools/guest_runtime/known_folders.c`, `environment.c`, the display answers' files (found by search),
  `crates/w2m_vm/src/process.rs` 560-600.
- `store/game-runs/` (the call traces of the 14 reference runs, read by tools, not whole), `store/reference_machine/`.
- `crates/call_compare/` (how G3 reads the display calls), `crates/w2m_vm/tests/batch_run.rs` (`FIRST_RETAKEN_CALL`).
- `grep -n "Which machine-decided values w2m answers as\|The version and system-info answers re-taken\|where the frozen stream's cut falls" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266 and 301.

## signal contract

### Emits

- None declared: the changes are to guest answers; the gates' existing runs judge them.

### Invariants

- Every run as held; G3's batch runs conform, or a moved call is named and measured; no Windows run on GitHub's
  runner and none on the laptop; `crates/guest_runtime/src/api_ids.txt` unchanged.

## artifact contract

### Files modified

- `tools/guest_runtime/known_folders.c`, `environment.c`, the display answers' files, `crates/w2m_vm/src/process.rs`
  where a row supports it, their tests and the class the search finds; file count recorded.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The display table: call, argument, w2m before, the laptop's (and whether its 13 runs agree), the runner's, w2m after.
  - Each string re-taken, with its source row or profile field, and each named as not measured.
  - 274 family runs; G3 on one and two vCPUs; the frozen stream's cut row and share.
- **Planted faults:**
  - (1) One display answer set back to the runner's: G3 or a family judge fails naming the call.
  - (2) The profile path set back to `runneradmin`: a judge fails naming the row.

## done criteria

w2m gives the laptop's profile folder, temp-path fallback and system path where it gave the runner's, each cited to a
stored laptop row or profile; the display is measured, not re-taken; the other runner-taken values the class search
finds are listed in the result, by group (card amendment, Sprint 381).

## self-review checklist

- [x] Scope names one verifiable property: the runner's values replaced where stored laptop evidence exists.
- [x] context_files cite the files, the game runs, the profile and the Decisions.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command; file count recorded.
- [x] Observation contract: the display table, the strings, the runs; numbered plants.
- [x] One concept.

## result

- **The display table**, the game's display calls in the 14 reference runs' traces (`jit_capture.bin.trace.tsv`,
  the main thread), with the display family's laptop rows for what a trace does not hold (a struct's contents):
  `lap-20261013T231059422Z-136bec80-034` and `-035`, the runners' two binaries, all four bodies byte-equal (metric 93
  1348 in each). The 13 laptop runs (ten `c8814547`, three `33b6423b`) agree among themselves in every return below.

  | call (argument) | w2m before | the laptop's (13 runs; probe rows) | the runner's (38216081440; probe 38273510163) | w2m after |
  |---|---|---|---|---|
  | GetSystemMetrics 0x4c, 0x4d | 0, 0 | 0, 0 | 0, 0 | 0, 0 |
  | GetSystemMetrics 0x4e, 0x4f | 0x400, 0x300 | 0x780, 0x438 (1920 x 1080; metric.78-79) | 0x400, 0x300 | 0x400, 0x300 |
  | EnumDisplayDevicesA (NULL, 0) | TRUE, \\.\DISPLAY1 "Microsoft Hyper-V Video" | TRUE; "__._DISPLAY12" "Intel(R) UHD Graphics" state 5 | TRUE, Hyper-V | as before |
  | EnumDisplayDevicesA (adapter, 0) | TRUE, "Generic PnP Monitor" MSH062E | TRUE; "Generic PnP Monitor" BOE0CFD | TRUE | as before |
  | EnumDisplayDevicesA (NULL, 1) | FALSE | TRUE; DISPLAY13, state 0 | FALSE | as before |
  | EnumDisplayDevicesA (NULL, 2), (NULL, 3), (NULL, 4), (NULL, 5) | not called | TRUE, TRUE, TRUE, FALSE; index 2 DISPLAY14, **indexes 3 and 4 in no row: not measured** | not called | as before |
  | EnumDisplayMonitors (NULL, NULL) | TRUE, one callback 0,0-1024,768 | TRUE; one callback 0,0-1920,1080 | TRUE, 1024 x 768 | as before |
  | GetMonitorInfoA (72), GetMonitorInfoW (104) | TRUE; 0,0-1024,768, work to 720, \\.\DISPLAY1 | TRUE; 0,0-1920,1080, work to 1020, \\.\DISPLAY12 | TRUE; 1024 x 768, work to 720 | as before |
  | MonitorFromWindow, three calls | 0x10001 | 0x10001 | 0x10001 | 0x10001 |
  | GetDpiForMonitor (MDT_EFFECTIVE_DPI) | S_OK, 96 x 96 | S_OK; 120 x 120 (angular 123, raw 141 S_OK) | S_OK, 96 x 96 | as before |
  | EnumDisplaySettingsW (the monitor's device, ENUM_CURRENT_SETTINGS) | TRUE, 1024 x 768 x 32 at 64 Hz | TRUE; 1920 x 1080 x 32 at 144 Hz, 66 modes | TRUE, 1024 x 768 at 64 Hz | as before |
  | CreateDXGIFactory | S_OK | S_OK (dxgi rows -036, -037: 58 lines unlike the runner's) | S_OK | as before |

  The display is not re-taken. On the laptop the game asks EnumDisplayDevicesA for adapters 0 to 5, and the probe
  rows describe adapters 0 to 2 (`probes/display/display.c` 140-147 asks 0, 1, 2 and \\.\DISPLAY9). Answering FALSE
  at index 3 puts w2m's fifth call at FALSE where all 13 laptop runs answer TRUE and the runner makes no fifth call,
  a G3 divergence; answering TRUE would compose the adapters' contents. And the display at 120 DPI reaches the windows
  family: `tools/guest_runtime/windows.c` 317 reads `w2m_display_work_area` and 330 holds 1024 x 768, and laptop run
  -192 differs from runner 38210803867 in 38 lines (the work area 1920 x 1020, WM_GETMINMAXINFO, CW_USEDEFAULT at
  160,160 where 78,78, the client and window rectangles). So the display, windows and dxgi are one class, the
  laptop's screen at 120 DPI, waiting on a row for adapters 3 and 4 that only a new probe run gives, which Smart App
  Control refuses (Sprint 379; blocked on the user).
- **The strings re-taken**, each with its source:
  - the profile, C:\Users\w2m: `store/reference_machine/profile-lap-20261128T042841141Z-4c0653e2.json`
    `user.profile`. `tools/guest_runtime/known_folders.c` 32 (`PROFILE`); `crates/w2m_vm/src/file_bridge.rs` 42-50
    (`PROFILE_FOLDERS`, the three AppData folders under Users/w2m, the runners' three and w2m's Temp before);
    `crates/w2m_vm/src/bin/w2m-vm.rs` 711-718 (`USER_ENVIRONMENT`'s USERPROFILE). The known folder rows show the user
    as `<user>`, and the laptop's -108 and -109 bodies are byte-equal to runners 38098265812's and 38098267402's, so
    `windows.shell.known_folder_path` stays at 0.1.0. The value a user decision named: BLACKBOARD 161 (the user,
    2026-10-11) has w2m report "the runner's user (profile `C:\Users\runneradmin`"; the user's decision of 2026-11-27,
    the laptop the reference ("Which Windows w2m answers as, the reference being the laptop"), moves it. After the
    change no answer names runneradmin; `git grep -i runneradmin` outside the stored runs and the records finds only
    comments naming the old value: `python_image.rs` 104, `w2m-vm.rs` 713, `file_bridge.rs` 44, `batch_run.rs` 50,
    `known_folders.c` 13.
  - GetTempPathW's fallback, C:\windows\: laptop row `temp.none` of -160 and -161, `"path":"C:\\windows\\"` (the
    runners' `C:\\Windows\\`); Microsoft Learn's GetTempPathW page (updated 2025-07-01): after TMP, TEMP and
    USERPROFILE, "The Windows directory.", and "The returned string ends with a backslash" (GetTempPath2W's page,
    updated 2025-11-25, the same order for a process not a system one), cited in `environment.c` 28-29.
    `tools/guest_runtime/environment.c` 219; `contracts/windows.environment.temp_path/0.2.0.json`.
  - the system DLLs' path, C:\windows\System32\: laptop row `file_name` of -158 and -159, GetModuleFileNameW of
    SHELL32, `"C:\\windows\\System32\\SHELL32.dll"` (the runners' `C:\\Windows\\System32\\`).
    `crates/w2m_vm/src/process.rs` 45-50 (`SYSTEM32_PATH`) and 583 (PEB.Ldr's FullDllName); `crates/w2m_vm/src/
    modules.rs` 441-448 and 463 (GetModuleFileNameW of KERNEL32, KERNELBASE and a system DLL);
    `contracts/windows.module.system/0.2.0.json`.
  - Each 0.2.0 is 0.1.0 copied by `cp` and edited: version, summary, applicability, scenario, the one row, the
    evidence the laptop runs, the runners in `history`. `store/family_runs/retaken-2026-11-27.tsv` 6-7 route the two
    families to -160 and -158; `target/release/family_known .` moves `known-2026-10-29.tsv` 235-238 column 4 alone, and
    the stored lookup equals its output. `RETAKEN_DIFFERENCES` (`runner_families_on_the_laptop.rs`) adds
    `temp.none` and `file_name`. The family outputs move in those rows alone against the last gates' outputs.
- **Named as not measured:** the laptop's EnumDisplayDevicesA adapters 3 and 4 (above); the rest of the laptop's
  environment block (w2m gives three variables; no row records the laptop's).
- **Found by the class search, not re-taken here:** every family's laptop run against the runner run of the same
  binary (body 1, by line). Beyond the four Sprints 379-380 re-took and this card's, display 77 lines, dxgi 58 and
  windows 38 and 32 (above), in three groups:
  - settled by a Decision: processor_topology (`machine` active 12, cores 8, l1d 32768, l3 12582912, against 4, 2,
    49152, 272629760; the Decision "Which machine-decided values w2m answers as": topology as the runners' rows, the
    user's of 2026-10-11, BLACKBOARD 161, four processors); process_queries (`ipfp.21` 0 against 1, `ipfp.61` 1
    against 0; the user's CPUID decision of 2026-10-11, `specs/guest_cpuid.json` 0.2.0); com (audio endpoints 1, 1
    and 6 and a default device, against none; the Decision "What audio the game sees").
  - machine values with a laptop row, not yet re-taken: time_and_error_mode (`time_zone` bias 480 "Pacific Standard
    Time" against 0 "Coordinated Universal Time"; `local_time`) and locale (`local.*`, `local.first_day` ret 0 err 87);
    loader (`ldr.*_order`: the runners list apphelp.dll, the laptop does not); paths (`full.device_name` "x\\CON":
    the laptop "<cwd>\\x\\CON", the runners "\\\\.\\CON"); file_attributes (`ex.dot` size_low 0 against 4096;
    `ex.file.trailing_backslash` err 267 against 123); process_parameters (`parent.standard_handles` input null,
    `parent.environment` version 3 against 4, `parent.block` region 0x6000).
  - values that vary by run or layout: heap_in_place (`limit.fresh.*` steps), heap_query
    (`allocation_base_below_block`), interlocked_lists (`push.*`), file_mapping and virtual_memory (the free regions'
    sizes), thread_stack (`limit+2p` against `limit+3p`), processes (a child's pid).
- **The game's runs, measured as Sprint 379 measured** (each change reverted in place, one batch run at a time,
  `scratchpad/381/run-batch.sh`, against the Sprint 380 gates' stream, which equals the frozen stream to row 6364):
  run A, 06:48:49-06:49:17Z, the whole profile: first difference row 29, EnterCriticalSection's a3 (a register past
  its one parameter), 0x200009d20 where 0x200009d40, 10,515 rows to the stop where 10,505; run B,
  06:49:57-06:50:23Z, USERPROFILE reverted: row 967, CoTaskMemFree's a2 after SHGetKnownFolderPath (row 966); run C,
  06:51:55-06:52:23Z, the profile reverted, the temp fallback and the system path re-taken: equal over 10,505 rows.
  The game's first GetTempPathW, row 469, answers the same before (the Sprint 380 stream) and after (run C and the
  gates' run): `469	4	UnityPlayer.dll	KERNEL32.dll	GetTempPathW	104	fffef840	fffefc40	10000005c6	a	2000014da	10b0009a20	18143092b	20	131`,
  0x20 units, TMP being set, so the fallback is not reached in the game's run.
  So the start environment's USERPROFILE moves the game first, read by ucrtbase's untraced copy after row 28.
- **The cut**, by the Decision "The system messages re-taken, and where the frozen stream's cut falls": row 28 of
  10,505, 0.3% held (`crates/w2m_vm/tests/batch_run.rs` `LAST_HELD_ROW` and `AT_THE_CUT`, the cut a row since the
  first answer moved is no traced call): "the Rust stream equals the Python's rows 0-28 of 10505, to the start
  environment re-taken from the laptop (0.3% held); its 10486 rows past it are judged against Windows alone". The
  cut below half is the Decision's revisit: a stream made by the Rust host joins the reference. The held stop counts
  (`store/batch_run/first-stop-2026-10-28.tsv`): hvc 8 38939 to 38938, compiled 274778 to 274788, blocks the host
  compiled 275556 to 275566, calls traced 10505 to 10515, four runs alike.
- **The exit ledger** (`crates/w2m_vm/tests/exit_ledger.rs`, the frozen host's ledger to the game's first MAP_VIEW
  29): the second gates run failed it, 3,281 cells against the 29 held extras. Measured in place, one batch run each:
  USERPROFILE reverted, 1,241 cells, the process area's hash from row 0; USERPROFILE and the loader's FullDllName
  (`process.rs` 583) both reverted, the 29 extras alone, exit 0. So the profile's environment value and the loader
  path change the process area from the start (main: the hash at 1,022 rows from row 0, executed from 96, warp from
  260, site from 318; child 4100: hash and warp from row 0, executed from 46), and the folder path, the temp fallback
  and GetModuleFileNameW's path leave the ledger. `store/exit_ledger/differences-2026-10-29.tsv` holds the test's
  3,281 lines for that run, two runs alike, with a Sprint 381 comment; its header's rule, "Written by
  crates/w2m_vm/tests/exit_ledger.rs and held by it". The frozen ledger now holds the rows' kinds and order, its
  values the Rust host's own run past the start, which card 382's stream will also be. No test changed for it.
- **The class: every test under `crates/w2m_vm/tests` naming the frozen host** (`grep -l -i frozen`, 12 files), each
  run in the gates: moved by this card, `batch_run.rs` (the call stream from row 29 by USERPROFILE, from 967 by the
  folder path; the held stop counts), `batch_record.rs` (the record held to the stop file's counts, which moved with
  them; plant 3) and `exit_ledger.rs` (USERPROFILE and the loader's FullDllName, above); not moved, passing unchanged
  with the change in place: `case_runner.rs`, `class_judge.rs`, `host_requests.rs`, `run_loop.rs`, `litmus.rs`,
  `guest_stubs.rs`, `private_chain.rs`, `skeleton_gate.rs`, and `family_runs.rs` but for the two families routed to
  their laptop runs.
- **G3 and the families** before the gates: "the Rust host's 10515 calls: conforms, 0 defects, 0 past the cut (0 for
  the frozen host's same rows), 0 scheduling, 25 values Windows varies"; the two-vCPU record's COMPARISON_COMPLETED
  verdict conforms, `first_divergence` null; "274 of 274 family runs conform, 69 of 69 families in every run"; "on
  two vCPUs: 274 of 274 held as on one vCPU, 274 of them conforming".
- **File count** (`WORKING_AGREEMENT.md` 301): 2 created (the two 0.2.0 contracts) and 13 modified
  (`known_folders.c`, `environment.c`, `process.rs`, `modules.rs`, `file_bridge.rs`, `w2m-vm.rs`, `batch_run.rs`,
  `python_image.rs` DEPARTURES 19, `runner_families_on_the_laptop.rs`, `retaken-2026-11-27.tsv`,
  `known-2026-10-29.tsv`, `first-stop-2026-10-28.tsv`, `differences-2026-10-29.tsv`), with this card and the gates'
  rows; one concept, the runner's strings w2m gives a program, their readers and their judges.
- **A third gates run passed** before the late additions (`exit 0 in 598 s`, 2026-11-28T08:10:58Z, root=84890a1dc,
  kept in `store/gates/passes.tsv`); the GetTempPathW citation in `environment.c` changed the gated tree after it, so
  the gates ran once more, below.
- **Two gates runs failed before the pass**, their steps' rows kept in `store/gates/steps.tsv`: the first (started 07:20:36Z)
  2 of 502 tests, the held stop file's line 2 written "stopReferenceEnd", its tab dropped by an Edit whose new text
  ended in a tab; the second (started 07:38:02Z) the exit ledger above, and line 5 of the regenerated ledger lost its tab the same way. Each fixed
  by `sed`, and every edited TSV scanned for a row with too few tab fields.
- **Plant 1** "temp_path	38098847298	ended; row \"temp.none\": path reference \"C:\\\\windows\\\\\", target
  \"C:\\\\Windows\\\\\" (windows.environment.temp_path@0.2.0)	(which store/family_runs/sweep-held-2026-10-29.tsv does
  not hold)", and the same for 38098848806, `family_runs::every_family_run_conforms_or_is_held_by_what_it_waits_on`
  exit 100; live 07:13:32-07:14:55 UTC. The card's plant (1) sets a display answer back, and the display is not
  re-taken; the plant is a re-taken answer set back to the runner's in its place:
  - backup `scratchpad/plants/381/environment.c.backup`; `environment.c` 218's fallback `'w'` set to `'W'` with the
    Edit tool (grep, line 218); every family run on one and two vCPUs (`scratchpad/381/family-runs.sh`, gates.sh
    242-263), then the judge under `W2M_REQUIRE_CASE_RUNS=1`; log `plant1.log`; restored by `cp`, `cmp` equal.
  - run again after `environment.c`'s comment cited the GetTempPathW page, live 08:13:32-08:14:51 UTC: backup
    `environment.c.backup2`, the same fault at line 219 (grep), the same two lines quoted, exit 100; log `plant1b.log`;
    restored by `cp`, `cmp` equal.
- **Plant 2** "known_folders	38098265812	stopped as expected; row \"profile\": missing (0 rows printed, 8 expected)
  (windows.shell.known_folder_path@0.1.0)	(which store/family_runs/sweep-held-2026-10-29.tsv does not hold)", and the
  same for 38098267402, the same judge, exit 100; live 07:15:09-07:16:24 UTC:
  - backup `known_folders.c.backup`; `PROFILE` set to C:\Users\runneradmin with the Edit tool (grep, line 32); the
    family runs and the judge as for plant 1; log `plant2.log`; restored by `cp`, `cmp` equal.
- **Plant 3** "vm_exits {\"hvc 1\": 1146.0, ... \"hvc 13\": 13638.0, ... \"hvc 8\": 38937.0} where the held stop file
  holds {\"hvc 1\": 1147.0, ... \"hvc 13\": 13639.0, ... \"hvc 8\": 38938.0}",
  `batch_record::the_batch_runs_record_is_the_frozen_hosts_in_form_and_its_own_stop_in_count` exit 100; live
  07:16:37-07:18:26 UTC; the stream test passed beside it, rows 0-28 being alike either way, so USERPROFILE's judge is
  the held counts:
  - backup `w2m-vm.rs.backup`; USERPROFILE set to C:\Users\runneradmin with the Edit tool (grep, line 717); w2m-vm
    rebuilt, the batch record run, the two tests; log `plant3.log`; restored by `cp`, `cmp` equal.

- **The judges in the last gates run:** G3, "the Rust host's 10515 calls: conforms, 0 defects, 0 past the cut (0 for
  the frozen host's same rows), 0 scheduling, 25 values Windows varies" and "the Rust host's 10569 calls on two vCPUs:
  conforms, 0 defects, 0 past the cut, 0 scheduling, 24 values Windows varies"; "274 of 274 family runs conform, 69
  of 69 families in every run"; "on two vCPUs: 274 of 274 held as on one vCPU, 274 of them conforming"; "the Rust
  stream equals the Python's rows 0-28 of 10505, to the start environment re-taken from the laptop (0.3% held); its
  10486 rows past it are judged against Windows alone".
- **Gates:** `exit 0 in 583 s	default	2026-11-28T08:25:13Z	sprint 381	3a776fe58bd93145c0b94bf0f5805da09ca6c790	32616916e8d6295f627e2f89cb83ab421c88c3dc9a20e8d2872c2c236ee0b95a	root=556e38d69570a7f7b44f5bb5ba575ffb90d8b888	rust=all python=0 samples=0	base=5fe915265660f096b73d1d6a6608a5a2a7126963`
  (502 tests run, 502 passed, 1 skipped).
- **G3's two-vCPU batch run in these gates:** `run_seconds` 132.734355062 (variant vm,
  `build/batch_run/record-2-vcpus-1795853831000000000`, verdict conforms); `load_at_start` "load averages 1.38 2.14
  2.75; busy: mds_stores 96%; other w2m: none".
