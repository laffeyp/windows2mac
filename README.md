# w2m

[![License](https://img.shields.io/badge/license-PolyForm--NC--1.0.0-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.99.1-orange)](rust-toolchain.toml)

w2m will one day (hopefully) run unmodified Windows x86-64 programs on an Apple silicon Mac. It is a research project,
experimental and far from finished, and nothing here is a product. The target is `w2m steam run <appid>`: a Windows
game obtained from Steam, prepared and launched from the Mac command line, with no Windows installation and no change
to the game's files. The direction is `windows_to_macos_research_direction_v0_2.md`; the plan is
`docs/rust-port-plan-2026-10-24.md`.

## The idea

w2m is a specification-driven binary porting compiler. Its premise is that complete behavioral specifications can be
turned into correct implementations by the project's own generation and automated verification.

- **The contract comes first.** What a Windows program can observe of its machine and its system is written down as
  versioned, executable contracts. They are built from evidence: Microsoft's documentation, machine models, probe
  programs run on real Windows, and the application's own runs. The evidence stays a separate, inspectable artifact.
- **Correctness is a relation over observable execution.** Returns, errors, memory effects, faults, synchronization,
  callbacks, file effects and clock readings must be ones the Windows contract permits. Each comparison declares its
  relation: exact bits, a state relation between differently represented objects, an allowed set of outcomes, a
  partial order of effects, a numerical envelope, or a progress obligation. One captured Windows schedule is a
  witness, not the only acceptable result.
- **Generation answers to verification.** Implementations are generated and accepted only when they conform; a
  disagreement becomes a structured discrepancy, routed either to repairing the implementation or to revising the
  contract on new evidence.
- **Each application is compiled for itself.** The organizing principle is partial evaluation: fix the application,
  the destination Mac and the stable facts of the installation, and compile away the work that no longer depends on
  live inputs.

## How it is built today

- Each Windows program runs in a small guest on Apple's Hypervisor.framework. The host process, `w2m-vm`, written in
  Rust, owns the guest's memory and answers its requests.
- The vCPU runs with the hardware's x86 total store order (`ACTLR_EL1`'s TSO bit), so translated code needs no added
  ordering barriers.
- A translator lifts x86-64 to AArch64 block by block, decoding with a decoder generated from a specification that
  cites the Intel manual.
- A guest runtime in C implements the Windows functions the program imports, its loader, threads and exceptions, and
  asks the host for files, memory, threads and processes.
- Every part is judged against Windows: each x86 instruction class case by case against results recorded on Intel
  CPUs, each family of Windows functions against rows that probe programs printed on Windows, and the test game's
  calls against its own runs on Windows. Every run and verdict is kept as a record on
  [substrate](https://github.com/laffeyp/substrate), the project's generation-and-verification harness.

## What it draws on

The research draws on binary lifting, semantics-preserving compilation, partial evaluation, formal memory models,
state-machine testing and native GPU compilation. [Remill](https://github.com/lifting-bits/remill) informs instruction
lifting, [rev.ng](https://rev.ng/) code discovery, [CompCert](https://compcert.org/) semantic preservation, and
[herdtools7](https://github.com/herd/herdtools7) memory-model testing. Proton's `lsteamclient` is the reference for the
versioned Steam interface adapters, [DXMT](https://github.com/3Shain/dxmt) for a Direct3D-to-Metal boundary, and
Apple's Metal Shader Converter for shader conversion. Wine, Rosetta and D3DMetal belong to the same field of work and
are not dependencies of this design. These are references for ideas and comparison, not adopted implementations.

## Status (2026-10-29)

- The test game, a Unity title, runs on the Rust host until the point its reference run on Windows stopped: its
  10,505 Windows calls equal the reference's, row for row.
- 437 of 458 x86 instruction classes conform to Intel's results on the Rust host, and 143 of 273 runs of the Windows
  function families conform to Windows.
- No window, graphics or sound yet. `BLACKBOARD.md` and `sprints/` hold the running record.

---

## Licensing and use

This project is licensed under the **PolyForm Noncommercial License 1.0.0**.
Source-available; noncommercial use only.

Permitted uses include personal, research, educational, hobbyist,
government, and nonprofit charitable use. Commercial use of any kind
is not permitted under this license. See `LICENSE` for the full text
and the additional statements on AI-mediated derivation that ride
alongside it.

### Position on machine learning

The licensor does not consent to the use of this software, its
documentation, or its records as training data for AI or machine
learning systems. Training crawlers should honor the `robots.txt` and
`ai.txt` at the repository root or any hosted docs site.

A reimplementation produced by exposing this codebase to an AI or ML
system is a derivative work and is subject to the PolyForm Noncommercial
License in full — that is, no commercial use of the derivative work.

See `LICENSE` Sections A and B for the full statement.
