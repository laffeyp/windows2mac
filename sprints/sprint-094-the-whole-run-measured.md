# Sprint 094 — the whole batch run measured on today's tree

```yaml
---
id: 094
status: closed
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

- **Tree 3d703c02**, records in `store/runs/`, `frontier-643a6283510c-` then 1792207287629103000 (warm-up),
  1792207349427127000 and 1792207411272865000 (to the stop), 1792207473041515000 and 1792207522108796000 (equal
  work); logs in `build/sprint-094/`. Every run: 168 of 168 log lines, 0
  divergences against the 11 reference runs, 24 values Windows varies.
- **At equal work**, warm cache:

  | | Sprint 070, all in the VM | work-1 | work-2 |
  |---|---|---|---|
  | load average at start | 2.0 to 3.7 | 3.45 | 3.10 |
  | instructions | 100,000,000,020 | 100,000,000,033 | 100,000,000,033 |
  | session seconds | 50.5, 51.6 | 48.7 | 44.3 |
  | seconds in the run | 39.5, 40.5 | 37.5 | 35.3 |
  | M instructions a second (session) | 1,938 to 1,980 | 2,055 | 2,259 |
  | fps-equivalent (over 41 M) | 47 to 48 | 50.1 | 55.1 |
  | host seconds at hvc 8 | — | 7.2 | 4.4 |
  | switches in the VM | 998,771 | 997,529 | 997,529 |

  The two runs did identical work (equal exits by kind, switches and thread census); they differ by 4.4 s, within
  070's 6.9 s spread. The whole run is 4 to 14% faster than 070's: no sprint since moved it far.
- **To the stop**: 61.35 to 61.38 s each, two ticks of the 30-second progress line on which the stop is checked
  (070's finding), with 131.5, 139.2 and 143.0 billion instructions retired; the seconds measure the tick, not the
  run.
- **Where the instructions go**: thread 4, the main thread, retires 63.8% of the 100 billion with 233.6 million
  PAUSEs; to the stop, 58 to 59% with 508 to 608 million. The rate counts a spin-wait as work, so the fps-equivalent
  overstates what the game gets. Step 2 (the scheduler and waits inside the VM) is where this goes.
- **Host time** at equal work: hvc 8 (host requests) 4.4 to 7.2 s, hvc 11 1.3 to 1.7 s, hvc 10 0.8 to 1.0 s.

