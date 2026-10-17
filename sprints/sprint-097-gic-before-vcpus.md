# Sprint 097 — the GICv3 created before any vCPU

```yaml
---
id: 097
status: active
phase: 5
pass_kind: architecture
---
```

## scope

Roadmap step 2's last item (`docs/plan.md`: "the GICv3 made before any vCPU"), which step 3's several vCPUs need to
signal one another. The monitor creates Hypervisor.framework's GICv3 after the VM and before its vCPU, its
distributor and redistributor regions placed above the guest's memory in the 40-bit IPA space at the alignments the
host reports, and sets the vCPU's affinity in MPIDR_EL1. Nothing uses an interrupt yet; every run must behave as
before.

**Known first (Apple's header and page, read 2026-10-16).**

- hv_gic.h: "The hv_gic_create() API must only be called after a virtual machine has been created. It must also be
  done before vCPU's have been created so that GIC CPU system resources can be allocated." "GIC v3 uses affinity
  based interrupt routing. vCPU's must set affinity values in their MPIDR_EL1 register. Once the virtual machine
  vcpus are running, its topology is considered final." (macOS 15.0 and later; developer.apple.com, hv_gic_create.)
- hv_gic_config.h: each base is aligned to the value `hv_gic_get_distributor_base_alignment` or
  `hv_gic_get_redistributor_base_alignment` returns; "The redistributor region will contain redistributors for all
  vCPUs supported by the virtual machine." hv_gic_parameters.h: "Each redistributor is two 64 kilobyte frames per vCPU
  and is contiguously placed."
- So step 3's pool of vCPUs is created whole at the start, after the GIC: the topology cannot grow once they run.

## prerequisites

- 096.

## context_files

- tools/vm_monitor/vm_monitor.c; the SDK's hv_gic.h, hv_gic_config.h, hv_gic_parameters.h; src/w2m/vm_backend.py
  (IPA_BASE, the monitor's size).

## signal contract

### Emits

- None new.

### Invariants

- Vocabulary unchanged; no interrupt is raised or taken.

## artifact contract

- vm_monitor.c: the GIC created before the vCPU, MPIDR_EL1 set, a `gic` command reporting the distributor base,
  the vCPU's redistributor base, the region size and MPIDR_EL1; a test reading it; `scripts/gates.sh` exit 0; CI
  `success` on the closing commit.

## observation contract

- The test: the vCPU's redistributor base equals the configured base, both bases are aligned and above the guest's
  memory, MPIDR_EL1 holds the affinity set.
- Plant: the GIC created after the vCPU; the monitor must fail at start with the framework's error (Apple's rule).
- Every family accepts and the batch run conforms (168 of 168 lines, 0 divergences) with the GIC present.

## done criteria

The GICv3 exists in every w2m VM, made in the order Apple requires, every judge unchanged, CI green.

## observed
