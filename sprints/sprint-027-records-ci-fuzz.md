# Sprint 027 — records repair, CI, fuzzing

```yaml
---
id: 027
status: closed
phase: 3
pass_kind: functional
---
```

## scope

Close the review's remaining items (§3.1 rules 6, 7, 10; §3.2 records; §4.9 last item; §5 CI and fuzzing; §6.8 phase acceptance). `WORKING_AGREEMENT.md`: bridge mappings for Hypervisor.framework, Capstone, XED; Windows oracle line answered; oracle-first technique amended to the 2026-10-07 user decision; no money word in the cadence policy; acceptance evidence per phase; hand-author log entry for Sprint 000's `check_vocab.py`; Hypothesis as a permitted dependency. `BLACKBOARD.md` Sprint tail rolled to 10, older entries moved verbatim to `docs/sprint-tail-archive.md`. `.github/workflows/ci.yml` runs every gate on each push with XED (pinned commits), llvm-objdump and the Python corpus present, and `W2M_REQUIRE_REFERENCES=1` makes any skipped test fail. `tests/test_fuzz.py` holds Hypothesis properties: `x86dec.decode` raises only `DecodeError` and returns length ≤ min(15, input); `pe.parse` raises only `PeError` on mutated real images.

## prerequisites

- 026

## context_files

- `docs/review-2026-10-08-project.md` §3, §5, §6; `sdd-kit-3/templates/BLACKBOARD.md` (tail of 10), `TECHNIQUES.md` #46
- `WORKING_AGREEMENT.md`, `BLACKBOARD.md`, `tests/`, `.github/workflows/windows-probe.yml`

## signal contract

### Emits

- none (records, CI configuration, tests).

## artifact contract

### Files created

- `.github/workflows/ci.yml`, `tests/conftest.py`, `tests/test_fuzz.py`, `docs/sprint-tail-archive.md`

### Files modified

- `WORKING_AGREEMENT.md`, `BLACKBOARD.md`, `pyproject.toml`, `uv.lock`

Hard rule 6 exception: records and CI are one concept (the review's process findings); recorded here.

### Content assertions

- `BLACKBOARD.md` `## Sprint tail` holds 10 entries; the archive holds the rest verbatim.
- `WORKING_AGREEMENT.md` has no "pending" bridge mapping for an SDK already used, no "pending user answer (Q2)", and no money word outside quotations.

### Command exit codes

- local: `uv run pytest` → 0; ruff, mypy → 0 (each run unpiped)
- CI: the `ci` workflow run on the sprint's commit concludes `success` with 0 skipped tests

## observation contract

### Expected log substrings

- CI log: `passed` with no `skipped`; XED kit built at commit 0bcb6237345c.
- Fuzz: either 0 failures, or each failure is a real defect reproduced and fixed in this sprint.

## done criteria

The records say what is true now, and every push runs the full gate list with nothing skipped.

## self-review checklist

- [x] Scope concrete. [x] Gradable. [x] Observation declared before running. [ ] ≤2 files: exception recorded.

## observed

- CI run 37851288883 on commit of this sprint: conclusion success in 1 min 53 s; XED built at 0bcb6237345c (`xed-install-base-2026-10-08-lin-x86-64`); corpus zip matched its manifest sha256; pytest `150 passed`, 0 skipped under `W2M_REQUIRE_REFERENCES=1`.
- Fuzzing: `pe.parse` raised `UnicodeDecodeError` on a non-ASCII DLL name (header byte 276 flipped); fixed. Decoder properties hold over 5,000 + 5,000 examples, seeds 1-3; aimed at the pre-023 decoder, the first property finds its `IndexError`.
- Tail: 10 entries kept (018-027), 18 archived verbatim.
