# Sprint 300 — One test binary a crate

```yaml
---
id: 300
status: done
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

- **A departure from the card's form:** the card moved each file into `tests/all/` by `git mv`; the files stay where
  they were instead, each a module of `tests/all.rs`, the manifests setting `autotests = false` and naming one
  `[[test]]`. In a crate root `mod batch_run;` reads `tests/batch_run.rs`, so no path changes, and the comments,
  scripts and records that cite `tests/<name>.rs` (`scripts/gates.sh` 130-170 among them) stay right. The file
  `virtual_memory.rs` is the module `virtual_memory_arena` by `#[path]`, its own name being the `virtual_memory`
  crate's, which its `use virtual_memory::…` means. `common` is declared once in the root, its two users reading
  `use crate::common;`. The gates' timing step (`scripts/gates.sh` 210) runs `--test all -E "test(/^exit_time::/)"`
  for the removed `--test exit_time`.
- **The change** (84fd8c7d3): the two roots, each with `every_file_is_a_module`; the two manifests; the two
  `common` lines; the gates' line.
- **Expected values:** `nextest list` with every ignored test: 79 in `w2m_vm::all` and 8 in `hypervisor::all`, equal to
  the `#[test]` functions in the two crates' test files, the roots' one each included; no warning in the build; the
  binaries 31 test files' worth down to 2; the gates' wall time against Sprint 298's 1,106 s, all Rust tests both,
  947 s whole and nextest 536.2 s at load 3.35, against Sprint 298's 1,106 s and 471.6 s at load 3.77, this run adding the substrate samples: the whole 159 s less, the test step 65 s more, one run each, the cause of either not measured.
- **The gates:** `scripts/gates.sh` exit 0 in 947 s, kept in `store/gates/passes.tsv` as sprint 300 at 84fd8c7d3 with root=ee3e1bec0 and "rust=all python=0 samples=1", `--all` (the gates' script changed), 330 tests, base=c9f542504, Sprint 299's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: `mod ledger;` commented out in `w2m_vm`'s root;
  `every_file_is_a_module` failed, "files under tests/ no module declares: [\"ledger.rs\"]".

