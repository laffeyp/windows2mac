// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The translator (Sprint 151): x86-64 instructions lifted to explicit operations (`lift`, from `src/w2m/lift.py`),
//! by one rule per instruction class (`rules`, from `src/w2m/rules`), and lowered to AArch64 words (`lower`, from
//! `vm_backend.compile_insn`). Link 5 of `docs/design-host-compile-path-2026-10-24.md` ports the rules and their
//! lowering in groups; group 1 is the integer and control-flow core.

pub mod lift;
pub mod liveness;
pub mod lower;
pub mod rules;
