# w2m

[![License](https://img.shields.io/badge/license-PolyForm--NC--1.0.0-blue)](LICENSE)
[![Python](https://img.shields.io/badge/python-3.12%2B-blue)](pyproject.toml)

w2m runs unmodified Windows x86-64 programs on an Apple silicon Mac. The goal is `w2m steam run <appid>`: a Windows game from Steam, started on macOS, with no Windows installation and no change to the game's files. The plan is `windows_to_macos_research_direction_v0_2.md`; the roadmap is `docs/plan-2026-10-09.md`.

Each Windows program runs in a Hypervisor.framework virtual machine. A translator turns its x86-64 code into AArch64, block by block. A guest runtime written in C serves the Windows functions the program imports. Each instruction class is judged against Intel CPUs, case by case. Each family of Windows functions is judged against rows that a probe records on real Windows. Generation and verification run on [substrate](https://github.com/laffeyp/substrate).

Status (2026-10-10): the test game, a Unity title, loads `UnityPlayer.dll`, runs its DllMain and enters `UnityMain`. One thread, no graphics yet. `BLACKBOARD.md` and `sprints/` hold the running record.

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
