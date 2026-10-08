# SIGNAL_REPORT — Sprint 023

## 1. Observed

### scope_confirmation

> Made the decoder refuse what XED refuses for prefixes before VEX/EVEX/XOP, length over 15, and truncated input; stopped the measuring tools from inventing bytes or scoring crashes as refusals; recorded per-row XED confirmation; wrote the decoder exit criteria.

### work_performed

- Added the prefix and REX check before VEX/EVEX/XOP, the 15-byte prefix limit, and `Truncated` to `src/w2m/x86dec.py`.
- Bounded `PeImage.read_rva` at the section's virtual end.
- Added prefixed VEX/XOP/EVEX cases to `encgen.cases`; removed padding and the `IndexError`/`KeyError` catch from `generated_differential.py`; added `--confirmations`.
- Found and removed three duplicate spec rows; `gen_decoder.py` now refuses duplicates.

### signal_trace

No runtime emission (the differential joins the signal plane in Sprint 025).

### signal_trace_tags

- (none)

## 2. Expected

> The card declared no `Emits`.

## 3. Delta

### dual_contract_self_grade

**signal contract:** vacuous by declaration; the gap it leaves is Sprint 025's scope. *pass*

**artifact contract:**

- 14 named encodings behave as declared — `tests/test_x86dec.py` [pass]
- `read_rva` stops at the section end — `tests/test_pe.py::test_read_rva_stops_at_section_end` [pass]
- `uv run pytest` → 0, 140 passed; ruff 0; mypy 0; `gen_decoder --check` 0 [pass]

**overall:** pass

## 4. Hypothesis

> The generator found only what it was built to vary; the review found a class it did not vary by reading the SDM, not by running more cases. Case generation should be derived from the SDM's own list of encoding fields (prefix groups, REX, escape, ModRM, SIB, displacement, immediate) rather than from per-row habits.
