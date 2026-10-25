# SIGNAL_REPORT — Sprint 133 — The guest runtime built from Rust

Made by `signal_report 133` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 133.

## 1. Observed

### scope_confirmation

> The walking skeleton's second link. A crate, `guest_runtime`, builds the C guest runtime image as
> `src/w2m/guest_runtime.py` does, with no Python: the memory layout the runtime is compiled against (the 60 defines
> `vm_backend.runtime_defines()` passes, each constant carried from `src/w2m/vm_backend.py` 80-355 and
> `src/w2m/scheduler.py` 28-49 as the same expression, cited by line); the five headers the build writes
> (`cpuid_table.h` from `specs/guest_cpuid.json`, `code_page_tables.h` from probe run 38044229348,
> `system_messages.h` from `specs/system_messages.json`, `approximation_tables.h` from `specs/approximations.json`, and
> `api_ids.h` from the services the runtime's C defines), each byte for byte as the Python wrote it; the same cache key
> (sha256 over every input); clang over the 51 runtime files and the 3 compiler files, and `ld -preload` at
> `MONITOR_VA + RUNTIME_IMAGE`; and the MH_PRELOAD image read back into segments and symbols. Its judge: the image Rust
> links equals, byte for byte, the image the Python build cached under the same key in `build/guest_runtime/`, built
> by the frozen host from the same sources. A wrong constant or a header off by one byte changes the key or the image,
> and the test fails.

### work_performed

- 4920428c5 Sprint 133: the guest runtime built from Rust

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> None. The build puts nothing on a record; the record comes with the guest (a later link).

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 133: its Rubber Duck Pass and close.
