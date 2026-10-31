# SIGNAL_REPORT — Sprint 277 — `edit_guard` refuses every `sed -i` that changes the line count

Made by `signal_report 277` from 0 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 277.

## 1. Observed

### scope_confirmation

> Sprint 276 broke the edit rule (WORKING_AGREEMENT 291) four times, and its tail roll inserted Sprint 266's entry into the
> archive by `sed -i '343r file'`. Fed by hand to `edit_guard` on 2026-10-31, the guard refused the `\n` replacement, the
> range deletion and the Python start, and passed the `r` insertion (exit 0). The guard's `sed` check
> (`crates/edit_guard/src/lib.rs` 140-193) knows `a\`, `i\`, `c\`, an address range, a newline and a `\n` escape. Under
> `-i` sed writes its whole output back to the file, so every command that prints, reads, joins or ends early changes the
> line count. The forms the check passes:

### work_performed

- 1273eeaa2 Sprint 277: the record check's test expects edit_guard's new a, i or c reason
- ef4dc2cd6 Sprint 277: edit_guard refuses every sed -i command that changes the line count
- 25a535f53 Sprint 277 card: edit_guard refuses every sed -i that changes the line count

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a refusal, held by tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 277: its Rubber Duck Pass and close.
