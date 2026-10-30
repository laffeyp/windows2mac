# Sprint 239 — The file bridge's typed error

```yaml
---
id: 239
status: done
phase: 6
pass_kind: functional
---
```

## scope

BLACKBOARD Deferred 2026-10-29 (the helpers' errors; reviewer-3's Medium 8 of its review at 219 and Medium 6 at 221,
"the file bridge's typed error is the first helper carded"): `file_bridge` returns its refusals and its failures as one
string, and `host_request` stops every one as a fault, `HostError::Failed`.

- **Where it stands, by lookup (2026-10-30):**
  - The frozen `src/w2m/file_bridge.py` at the tag frozen-python-2026-10-29 raises `Unimplemented` (a request w2m does
    not serve) at 135, 137, 156, 163, 180, 213, 217, 222, 285, 326, 328, 355 and 357: a change under the read-only
    C:\w2m\app or in a recorded folder, a recorded file known by name only or whose copy is missing, a name in a folder
    with no recorded listing, another drive, a rooted or UNC name, "..", the listing of a folder recorded without one,
    a write or truncation of a recorded or read-only file. It raises `BridgeRequestError` (a runtime defect) at 199
    and 314, a NUL in a path and a descriptor not open; `ValueError` at 360, an unknown primitive; and lets `OSError`
    from the host's calls and FILE_SIZE's `assert` (351) through, failures each.
  - `crates/w2m_vm/src/file_bridge.rs` carries each of these as a `String`.
- **The readers, by search (2026-10-30; WORKING_AGREEMENT, a card lists its readers first):**
  - `host_request.rs` 540-542: `files.request(..).map_err(HostError::Failed)`, every refusal a fault.
  - `host_request.rs` `map_view` 304-309: `view`'s error, a host read's, `HostError::Failed`.
  - `modules.rs` 155 (`load_dll`) and 394 (`module` through `AppDirectory::host_file`, 253-260): `host_file`'s error,
    every one `ModuleError::NotServed`; `host_file` fails only in `parts`, refusals each.
  - `bin/w2m-vm.rs` 318, 546, 608: `FileBridge::new`, the run's set-up, not a request; its `String` stays.
  - Stored stops: no file outside `store/runs/` holds a bridge message as a stop; `family_runs.rs` 1260 holds one as a
    string for `steady`'s rewriting, which the kind does not change.
- **The work:**
  - `FileError { NotServed, Failed }` in `file_bridge.rs`, each site the kind the frozen bridge gives it; `request`,
    `host_file`, `view` and `parts` return it.
  - `From<FileError>` for `HostError` and for `ModuleError`, so a refusal reaches `HostError::NotServed`, the run's
    `Stop::HostRequest`, and a failure `Stop::HostFailure`.
  - `HostError::Failed`'s comment names `kernel` and `threads` alone as the helpers still returning one string.
  - A test of each reachable site's kind through `request`, and of the two conversions.
- **Not here:** `kernel` and `threads`, a card each, as the Deferred item orders.

## prerequisites

- 238 (closed)

## context_files

- `crates/w2m_vm/src/file_bridge.rs`, `host_request.rs`, `modules.rs`; `src/w2m/file_bridge.py` at the tag

## signal contract

### Emits

- None: a host helper's error type, with no record of its own. The batch run's record, written by the gates, is
  unchanged: its stop is the reference's.

### Invariants

- Every family run, guest and the batch run end as before; no stored stop moves.

## artifact contract

### Files modified

- `crates/w2m_vm/src/file_bridge.rs`, `host_request.rs`, `modules.rs`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** 13 refusal sites `NotServed` and the failure sites `Failed`, as the frozen bridge raises them;
  the family runs 273 of 273 and the guests 16 of 16 as in the gates.
- **Planted faults:**
  1. The read-only mount's refusal given `FileError::Failed`: the bridge's test names it.
  2. `From<FileError> for HostError` sending a refusal to `HostError::Failed`: the conversion's test names it.
  3. A descriptor not open given `FileError::NotServed`: the bridge's test names it.

## done criteria

A file request the frozen bridge refuses stops the run as not served, and one that fails stops it as a fault.

## self-review checklist

- [x] Scope names one verifiable property: each bridge error the kind the frozen bridge gives it.
- [x] context_files cite the bridge, its readers and the frozen source.
- [x] Emits: none, stated with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the file bridge's typed error.

## result

- **The type:** `file_bridge::FileError { NotServed, Failed }`, with no conversion from a string, so each site names
  its kind where it arises, as `ModuleError` does. `request`, `host_file`, `view` and `parts` return it;
  `FileBridge::new` keeps its `String`, a run's set-up and not a request.
- **The kinds:** each of the frozen bridge's 13 `Unimplemented` raises is `NotServed` here; 156's and 163's each
  cover two conditions, which the Rust tests apart and answers with one message from one closure. Its two
  `BridgeRequestError`s, the unknown primitive's `ValueError`, FILE_SIZE's `assert`, every host call's error and every
  guest-memory read or write are `Failed`. So is "a recorded file outside the drive", which the frozen bridge reaches
  as `assert parts is not None` (152).
- **The readers:** `From<FileError>` for `HostError` and for `ModuleError`. `host_request`'s file arm and `map_view`
  pass the kind through; before, every file request's error was `Failed`, a fault. `load_dll` and
  `AppDirectory::host_file` pass it through too; before, every one was `NotServed`, a failure included.
  `app_parts`' three errors without a bridge stay `NotServed`, each one of the frozen bridge's refusals.
  `HostError::Failed`'s comment names `kernel` and `threads` alone.
- **The tests:** `each_error_is_the_kind_the_frozen_bridge_raises` drives seven refusals and four failures through
  `request`; `a_file_refusal_is_not_served_and_a_failure_is_a_fault` holds both conversions. The recorded-folder sites
  need `specs/recorded_folders.json`'s copies and are not driven; their kinds are read from the frozen source above.
- **Departures:** none in files; no stored stop moved (the search the card names).
- **The gates:** `scripts/gates.sh` exit 0 in 489 s on 91177ad59, "rust=crates python=0 samples=0": `w2m_vm` and
  its dependants, 103 of 103 tests, the family runs and guests among them; the batch run's record finalised.
- **Plant 1** (the read-only mount's refusal, `file_bridge.rs` 200, given `FileError::Failed`): "C:\w2m\app\Data\new:
  the frozen bridge refuses it, and here Failed("a change under the read-only C:\\w2m\\app: …/Data/new")".
- **Plant 2** (`From<FileError> for HostError` sending `NotServed` to `HostError::Failed`): "assertion failed:
  matches!(HostError::from(refusal()), HostError::NotServed(m) if …" in
  `a_file_refusal_is_not_served_and_a_failure_is_a_fault`.
- **Plant 3** (a descriptor not open, `file_bridge.rs` 527, given `FileError::NotServed`): "not open: the frozen bridge
  fails it, and here NotServed("file primitive 9 on descriptor 12, which is not open")".
- **Plants that did not land:** the first try at plants 1 and 3 changed nothing. Plant 1's `grep` took the enum's doc
  line for the site, and plant 3's `sed` named a line rustfmt had split; both tests passed on the unchanged file. An
  empty `git diff --stat` and a `grep -c` of 0 showed it; both ran again by line number, each `git diff` showing the
  one line changed. Each plant was restored by `cp`, and `cmp`
  found the files equal to their copies.
