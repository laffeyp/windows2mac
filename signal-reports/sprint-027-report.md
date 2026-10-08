# SIGNAL_REPORT — Sprint 027

## 1. Observed

### scope_confirmation

> Brought the records in line with the code, put every gate on CI with nothing allowed to skip, and fuzzed the two parsers of untrusted bytes.

### signal_trace

No harness emission. CI run 37851288883: every step success; `150 passed in 22.10s`.

## 3. Delta

### dual_contract_self_grade

**signal contract:** vacuous by declaration (records, CI, tests). *pass*

**artifact contract:** tail 10 / archive 18; no "pending" mapping for a used SDK; no money word in the agreement; CI success with 0 skips; local gates 0, run unpiped. *pass*

## 4. Hypothesis

> The first fuzz run passed and the second failed on the same property: 3,000 examples is near the edge of what finds a one-byte header defect. CI runs the default seed each push; a scheduled run with a larger example count would cover more of the space without slowing pushes.
