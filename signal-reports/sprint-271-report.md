# SIGNAL_REPORT — Sprint 271 — The image judge on the frozen sources

Made by `signal_report 271` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 271.

## 1. Observed

### scope_confirmation

> The Decision "Sequence for step 6" put the memory XCHG first (census D1). Its card's lookup found the C compiler inside
> the VM compiles a memory XCHG too: `store/stencil/table.txt.gz` holds 25 memory XCHG forms with stencils (32- and
> 64-bit, base, base and index, RIP-relative), lowered as the frozen host lowered them, a load and a plain store. So the
> fix reaches the C runtime, as plan step 6 says its lock changes do ("in the C runtime, changed only there", 162). Any
> change there fails `crates/guest_runtime/tests/python_image.rs`, which builds the working tree's C and requires the
> frozen Python build's image under the same key. This card moves that judge to the C sources as they stood at the tag
> `frozen-python-2026-10-29`, so it still proves the Rust build reproduces the Python build byte for byte while the
> working tree's C may change; the XCHG becomes card 272.

### work_performed

- dfc12fd11 Sprint 271: the image judge builds the C of frozen-python-2026-10-29, exported by git archive
- 61e99e467 Sprint 271 card: the image judge on the frozen sources; the step 6 Decision annotated

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a judge's reference moved, held by tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 271: its Rubber Duck Pass and close.
