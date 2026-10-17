# Sprint 113 — the host for several vCPUs, designed twice

```yaml
---
id: 113
status: done
phase: 5
pass_kind: design
---
```

## scope

Step 1 resumes (docs/plan.md, after the SDD hardening): the design sprint the hardening order named, in
`docs/design-several-vcpus.md` "The host for several vCPUs, designed twice". Two designs (one lock held by whoever
serves; a process server) over what both share (a `Vcpu` object passed to each handler, one exit handler, a kick
through `hv_vcpus_exit` in place of the 50 ms cut), a choice with its revisit, and the chain of sprints.

## prerequisites

- 112; Sprint 107 parked (Decision 2026-10-17).

## context_files

- docs/design-several-vcpus.md; docs/review-2026-10-17-sdd-hardening.md §3; branch wip/sprint-107-host;
  src/w2m/vm_backend.py (`steps`); tools/vm_monitor/vm_monitor.c (`watchdog`, `run_vcpu`); the SDK's
  Hypervisor.framework hv_vcpu.h (hv_vcpus_exit, hv_vcpu_run).

## signal contract

### Emits

- None: a design sprint; no run.

### Invariants

- Vocabulary 0.9 unchanged.

## artifact contract

- The design section; `scripts/gates.sh` exit 0; CI `success` on the closing commit.

## observation contract

- Not behaviour-touching (hard rule 9 does not apply). The design's one looked-up fact: hv_vcpu.h, quoted.

## done criteria

The several-vCPU host has two designs, a choice and a chain of one-concept sprints.

## observed

- The design written; hv_vcpu.h read from the installed SDK (Apple's page renders only with JavaScript):
  hv_vcpus_exit takes a set of vCPUs, names no owning thread, and a kick to a vCPU not running ends its next
  hv_vcpu_run at once.
