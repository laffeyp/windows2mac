# Sprint 300 — One test binary a crate

```yaml
---
id: 300
status: active
phase: 6
pass_kind: functional
---
```

## scope

Each file under a crate's `tests/` is its own binary: `w2m_vm` has 26, `hypervisor` 5, and the gates' nextest run
counted 40 binaries over the crates it selected (Sprint 298). On this Mac, macOS's `syspolicyd` assesses each newly
linked executable on its first run: at Sprint 298's gates it ran at 210% CPU while three listings waited 1 min 28 s
each, and a run of one 1.1 s test after a manifest change passed 600 s (298's Observation 4). nextest runs every
binary twice to list it before any test. None of the 31 files has `fn main`, an `[[test]]` entry, `harness = false`
or `env::set_var`; nextest runs each test in its own process, so a test that needs one VM a process keeps it.

- **The work:** `crates/w2m_vm/tests/` and `crates/hypervisor/tests/` each one binary, `tests/all/main.rs` declaring a
  module a former file, moved by `git mv` so their history follows; `common/mod.rs` a module of it; each file's
  `use` of `common` and any `#![cfg(..)]` kept as the module's own; `every_file_is_a_module` in each `main.rs`.
- **The measure:** the gates' wall time and nextest's summary before (Sprint 298's gates) and after, on a change that
  relinks the same crates.
- **Not here:** the other crates' 1 to 3 files each; the Developer Tools exemption, a setting of the Mac's.

## prerequisites

- 299 (closed)

## context_files

- `crates/w2m_vm/tests/`, `crates/hypervisor/tests/`; `scripts/sign_and_run.sh`; `.cargo/config.toml`
- Sprint 298's gates log and Observation 4

## signal contract

### Emits

- None; the tests' layout.

### Invariants

- The same tests, by name, pass; nextest's count of tests run equal before and after.

## artifact contract

### Files modified

- `crates/w2m_vm/tests/**` and `crates/hypervisor/tests/**`, moved; the tests' names under nextest gaining the
  module's prefix

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the test count before and after equal; the binaries listed down by 29; the gates' wall time
  before and after, with the load.
- **Planted faults:** (1) one module's line struck from `w2m_vm`'s `tests/all/main.rs`: a test in it,
  `every_file_is_a_module`, reading the directory and `main.rs`, must name the file no `mod` declares, since an
  undeclared file compiles to nothing and its tests would vanish without a failure.

## done criteria

Each of the two crates links one test binary; every test runs as before; the time saved is measured.

## self-review checklist

- [x] Scope names one verifiable property: one test binary a crate, the tests unchanged.
- [x] context_files cite the tests, the runner and the measure's source.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and a plant changing what its judge reads.
- [x] One concept: the tests' layout; 31 files moved, mechanical, at the floor of a layout change.

## result

