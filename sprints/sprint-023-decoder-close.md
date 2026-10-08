# Sprint 023 — decoder phase close: prefix and truncation faults, per-row confirmation

```yaml
---
id: 023
status: closed
phase: 3
pass_kind: functional
---
```

## scope

Repair the two decoder defects the project review confirmed (`docs/review-2026-10-08-project.md` §4.3, §4.4) and close the decoder phase on written exit criteria. `x86dec.decode` raises `InvalidEncoding` for a 66/F2/F3/F0 or REX before VEX/EVEX/XOP and for prefixes reaching 15 bytes, and `Truncated` (a new `DecodeError` subclass) for input that ends inside an instruction; it never raises `IndexError`. `PeImage.read_rva` stops at the section's virtual end. `encgen.cases` adds prefixed VEX/XOP/EVEX cases; `generated_differential.py` stops padding input and stops counting crashes as refusals; `--confirmations` writes per-row XED confirmation keyed by row sha256.

Card history: this card was written after the code, inside the review-response turn; the review's reproductions (§4.3 table, §4.4 inputs) served as the pre-declared expectations. Cards 024-027 are written before their work.

## prerequisites

- 022

## context_files

- `sdd-kit-3/AGENTS.md`, `signals/0.3.json`, `BLACKBOARD.md`, `WORKING_AGREEMENT.md`
- `docs/review-2026-10-08-project.md` §4.1-4.4
- `src/w2m/x86dec.py`, `src/w2m/encgen.py`, `src/w2m/pe.py`, `scripts/generated_differential.py`, `scripts/gen_decoder.py`
- Intel SDM Vol 2A §2.3.2 (prefixes before VEX cause #UD); Vol 3A §6.15 interrupt 13 (#GP when length exceeds 15); Microsoft "PE Format", Section Table (zero padding past SizeOfRawData)

## signal contract

### Emits

- none. The differential's signal-plane emission is Sprint 025.

### Consumes

- the files above

### Invariants

- `decode` raises only `DecodeError` and subclasses, on any input.
- No caller feeds `decode` bytes the image does not contain.

## artifact contract

### Files created

- `specs/x86_decode_confirmations.json` (derived)
- `sprints/sprint-023-decoder-close.md`, `signal-reports/sprint-023-report.md`

### Files modified

- `src/w2m/x86dec.py`, `src/w2m/encgen.py`, `src/w2m/pe.py`, `scripts/generated_differential.py`, `scripts/gen_decoder.py`, `specs/x86_decode.json` (0.6.2), `src/w2m/x86dec_tables.py` (generated), `specs/x86_generated_worklist.json` (derived), `tests/test_x86dec.py`, `tests/test_pe.py`, `BLACKBOARD.md`, `WORKING_AGREEMENT.md`

Hard rule 6 exception: two source files carry the concept (`x86dec.py`, `encgen.py`); the rest are their tests, generated or derived outputs, and records. Recorded here as Sprint 010 recorded its own.

### Content assertions

- `66 c5 f8 77`, `f3 c5 f8 77`, `f2 c4 e1 7d 58 c0`, `66 62 f1 7c 48 58 c0`, `40 c5 f8 77`, and `66`×15 raise `InvalidEncoding`; `2e c5 f8 77` and `67 c5 f8 77` decode (XED agrees on all eight).
- `""`, `66`, `0f`, `48`, `c5`, `e8 00 00 00`, `48 8b 04 25` raise `Truncated`.
- `gen_decoder.py` refuses a spec with a duplicate row.

### Command exit codes

- `uv run pytest` → 0 (140 passed); `uv run ruff check src scripts tests` → 0; `uv run mypy` → 0; `python3 scripts/gen_decoder.py --check` → 0

## observation contract

### Expected log substrings

- `generated_differential.py`: `length_differs 0`; no traceback.
- `decoder_differential.py` on the corpus and probes: `ours length differs 0; ours iclass differs 0`; `spec gaps 0`.

### Observed

| generated cases | before (committed decoder) | after |
|---|---|---|
| prefixed VEX/XOP/EVEX cases XED refuses, ours accepts | 1,170 of 2,813 | 172 of 2,813 |
| all cases | — | 41,616: agree 68.73%, both reject 19.79%, ours accepts 10.79%, ours gap 0.18%, length 0, iclass 0.52%; operands 11,902 compared, 0 differ |

The 172 remaining are existing residue classes (VEX W/L constraints, XED GENERAL_ERROR; EVEX zeroing without a mask) behind a permitted 2E or 67 prefix. XED decodes 466 prefixed cases, all with 2E or 67 before the escape.

Corpus (2,391,671) and probes (52,785): length and iclass equal XED at every position; spec gaps 0.

The spec held three duplicate rows (vpunpckldq, vpunpckhdq, vpunpckhqdq), found when confirmations were keyed by row hash; removed in spec 0.6.2. Confirmed by XED: 548 of 809 rows.

## done criteria

The decoder refuses, as XED does, every prefix-before-VEX, over-length, and truncated case named above; exit criteria (a)-(c) hold and are written down.

## notes

Review claim 4.4 holds in part: `read_rva` zero-fill between SizeOfRawData and VirtualSize is what the Windows loader maps (PE Format, Section Table), so it stays; only bytes past the section's virtual end were invented, and those are gone.

## self-review checklist

- [x] Scope is concrete and bounded.
- [x] `context_files` covers the SDM and PE Format sections relied on.
- [x] Signal contract's `Emits` references only vocabulary tags (none).
- [x] Artifact contract is gradable.
- [x] Observation contract present.
- [ ] ≤2 files: exception recorded above.
