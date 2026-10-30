# w2m

[![License](https://img.shields.io/badge/license-PolyForm--NC--1.0.0-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.99.1-orange)](rust-toolchain.toml)

w2m will one day (hopefully) run unmodified Windows x86-64 programs on an Apple silicon Mac. It is an experiment: a
research project in one method of doing so, far from finished, with no graphics yet. Nothing here is a product. The
target is `w2m steam run <appid>`: a Windows game from Steam, started on macOS, with no Windows installation and no
change to the game's files. The direction is `windows_to_macos_research_direction_v0_2.md`; the plan is
`docs/rust-port-plan-2026-10-24.md`.

## The method

- **A virtual machine per program.** Each Windows program runs inside a small guest that w2m builds on Apple's
  Hypervisor.framework. The host process, `w2m-vm`, owns the guest's memory and answers its requests.
- **x86 memory order from the hardware.** Apple silicon can run a vCPU with x86's total store order (`ACTLR_EL1`'s
  TSO bit, reachable through the public Hypervisor.framework API); w2m turns it on rather than adding barriers to
  translated code.
- **Translation block by block.** A translator (`crates/translator`) decodes x86-64 with a decoder generated from a
  specification that cites the Intel manual (`crates/x86_decoder`), and lowers each block to AArch64 as the guest
  reaches it.
- **Windows served from inside the guest.** A guest runtime written in C (`tools/guest_runtime`) implements the
  Windows functions the program imports, the loader, threads and exceptions. What it cannot do inside the guest, it
  asks the host: files, memory, threads, processes.
- **Every part judged against Windows itself.** Each x86 instruction class is judged case by case against the
  results Intel CPUs recorded; each family of Windows functions against rows that probe programs printed on real
  Windows; the test game's calls against its runs on Windows. A record of every run and verdict is kept on
  [substrate](https://github.com/laffeyp/substrate).

## Related work

w2m sits among other work on running programs built for one system on another, and borrows from it:

- [Wine](https://www.winehq.org/) implements the Windows API on POSIX systems so Windows programs run there; Valve's
  Proton builds on it for Steam, and w2m's planned Steam bridge follows Proton's `lsteamclient` pattern.
- Apple's Rosetta 2 translates x86-64 Mac programs to Apple silicon.
- [FEX-Emu](https://fex-emu.com/) and [Box64](https://github.com/ptitSeb/box64) translate x86-64 Linux programs on
  ARM64 Linux.

w2m's method differs in kind: a whole Windows process in its own lightweight virtual machine, its Windows layer
inside the guest, and each piece held to recorded Windows behavior before it is built on.

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
