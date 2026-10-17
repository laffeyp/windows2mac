# Sprint 094 — the whole batch run measured on today's tree

```yaml
---
id: 094
status: active
phase: 5
pass_kind: functional
---
```

## scope

Step 1 of the roadmap (`docs/plan.md`, "Do next"; the full review of 2026-10-16): Hail to the Rainbow's batch run
under w2m on today's tree, measured whole, so steps 2 to 5 have a figure to move. No code change is planned; a defect
the runs show is fixed here only if the run cannot finish without it.

**Known first.**

- The last whole-run figures are Sprint 070's (bc6c956c, 2026-10-14): 62.42 s warm to the frontier's stop (a figure
  070 withdrew, since the stop is checked only on the 30-second progress tick, so a faster run goes further), and at
  equal work, 100,000,000,020 instructions with a warm cache, 50.5 and 51.6 s a session (39.5 and 40.5 s inside the
  VM), with the warp and switches in the VM.
- Two runs of one configuration differed by up to 6.9 s with identical work (load average 2.0 to 3.7 at their
  starts), so each figure is two runs with the Mac otherwise idle, its load average recorded.
- The fps-equivalent's divisor: about 41 million instructions a frame (Sprint 068, from the laptop's play: 9,122
  Presents).
- The run: `scripts/run_frontier.py "<depot>/Hail to the Rainbow.exe"`, its end report giving session seconds,
  seconds inside the VM, exits by kind and switch counts (d6edf5ca).

## the measure

- **To the stop**, warm cache, twice: instructions, session seconds, conformance (168 of 168 log lines, 0
  divergences).
- **At equal work**, `--max-steps 100000000020`, warm cache, twice: session seconds, seconds inside the VM,
  instructions a second, the fps-equivalent (instructions a second over 41 million), hvc counts and host seconds by
  kind, switches in the VM. Compared with 070's column "all in the VM".

## prerequisites

- 093 closed (the tree the runs measure).

## context_files

- docs/plan.md; sprints/sprint-070-warp-in-the-vm.md; sprints/sprint-068-speed-to-playable.md;
  scripts/run_frontier.py

## signal contract

### Emits

- None new.

### Invariants

- Vocabulary unchanged.

## artifact contract

- The four runs' records in `store/runs/` from a committed tree; the card's table; `scripts/gates.sh` exit 0; CI
  `success` on the closing commit.

## observation contract

- The run to the stop conforms: 168 of 168 lines, 0 divergences.
- The equal-work runs retire 100,000,000,020 instructions each; the table gives each figure from both runs, with the
  load average at each start.

## done criteria

The whole run measured on today's tree in both forms, compared with Sprint 070, with CI green.

## observed
