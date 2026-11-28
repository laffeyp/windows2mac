# Sprint 380 — The system messages re-taken from the laptop's stored rows

```yaml
---
id: 380
status: done
phase: 6
pass_kind: functional
---
```

## scope

`specs/system_messages.json` (version 0.1.0, `windows_build` 10.0.26100) holds FormatMessage's text for each system
error code, taken from GitHub's Server runner: `store/probe-runs/38120951889/body-1.jsonl`, probe `format_message`,
rows "dump". The laptop ran the same binary on 2026-10-13: `store/probe-runs/lap-20261013T231059422Z-136bec80-086`
(`probe.sha256` 4224a823... in both), build 26200.7840. This card re-takes the table from the laptop's rows (the
Decision "Which Windows w2m answers as, the reference being the laptop"; the user's rule "Match the reference",
BLACKBOARD 160), one path from input to verdict (`WORKING_AGREEMENT.md` 301).

- **The table.** `specs/system_messages.json` at 0.2.0 from `-086`'s "dump" rows, 0.1.0 kept; each code whose text
  differs between the runner and the laptop listed in the result with both texts, and each code present on one machine
  only.
- **Its readers.** Every program and test that reads the table follows the new version (a search for
  `system_messages` finds them); the `format_message` family, if it is frozen, held to the laptop's rows through
  `store/family_runs/retaken-2026-11-27.tsv` as Sprint 379 held three families.
- **Committed with this card:** `crates/signal_report/src/main.rs` names Sprint 379's close 3b17c1b78 in
  `BETWEEN_SPRINTS` (it carried the work and the close under "Sprint 379 closed-partial:", where "Sprint 379: " was due
  from Sprint 334), and a named close passes the close-apart, bookkeeping-path and work-commit checks; 51 of 51
  `signal_report` tests pass. This card's gates run holds it.
- **Not here:** `specs/windows_exports.json`: its probe, `exports`, has no stored laptop run, and Smart App Control
  refuses new laptop probe runs (Sprint 379; blocked on the user). `recorded_folders.json` and the display calls, card
  381. No API id is added.

## prerequisites

- 379 (closed-partial; its open question does not touch this table)

## context_files

- `specs/system_messages.json` (its head and its readers), `store/probe-runs/lap-20261013T231059422Z-136bec80-086/` and
  `store/probe-runs/38120951889/` (their first lines).
