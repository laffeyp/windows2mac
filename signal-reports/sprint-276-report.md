# SIGNAL_REPORT — Sprint 276 — Guest memory on several vCPUs

Made by `signal_report 276` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 276.

## 1. Observed

### scope_confirmation

> Reviewer-3's Medium 1 at 275: the host reaches guest memory through `GuestMemory::as_slice` and `as_mut_slice`
> (`crates/hypervisor/src/lib.rs` 355-361), a `&[u8]` or `&mut [u8]` over the whole mapping, and since Sprint 274 other
> vCPUs write that mapping while such a reference lives. Rust's rules make that undefined: "data races are undefined
> behavior. A data race is defined as conflicting non-synchronized accesses where at least one of the accesses is
> non-atomic" (`std::sync::atomic`), and volatile is no remedy, "whether an operation is volatile has no bearing
> whatsoever on questions involving concurrent accesses from multiple threads" (`std::ptr::read_volatile`). The
> virtual-machine monitors that share guest memory with running vCPUs keep two rules, which `vm-memory` (Firecracker's
> `vmm` and Cloud Hypervisor, both 0.18.0) and crosvm's `base/src/volatile_memory.rs` state alike: "No references or
> slices to volatile memory (`&` or `&mut`)", because "having references of any kind to memory considered volatile would
> violate pointer aliasing", and words shared with running code are reached by atomics (`vm-memory`'s
> `get_atomic_ref`, `load`, `store`).

### work_performed

- 545d19c49 Sprint 276: guest memory reached with no reference while other vCPUs run
- c0346fe0d Sprint 276 card: guest memory on several vCPUs

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

See the BLACKBOARD tail entry for Sprint 276: its Rubber Duck Pass and close.
