# SIGNAL_REPORT — Sprint 279 — The view's copies tear but never race

Made by `signal_report 279` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 279.

## 1. Observed

### scope_confirmation

> Reviewer-3's Lows 1 and 3 at 278. `ConcurrentGuestMemory::read` and `write` (`crates/w2m_vm/src/concurrent_memory.rs`)
> copy with `copy_nonoverlapping`, a plain access, which the design (`docs/design-guest-memory-on-several-vcpus-2026-10-31.md`
> choice 2) allows only for bytes "no running vCPU writes at the time". The next module, `host_request.rs`, copies the
> guest buffers a host request names (a file read's destination, a path), which the guest's other threads may write while
> the host copies, as they may on Windows. Under Rust's model that race is undefined behaviour, not torn data: "data races
> are undefined behavior … at least one of the accesses is non-atomic" (`std::sync::atomic`). RFC 3301, "AtomicPerByte
> (aka atomic memcpy)" (https://github.com/rust-lang/rfcs/pull/3301, open, not merged), states the remedy: "All of the data
> would have to be written and read through atomic operations to prevent a data race. We don't need the atomicity of the
> data as a whole though; it's fine if there's tearing", and notes the `seqlock` crate's volatile copy "is technically
> undefined behavior"; C++'s P1478 `atomic_load_per_byte_memcpy` is "effectively a series of `AtomicU8::load`s", in any
> order and not necessarily a byte at a time.

### work_performed

- 54a6d4c01 Sprint 279: the view's copies tear but never race
- d5cf7f1c2 Sprint 279 card: the view's copies tear but never race

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; an access path, held by tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 279: its Rubber Duck Pass and close.
