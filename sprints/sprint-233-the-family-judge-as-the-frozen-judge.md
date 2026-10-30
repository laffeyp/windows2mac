# Sprint 233 — The family judge as the frozen judge

```yaml
---
id: 233
status: done
phase: 6
pass_kind: functional
---
```

## scope

Plan step 4's "68 of 68 families", and a fault found by lookup at Sprint 232's close: the Rust family judge
(`crates/w2m_vm/tests/family_runs.rs`, from Sprint 196) is weaker than the frozen judge it stands for. So the 249
conforming runs counted in Sprints 230 and 231 were judged by a weaker rule.

- **Where it stands, by lookup (2026-10-30):**
  - The frozen judge, `family_run.py` 3141-3153 at the tag frozen-python-2026-10-29:
    - It compares `derived_output(f, out)`, the rows after the family's rule with its excluded rows dropped
      (`Family.rows`, 116-118), by `compare_exact_rows` (`src/w2m/compare.py` 168-203 at the tag).
    - `compare_exact_rows` takes the rows in the contract's order. The first missing row, wrong name in its place,
      unequal field (either side's keys) or extra row is the divergence.
    - A run that stops on its family's `expected_stop` (a prefix of the stop) is judged on its rows. Any other stop
      but ExitProcess is a discrepancy.
  - The Rust judge keys the printed rows by name in a map, so repeated names collapse and order is not read. It
    ignores extra rows and drops no excluded row. It knows no expected stop, so the 14 runs that stop on one are held
    as stops.
  - `family_known` counts a contract's excluded rows by splitting its text at "; ", and the reasons hold "; " too
    (`windows.file.attributes`: 2 rows, counted 4).
  - The tag names each family's excluded rows in a dictionary (593, 772, 838, 1331, 1484, 1518, 1550, 1611, 1705,
    2057) and four expected stops (438, 775, 1333, 1713).
  - `output_debug_string` derives its rows by `numbered_exceptions` (77-81), not ported.
- **The work:**
  - `compare_exact_rows` ported, its divergence text as written there.
  - The extract gains each family's excluded row names and expected stop, entered from the cited lines and held to
    them by `the_extract_start_options_are_the_tags`, widened to these.
  - `family_known` prints the names in column 8, the count read from them.
  - The judge drops excluded rows after the rule, and judges a run whose stop is its family's expected one.
    - The frozen stop "Unimplemented: X" is the Rust host's "HostRequest op 4: X argument form …".
    - "Unimplemented: import D!F" is its "import D!F".
  - `numbered_exceptions` ported.
  - Every count is taken again, and each run that no longer conforms is named with its first divergence.
- **If the frozen judge is wrong, its wrong parts are not reproduced:** a difference the strict rule finds is read
  against the frozen record's verdict and the Windows runs before it is called a Rust defect.

## prerequisites

- 232 (closed)

## context_files

- `family_run.py` and `src/w2m/compare.py` at the tag (the lines above)
- `crates/w2m_vm/tests/family_runs.rs`, `crates/w2m_vm/src/bin/family_known.rs`
- `store/frozen_python/family-run-2026-10-29.tsv`, `store/family_runs/`, `docs/step-4-census-2026-10-28.md` (row 6)

## signal contract

### Emits

- None: the families' runs write no record on the Rust host.

### Invariants

- The judge is the frozen judge: the same rows, the same order, no extra row, the same stops accepted.

## artifact contract

### Files modified

- `family_runs.rs`, `family_known.rs`, the extract, the lookup, the held file, census row 6

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** runs conforming of 273 and families in every run, against 249 and 60. Each run that no longer
  conforms is named with its first divergence. The 14 expected-stop runs are judged.
- **Planted faults:**
  1. A row appended to one run's printed rows: "rows past the reference's".
  2. Two rows of one run swapped: "target printed … in its place".
  3. An excluded name removed from the extract: the tag test names it.

## done criteria

The Rust family judge is the frozen judge, and every count of step 4's family item is taken by it.

## self-review checklist

- [x] Scope names one verifiable property: the Rust family judge is the frozen judge.
- [x] context_files cite the frozen judge and comparator, the Rust judge, the lookup and the extract.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the family judge.

## result

- **The comparison:** `exact_rows` is `compare_exact_rows` (`compare.py` 168-203; the card first cited 168-210, then
  168-206, before the function's last line, 203, was read). It takes the contract's rows in order. The first missing
  row, wrong name in a row's place, field unequal on either side's keys, or extra row is the divergence, in its
  words. Values are compared by `py_eq`, Python's `==` (`True == 1`, `1 == 1.0`), where `serde_json` holds them
  apart.
- **The exclusions and the expected stops:**
  - The extract's eleven family rows gained two columns, held to the tag by `the_extracts_family_rows_are_the_tags`
    (renamed from `the_extract_start_options_are_the_tags`, which it widens). The new table `EXCLUDED` gives each
    family's `name=` line, its `excluded={` line and each name on its line. `SHOWN` gains the four expected stops.
  - `family_known` prints them as columns 9 and 10 and counts the exclusions from them. The contract-text count had
    been wrong for 8 families, 25 runs; `file_attributes`' 2 rows, for one, had been counted 4.
  - The judge drops excluded rows after the rule. A run whose stop, named as the frozen host names it (`frozen_stop`:
    "HostRequest op 4: X" is "Unimplemented: X", "import D!F" is "Unimplemented: import D!F"), begins with its
    family's expected stop is judged on its rows.
- **Two rules ported:** `numbered_exceptions` (77-81) and `known_folders_rows` (1683).
- **The counts under the frozen judge:**
  - All 249 runs that conformed under the weaker judge still conform.
  - The 14 that stop on their family's expected stop (thread_storage 8, output_debug_string 4, known_folders 2)
    conform.
  - So 263 of 273 runs conform, from 249, and 63 of 68 families in every run, from 60. The 10 held all stop on the
    warp to a deadline.
- **The unit test,** `the_comparison_is_the_frozen_comparison`, holds the four divergences, Python's equality and the
  two stop names.
- **Plant 1** (a row `{"s":"extra"}` appended to code_pages 38044061149's rows): "code_pages 38044061149 ended; 1 rows past the reference's 3827 (windows.code_pages.ansi_1252@0.2.0)".
- **Plant 2** (that run's first two rows swapped): "ended; row \"acp\": target printed \"oemcp\" in its place (windows.code_pages.ansi_1252@0.2.0)".
- **Plant 3** (`limit.warm.1` removed from heap_in_place's row in the extract): "heap_in_place excluded: store/frozen_python/family-run-2026-10-29.tsv has [\"limit.fresh.800\",\"limit.fresh.1000\"], frozen-python-2026-10-29 [\"limit.fresh.800\",\"limit.fresh.1000\",\"limit.warm.1\"]".
- Each plant was restored by `cp`, and `cmp` found the files equal to their copies.
