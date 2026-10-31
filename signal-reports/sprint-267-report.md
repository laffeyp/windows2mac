# SIGNAL_REPORT — Sprint 267 — The guard over both standing rules

Made by `signal_report 267` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 267.

## 1. Observed

### scope_confirmation

> reviewer-3's Medium 1 at 266: the user's rule "No Python runs" was broken by a Python start in Sprints 238, 240, 250
> and 266, and the rule that multi-line edits use the Edit tool in 262, 263 and 266, each breach recorded and the rate
> unchanged. `crates/edit_guard`, the PreToolUse hook program built at Sprint 195, refuses a heredoc written to a project
> file and a multi-line `sed -i`, and refuses neither an in-place `perl` or `awk` edit nor a Python start. This card
> gives it both, so the hook, once the user installs it, stops both rules' breaches before they run.

### work_performed

- d9c467292 Sprint 267: edit_guard refuses a Python start and an in-place perl or awk edit
- cc289e418 Sprint 267 card: the guard over both standing rules; reviewer-3's review at 266 in Surfaced

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a hook's refusals, held by tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 267: its Rubber Duck Pass and close.
