# SIGNAL_REPORT — Sprint 137 — The PE reader in Rust

Made by `signal_report 137` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 137.

## 1. Observed

### scope_confirmation

> The walking skeleton's sixth link, and loading's (S2) first piece: the guest program is a PE file, and the host reads
> it before it can place it. A crate, `pe_image`, carries `src/w2m/pe.py` item for item: the headers, the sections, the
> imports and delay imports with each IAT slot named, the base relocations, the exports with their forwarders and
> ordinals, the TLS directory and callbacks, the `.pdata` function table, the resources, `read_rva`, `resource` and
> `code_targets`. Malformed input is refused with the Python's message. Its judge is the Python's
> (`tests/test_pe.py`): for every executable under `store/binaries/` (268), the entry RVA, the image base and the imports
> equal llvm-objdump's reading of the same file; the structure checks hold (the `.text` section `r-x` and holding the
> entry, only DIR64 and padding relocations, the entry among the code targets, the imports a probe may make); the three
> malformed inputs and the i386 machine are refused with the Python's words; `read_rva` stops at a section's end; the
> delay imports of `corpus/python-3.14.8/_wmi.pyd` are the five `ole32.dll` names; the relocation sites match their
> counts.

### work_performed

- 646bbc4b0 Sprint 137: the PE reader in Rust

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> None yet: the skeleton's record comes with the guest program.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 137: its Rubber Duck Pass and close.
