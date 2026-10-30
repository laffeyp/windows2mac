# Sprint 228 — The Python words fixture as a release asset

```yaml
---
id: 228
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "No Python runs" and reviewer-3's Medium 2 of its review at Sprint 226: the Python words fixture, whose
only copy is this Mac's since its generator left the tree.

- **Where it stands, by lookup (2026-10-29):**
  - `build/case_runner/python-words.txt` is 3,107,809 bytes and gitignored.
  - `store/case_runner/python-words.sha256` pins it (a20dc6ef…), and this Mac's copy matches the pin.
  - Its generator, the frozen lowering in `src/w2m`, left in Sprint 226.
  - The class judge and the case runner compile 6 conforming and 19 inconclusive classes from it, until the card for
    the 17 classes and the 6 retires it.
  - No release holds it. The only release other than the case sets' is `game-1699230-24247624`.
- **The work:**
  - A release `python-words-2026-10-29` holds the fixture, checked against the pin before the upload.
  - A script, `scripts/fetch-python-words.sh`, downloads it into `build/case_runner/` and checks it against the pin
    with `shasum -a 256`. `Fixture::load`'s "None when it is absent" names that script.
- **Not here:** retiring the fixture (the card for the 17 classes and the 6); the removed records' index (Sprint 229).

## prerequisites

- 227 (closed)

## context_files

- `crates/w2m_vm/src/case_inputs.rs` (`Fixture::load`), `store/case_runner/python-words.sha256`, `.gitignore`

## signal contract

### Emits

- None; an input.

### Invariants

- The asset's sha256 equals the pin.

## artifact contract

### Files modified

- the release's asset, `scripts/fetch-python-words.sh`, `case_inputs.rs`' message

### Command exit codes

- `scripts/gates.sh` returns 0; the script, run with the fixture moved away, restores it and exits 0

## observation contract

- **Expected values:** the asset's size and sha256 equal to the pin; the fetched file equal byte for byte.
- **Planted faults:**
  1. A wrong pin: the script names the mismatch and exits non-zero.
  2. A truncated download (a scratch copy checked by the script's check): named.
  3. The asset's name wrong in the script: `gh` names the missing asset, and the script exits non-zero.

## done criteria

The Python words fixture has a source other than this Mac, fetched and checked by its pinned hash.

## self-review checklist

- [x] Scope names one verifiable property: the fixture reachable without this Mac.
- [x] context_files cite the loader and the pin.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the fixture's source.

## result