- `store/family_runs/retaken-2026-11-27.tsv`, `crates/w2m_vm/src/bin/family_known.rs` (the route).
- `grep -n "The version and system-info answers re-taken\|The order after Sprint 378" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266 and 301.

## signal contract

### Emits

- None declared: the change is to a guest table; the runs that judge it are the gates' existing runs.

### Invariants

- Every run as held; G3's batch runs conform, or a moved call is named; no Windows run on GitHub's runner;
  `crates/guest_runtime/src/api_ids.txt` unchanged; `specs/system_messages.json` 0.1.0 kept byte-equal.

## artifact contract

### Files created

- the table's 0.2.0 file, beside 0.1.0 as the project keeps a spec's versions (found from how `specs/` holds other
  versioned tables).

### Files modified

- the table's readers, and the `format_message` family's route row if the family is frozen; file count recorded.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The count of codes in each table, the codes whose text differs with both texts, the codes on one machine only.
  - The family runs that read the table, each conforming.
  - G3's verdict on one and two vCPUs, and any call the new text moves.
- **Planted faults:**
  - (1) One code's laptop text replaced by the runner's: the family judge or the table's test fails naming the code.
  - (2) The table's source changed to a runner run: a test fails naming it.

## done criteria

w2m's FormatMessage text is the laptop's, each code cited to the laptop's row, and the families and G3 that read it
conform.

## self-review checklist

- [x] Scope names one verifiable property: the table equal to the laptop's rows.
- [x] context_files cite the table, both runs, the route and the Decisions.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: counts, differing codes, families, G3; numbered plants.
- [x] One concept.

## result

- **The two tables.** `diff store/probe-runs/38120951889/body-1.jsonl
  store/probe-runs/lap-20261013T231059422Z-136bec80-086/body-1.jsonl` (the same binary, 4224a823...): 4,493 "dump" rows
  in each; four ids on one machine only, and no id both hold has unlike text. The runner only (Server 2025, build
  26100): 4566 "The measurement file included in the firmware update capsule has an invalid format.\r\n", 8661 "The
  Active Directory denied a modification to this object because it is managed by a cloud SOA policy.\r\n". The laptop
  only (build 26200.7840): 15680 "The source uri is blocked by MsixAllowedZones policy. Please contact your system
  administrator.\r\n", 15681 "The source uri is recommended to be blocked by SmartScreen. Please contact your system
  administrator.\r\n". -086's two bodies and both of -087's (the Server 2022 binary a2ecf624) are byte-equal; the 14
  "form" rows equal the runners' on both binaries.
- **The table at 0.2.0.** `specs/system_messages-0.2.0.json`: 0.1.0 copied by `cp`, then four Edit-tool edits (lines
  2-5: version, `windows_build` 10.0.26200.7840, `source` -086's body-1, a `summary`; 4566 and 8661 removed; 15680 and
  15681 added after 15679). `jq` of its messages against -086's dump rows: no difference, 4,493 ids. 0.1.0 `git diff
  --quiet`, exit 0.
- **Its readers.** A search for `system_messages` finds three. `crates/guest_runtime/src/headers.rs` 143-158:
  `SYSTEM_MESSAGES`, the two versions newest first, and `system_messages_table`, the newest one the root holds. The
  generator reads that one and names it in the header's first line (181). The frozen export of
  frozen-python-2026-10-29 holds 0.1.0 alone, so its key and image are the Python build's. `tools/guest_runtime/messages.c` 5-8: its comment names
  the 0.2.0 table, the code unchanged, and `crates/guest_runtime/tests/python_image.rs` DEPARTURES 17 (95-99) gives the
  reason. `probes/format_message/format_message.c` 10 is the stored binaries' source and stays unchanged. No family
  run judges the table's texts, since `format_message_rows` drops the dump rows (`crates/w2m_vm/tests/family_runs.rs`
  321). So `crates/guest_runtime/tests/system_messages.rs` judges the table, in four tests:
  `the_newest_table_is_the_laptop_runs_rows_it_names`, against every body of the run its `source` names, which must
  have the laptop's profile at 26200; `the_laptops_rows_differ_from_the_runners_as_named`, `RUNNER_ONLY` [4566, 8661]
  and `LAPTOP_ONLY` [15680, 15681], 0.1.0 itself the dump rows of 38120951889; `the_table_0_1_0_is_unchanged`, against
  3b132e94d; `the_header_is_generated_from_the_newest_table`. 4 passed.
- **The family.** format_message is frozen (`store/family_runs/known-2026-10-29.tsv` 95-96, runs 38120951889 and
  38120953179). `contracts/windows.messages.format_message/0.2.0.json` is 0.1.0's with the same 14 rows (`jq -S
  .acceptance` equal), the laptop runs -086 and -087 as evidence, the runner runs in `history`, and `open_questions`
  naming the four ids. `store/family_runs/retaken-2026-11-27.tsv` 5 adds `format_message
  windows.messages.format_message@0.2.0 lap-20261013T231059422Z-136bec80-086`. `target/release/family_known .` moves
  lines 95-96 column 4 alone to @0.2.0, and the stored lookup equals its output. `crates/windows_machine/tests/
  runner_families_on_the_laptop.rs` 126-180: a row with an `id` is compared and named by its `s` and `id` ("dump
  4566"), and rows without one line by line as before, so a row missing on one side no longer moves every row after
  it. `RETAKEN_DIFFERENCES` (428-443) adds format_message's four. Its output: "format_message: laptop
  lap-20261013T231059422Z-136bec80-086 against 1 runner runs of binary 4224a823..., 4 pairs"; version 4, files_more 4
  and system_info 24 pairs, as before. `each_retaken_contract_holds_its_laptop_runs_rows`: "format_message:
  windows.messages.format_message@0.2.0 (format_message_rows), laptop runs [-086, -087], 4 bodies conforming".
- **The runs that read the table**, with the gates' batch-run and family-run steps run alone first (gates.sh 167-207
  and 242-263, 06:03:08-06:08:47 UTC, `scratchpad/plants/380/runs-before-gates.sh`), then `W2M_REQUIRE_CASE_RUNS=1
  cargo nextest run`. Family runs: "274 of 274 family runs conform, 69 of 69 families in every run"; "on two vCPUs:
  274 of 274 held as on one vCPU, 274 of them conforming", format_message's 2 among them.
  `batch_run::the_batch_runs_calls_are_the_python_runs_up_to_the_stop` and the two-vCPU stop test pass. The
  batch-record tests pass, 9 of 9. The guests' two tests read outputs that run did not make (the guests step) and
  were left to the gates. G3 (`cargo nextest run -p call_compare`, 23 passed): "the Rust host's 10505 calls:
  conforms, 0 defects, 0 past the cut (0 for the frozen host's same rows), 0 scheduling, 25 values Windows varies";
  "the Rust host's 10555 calls on two vCPUs: conforms, 0 defects, 0 past the cut, 0 scheduling, 25 values Windows
  varies". No call moved. The frozen stream (`store/frontier-calls/frontier-643a6283510c-1792623423346201000.tsv`)
  holds 32 FormatMessageW rows from line 947, a2 (dwMessageId) 2 in 16 and 3 in 16, codes whose text both machines
  give alike. So the re-take changes no answer the game receives, and `FIRST_RETAKEN_CALL` stays GetWindowsDirectoryW,
  the cut at stream row 6364 of 10,505.
- **File count** (`WORKING_AGREEMENT.md` 301): 3 created (`specs/system_messages-0.2.0.json`,
  `contracts/windows.messages.format_message/0.2.0.json`, `crates/guest_runtime/tests/system_messages.rs`) and 6
  modified (`headers.rs`, `python_image.rs`, `messages.c`, `retaken-2026-11-27.tsv`, `known-2026-10-29.tsv`,
  `runner_families_on_the_laptop.rs`), with this card and the gates' rows.
- **Plant 1** "id 4566: in specs/system_messages-0.2.0.json, not in lap-20261013T231059422Z-136bec80-086/body-1.jsonl"
  and "id 15680: in lap-20261013T231059422Z-136bec80-086/body-1.jsonl, not in specs/system_messages-0.2.0.json"; with
  them "left: ([8661], [15681], []) right: ([4566, 8661], [15680, 15681], [])", `cargo test -p guest_runtime --test
  system_messages` exit 101; live 06:01:51-06:02:03 UTC:
  - backup `scratchpad/plants/380/system_messages-0.2.0.json.backup`, `cmp` equal; the table's row 15680 replaced by
    the runner's row 4566 with the Edit tool (`grep -c '^4566,$'` 1, `'^15680,$'` 0); judge log `plant-1.log`; restored
    by `cp`, `cmp` equal; the test again, 4 passed.
- **Plant 2** "specs/system_messages-0.2.0.json's source names run 38120951889, whose profile
  \"gha-windows-2025-x64-10.0.26100.33438-Server\" is not the laptop's", the same judge, exit 101; live
  06:02:10-06:02:17 UTC:
  - the same backup; `source` set to `store/probe-runs/38120951889/body-1.jsonl` with the Edit tool (grep, line 4);
    judge log `plant-2.log`; restored by `cp`, `cmp` equal; the test again, 4 passed, and the `jq` comparison with
    -086 equal.
- **Gates:** `exit 0 in 648 s	default	2026-11-28T06:21:33Z	sprint 380	b4f9121c23e3d5f4f0bcf825ceb8ffabc9861161	4c24125eeb3895dd7c0fb86490872eec40a8656e962dd502aa522c5b2c711349	root=e4abad5ae1afa2fbda1bdf99f112fc9d8c7ef0dc	rust=all python=0 samples=0	base=3b17c1b7897d5256320217cf6f14f72da7ef2452`
  (502 tests run, 502 passed, 1 skipped).
- **G3's two-vCPU batch run in these gates:** `run_seconds` 133.080551667 (variant vm,
  `build/batch_run/record-2-vcpus-1795846350000000000`, verdict conforms); `load_at_start` "load averages 1.61 1.88
  2.02; busy: mds_stores 98%; other w2m: none".
