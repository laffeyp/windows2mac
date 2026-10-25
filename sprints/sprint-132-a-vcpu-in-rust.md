# Sprint 132 — A vCPU in Rust

```yaml
---
id: 132
status: done
phase: 6
pass_kind: bridge
---
```

## scope

The walking skeleton's first link (step 3 of `docs/rust-port-plan-2026-10-24.md`). A crate, `hypervisor`, binds the
Hypervisor.framework calls a one-vCPU host needs, each declaration copied from the macOS 26.5 SDK's headers, and wraps
them in two owned types: `Vm` (one per process, destroyed on drop) and `Vcpu` (created on the thread that runs it,
not `Send`, destroyed on drop on that thread). A test maps anonymous memory into a VM, places a few arm64
instructions in it, runs them on one vCPU at EL1 with the MMU off, and reads the exits: an `hvc #8` with x0 = 42 and
the PC after the `hvc` (the bridge mapping's measured fact, WORKING_AGREEMENT), then a resumed run to a second `hvc`.
It also measures the time of an exit and its resume on this Mac against the C monitor's 0.72 µs, and a run stopped
from another thread by `hv_vcpus_exit` returns as canceled. Tests that create a VM need the
`com.apple.security.hypervisor` entitlement: a cargo runner signs each test and binary ad hoc before running it, as
`vm_backend.py` signs `vm_monitor`. On Linux (CI's runner) the crate compiles to nothing and its tests do not run.

## prerequisites

- 131 (closed)

## context_files

- `tools/vm_monitor/vm_monitor.c` (the setup and run loop this replaces), `tools/vm_monitor/entitlements.plist`
- `WORKING_AGREEMENT.md` External SDK bridge mappings (Hypervisor.framework, three entries)
- the SDK's `Hypervisor.framework/Headers/hv_vm.h`, `hv_vm_config.h`, `hv_vm_types.h`, `hv_vcpu.h`,
  `hv_vcpu_types.h`, and `usr/include/arm64/hv/hv_kern_types.h` (read in full 2026-10-25)
- `docs/design-rust-host-2026-10-24.md` (a thread per vCPU; the VM owned by its process)

The declarations bound, copied from those headers:

- `hv_return_t hv_vm_create(hv_vm_config_t _Nullable config);` `hv_return_t hv_vm_destroy(void);`
- `hv_return_t hv_vm_map(void *addr, hv_ipa_t ipa, size_t size, hv_memory_flags_t flags);`
  `hv_return_t hv_vm_unmap(hv_ipa_t ipa, size_t size);`
- `hv_vm_config_t hv_vm_config_create(void);` `hv_return_t hv_vm_config_set_ipa_size(hv_vm_config_t config, uint32_t
  ipa_bit_length);` `hv_return_t hv_vm_config_set_ipa_granule(hv_vm_config_t config, hv_ipa_granule_t granule);`
  (`hv_ipa_granule_t` is a `uint32_t` enum, `HV_IPA_GRANULE_4KB` = 0); the config is released with `os_release`
- `hv_return_t hv_vcpu_create(hv_vcpu_t *vcpu, hv_vcpu_exit_t * _Nullable * _Nonnull exit, hv_vcpu_config_t _Nullable
  config);` `hv_return_t hv_vcpu_destroy(hv_vcpu_t vcpu);` `hv_return_t hv_vcpu_run(hv_vcpu_t vcpu);`
  `hv_return_t hv_vcpus_exit(hv_vcpu_t *vcpus, uint32_t vcpu_count);`
- `hv_return_t hv_vcpu_get_reg(hv_vcpu_t vcpu, hv_reg_t reg, uint64_t *value);` `hv_return_t hv_vcpu_set_reg(hv_vcpu_t
  vcpu, hv_reg_t reg, uint64_t value);` `hv_return_t hv_vcpu_get_sys_reg(hv_vcpu_t vcpu, hv_sys_reg_t reg, uint64_t
  *value);` `hv_return_t hv_vcpu_set_sys_reg(hv_vcpu_t vcpu, hv_sys_reg_t reg, uint64_t value);`
  `hv_return_t hv_vcpu_set_vtimer_mask(hv_vcpu_t vcpu, bool vtimer_is_masked);`
- types: `hv_vcpu_t` `uint64_t`; `hv_reg_t` a `uint32_t` enum, X0 to X30 = 0 to 30, PC 31, FPCR 32, FPSR 33, CPSR 34;
  `hv_sys_reg_t` a `uint16_t` enum (ids as `hv_vcpu_types.h` lists them); `hv_exit_reason_t` a `uint32_t` enum,
  CANCELED 0, EXCEPTION 1, VTIMER_ACTIVATED 2, UNKNOWN 3; `hv_vcpu_exit_t` = { reason; { syndrome, virtual_address,
  physical_address } }; `hv_return_t` a `mach_error_t`, `HV_SUCCESS` 0, errors `0xfae940xx`; memory flags READ 1,
  WRITE 2, EXEC 4

## signal contract

### Emits

None. The crate and its tests put nothing on a record; the skeleton's record comes with the guest (a later link).

### Consumes

The SDK headers above.

### Invariants

- No crate is added beyond the workspace's; the bindings are written by hand (Decision 2026-10-24, serde_json the
  only crate).
- No Python file is added.

## artifact contract

### Files created

- `crates/hypervisor/` (bindings, `Vm`, `Vcpu`, tests); `scripts/sign_and_run.sh` (the cargo runner);
  `.cargo/config.toml`

### Files modified

- `Cargo.toml` (member), `Cargo.lock`, `WORKING_AGREEMENT.md` (the bridge mapping's Rust entry; the canonical home of
  the bindings)

### Content assertions

- Every `extern` declaration in `crates/hypervisor/src/` matches a declaration quoted above, argument for argument.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the first run exits with reason EXCEPTION, exception class 0x16 (HVC), immediate 8, x0 = 42,
  PC = the `hvc`'s address + 4; the resumed run exits at `hvc #9` with x0 = 43. A run stopped by `hv_vcpus_exit`
  from another thread returns CANCELED. 100,000 exits and resumes take a measured time per exit, reported against the
  C monitor's 0.72 µs.
- **Planted fault 1:** a `Vcpu` used from a thread other than its creator does not compile (`!Send`), held by a
  compile-fail check in the test's documentation.
- **Planted fault 2:** an unsigned test binary is refused by the framework (`HV_DENIED`), shown by running the built
  test with no signature once.

## done criteria

Rust creates a VM, runs one vCPU and serves its exits on that vCPU's own thread, with no C monitor between.

## self-review checklist

- [x] Scope names one verifiable property: an arm64 program's exits read from Rust.
- [x] context_files quote every declaration bound.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values and two planted faults.
- [x] One concept: the vCPU from Rust.

## result

- `crates/hypervisor`: 16 Hypervisor.framework calls and `os_release`, each declaration copied from the macOS 26.5 SDK
  (the arm64 return codes and memory flags from `usr/include/arm64/hv/hv_kern_types.h`; `hv_error.h` and
  `hv_types.h` are x86 only); `Vm` (one a process, destroyed on drop), `Vcpu<'vm>` (not `Send`, destroyed on drop),
  `VcpuHandle` (`hv_vcpus_exit` from any thread), `GuestMemory` (anonymous, freed with the process), `Exit`, `Reg`.
  The crate is empty off macOS on arm64, so CI's Linux build compiles it to nothing.
- Expected values, all held: the first run exits EXCEPTION, class 0x16, immediate 8, x0 = 42, PC = the `hvc` + 4;
  the resumed run exits at `hvc #9`, x0 = 43; a vCPU in `b .` stopped by `hv_vcpus_exit` from another thread after
  50 ms returns CANCELED with its PC still in the loop.
- Exit time: 832 to 873 ns an exit and resume from Rust (release, three runs of 100,000); `vm_monitor`'s `spin` on the
  same two instructions, the same day: 813 to 852 ns. The two are the same within run-to-run spread; the C monitor's
  720 ns of 2026-10-14 was that day's.
- Planted faults: a `Vcpu` moved to another thread does not compile (the doc test, E0277); a test binary signed ad
  hoc without the entitlement gets HV_DENIED (0xfae94007) from `hv_vm_create`. A binary with no signature at all is
  killed (SIGKILL, exit 137) before it runs: the first attempt at the plant tested code signing, not the entitlement.
- `scripts/sign_and_run.sh`, the cargo runner (`.cargo/config.toml`, aarch64-apple-darwin only), signs each test and
  binary with `tools/vm_monitor/entitlements.plist`.
- Tests: hypervisor 3 and the doc test; the gates pass.
