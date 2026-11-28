# SIGNAL_REPORT — Sprint 375 — G3's reference set chosen by its rules, so laptop game runs can join it

Made by `signal_report 375` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 375.

## 1. Observed

### scope_confirmation

> G3 holds the game's batch run to a set of eleven Windows runs: runner run 38216081440 and the laptop runs
> `lap-20261014T032442034Z-c8814547-01` to `-10`. The set is fixed: `crates/call_compare/src/lib.rs` 1251-1253, "the
> membership rules, which read laptop and runner run files and records, are not ported; the set is the frozen host's
> last choice". So no laptop game run made since 2026-10-14 can join it (the postmortem's next steps; the Decision "The
> order after the stop, the cards that take each deferred item", card 375).

### work_performed

- 75bcffba3 Sprint 375 card amendment: Emits none on today's store, every game run the rules read already on the record; the record step held to the 38 frozen records
- b7269ac26 Sprint 375 card: G3's reference set chosen by the frozen host's rules from the stored game runs, game runs put on the record, held to the frozen choice

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None on today's store (amendment, builder-375's finding): every stored game run the rules read, 38 of the 44, is
>   already on the record (`store/runs/game-run-<run>/`, written by the frozen `record_game_run.py`), and a record is
>   never written over, so the record step writes none. Its execution start and completion payloads, built
>   from each stored run, are held field for field to those 38 frozen records by a test; the step's first records are
>   card 376's new laptop game runs. No other emit site changes.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 375: its Rubber Duck Pass and close.
