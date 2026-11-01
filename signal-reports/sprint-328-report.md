# SIGNAL_REPORT — Sprint 328 — The host reaches a vCPU's own pages by address

Made by `signal_report 328` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 328.

## 1. Observed

### scope_confirmation

> Deferred (Sprint 327, partial) and reviewer-3's Low 1 at 327. Form B gives each vCPU past the first its own monitor
> data page, state block and runtime stack at vCPU 0's guest addresses (the per-vCPU pages design; `Guest::private_chain`,
> Sprint 315). The host reaches the first two through `VcpuPages` (Sprint 313), and a guest address through
> `Layout::offset` (`guest.rs` 280), the layout's maps alone, which are vCPU 0's: 17 calls in 7 files. The C runtime
> passes the host pointers into its own frames on the runtime stack: `GetFileAttributesExW`'s `u64 st[4]`
> (`file_attributes.c` 35-58), the host writing the stat; `FindFirstFileExW`'s `u16 full[FULL]` (`file_search.c` 113), a
> path the host may read. On vCPU 1 those land in vCPU 0's stack: `statsize` on two vCPUs prints "sizes differ".

### work_performed

- 422e47cfe Sprint 328: the host reaches a vCPU's own pages by address
- 0367e6447 Sprint 328 card: the host reaches a vCPU's own pages by address; the designs amended

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; where the host reaches a guest address.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 328: its Rubber Duck Pass and close.
