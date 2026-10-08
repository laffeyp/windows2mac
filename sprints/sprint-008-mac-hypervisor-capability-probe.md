# Sprint 008 — Mac Hypervisor capability probe

```yaml
---
id: 008
status: closed
phase: 2
pass_kind: bridge
---
```

## scope

Author `probes/mac_hv_capability/hv_capability.c`, a Hypervisor.framework program run on this Mac in two modes. Mode `4k` creates a VM with `HV_IPA_GRANULE_4KB`, maps one 4 KB page at IPA 0x10001000 (4 KB-aligned, not 16 KB-aligned), runs a vCPU whose guest code is `movz x0,#42; hvc #0` from that page, and checks X0=42 at the HVC exit. It sets `HV_SYS_REG_ACTLR_EL1` bit 1 (EnTSO), reads it back before and after the guest run. Mode `16k` repeats the mapping with the default 16 KB granule and expects refusal. Author `scripts/probe_capabilities.py`, which builds and ad-hoc signs the probe with the `com.apple.security.hypervisor` entitlement, runs both modes, and records CAPABILITY_PROBED events on a substrate record (ipa_granule_4k, guest_tsoen, guest_tsoen_context_switch, host_page_16k, native_process_tso).

## prerequisites

- 002

## context_files

- macOS 26.5 SDK `Hypervisor.framework/Headers/hv_vm_config.h` lines 110-143, `hv_vcpu_types.h` lines 340-346, `hv_vm.h`, `hv_vcpu.h` (read 2026-10-07)
- `signals/0.1.json` (CAPABILITY_PROBED, enum `capability`)
- v0.2 §8.1, §8.2

## bridge mapping

- `hv_vm_config_create()`, `hv_vm_config_get_default_ipa_granule(&g)`, `hv_vm_config_set_ipa_granule(cfg, HV_IPA_GRANULE_4KB)` (macOS 26.0), `hv_vm_create(cfg)`.
- `hv_vm_map(host_addr, ipa, size, HV_MEMORY_READ|HV_MEMORY_EXEC)`.
- `hv_vcpu_create(&vcpu, &exit, NULL)` on the thread that runs it; `hv_vcpu_set_reg(vcpu, HV_REG_PC, ipa)`, `HV_REG_CPSR = 0x3c5` (EL1h, DAIF masked); `hv_vcpu_run`; exit `HV_EXIT_REASON_EXCEPTION`, ESR EC 0x16 = HVC64.
- `hv_vcpu_set_sys_reg(vcpu, HV_SYS_REG_ACTLR_EL1, 1<<1)` / `hv_vcpu_get_sys_reg` (macOS 15.0; only bit 1 is accessible).
- Entitlement `com.apple.security.hypervisor`; ad-hoc `codesign -s - --entitlements`.

## signal contract

### Emits

- `SESSION_INIT`, `CAPABILITY_PROBED` x5, `SESSION_COMPLETED`.

## artifact contract

### Files created

- `probes/mac_hv_capability/hv_capability.c`
- `probes/mac_hv_capability/entitlements.plist`
- `scripts/probe_capabilities.py`

### Command exit codes

- `uv run python scripts/probe_capabilities.py` returns 0
- `uv run pytest`, `uv run ruff check`, `uv run mypy` return 0

## observation contract

### Expected runtime signals

- CAPABILITY_PROBED(ipa_granule_4k, present) with the guest's X0=42 in `detail`.
- CAPABILITY_PROBED(guest_tsoen, present): ACTLR_EL1 reads back with bit 1 set.
- CAPABILITY_PROBED(guest_tsoen_context_switch, present): bit 1 still set after an `hv_vcpu_run` exit.
- CAPABILITY_PROBED(host_page_16k, present): `hw.pagesize` 16384.
- CAPABILITY_PROBED(native_process_tso, absent) with `detail` limited to "no public interface in macOS 26.5 SDK headers outside Hypervisor.framework".
- The `16k` mode refuses the 4 KB-aligned mapping (recorded in the ipa_granule_4k detail as the control).

## done criteria

OQ-1 is answered with observed evidence from this host: a public API sets guest TSO, and a guest runs from a 4 KB-granule page.

## notes

The TSO *effect* (forbidden weak outcomes disappear) is not tested here; that needs a two-vCPU litmus run (next sprint). This sprint establishes that the controls exist and stick.

## self-review checklist

- [x] Scope concrete.
- [x] Bridge mapping from installed SDK headers.
- [x] Emits are locked tags; capability enum values exist in 0.1.
- [x] Assertions gradable.
- [x] Observation contract present.
- [x] One concept; three files (probe, entitlement, driver).
