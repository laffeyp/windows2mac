# Review — Windows-to-macOS Binary Porting System, Research Direction v0.1

Reviewed 2026-10-03 against `windows_to_macos_research_direction_v0_1.md` (project root, 27,382 bytes). Ground rules: `sdd-kit-3/AGENTS.md` resolve-and-record; every external claim below was checked against the named source on 2026-10-03 or is marked unverified.

---

## 1. Verdict

The correctness model (sections 3, 11, 12) is the strongest part of the document and should be kept nearly verbatim: correctness as trace refinement over declared relations, the reference execution as a witness rather than the definition, nondeterminism as allowed-outcome sets, and failures returned as minimized structured discrepancies. That last item is exactly what the SDD kit calls a Signal Report, and it is what lets substrate's writer/checker topologies close a loop without a human narrating failures.

The weak part is the premise in section 1: that a complete behavioral specification of Windows exists to be compiled. It does not. Microsoft's documentation is incomplete and games depend on undocumented behavior: TEB/PEB layout, exact error codes, heap allocation patterns, stack contents. Wine has spent thirty-three years (since 1993) discovering that contract empirically. The contract therefore has to be *mined from a Windows oracle by probes*, with documentation as a hint. The generation system then implements against the mined contract. That turns the project's first deliverable into the probe-and-contract pipeline, not the compiler.

Five gaps below change the architecture or the scope. The rest are sharpenings.

## 2. Gaps that change the plan

### 2.1 Static recovery of all shipped code is undecidable; dynamic translation is not optional

Section 6 says the design "does not require heuristic fallback behavior" and treats runtime translation as needed only for "newly supplied Windows machine code." Separating code from data in a stripped x86 binary is undecidable in general: indirect jumps, jump tables, and data interleaved in `.text` defeat any static disassembler. Code that ships in the PE but is never discovered statically is not "newly supplied," and the design as written has no path for it. Add to that the common runtime code generators in games: Mono's JIT in Unity titles that are not IL2CPP, LuaJIT, CEF/V8 in launchers and overlays.

Consequence: a dynamic x86-64 to ARM64 translator is a day-one correctness component. AOT becomes a cache that the dynamic translator's verified output populates. This does not contradict the staging rule in section 6 — operations still move to the earliest valid stage — but the claim of "complete translation without fallback" should be withdrawn.

### 2.2 The x86 memory model points to the VM backend, not to native processes

Section 9 discusses acquire/release lowerings and omits the decisive hardware fact. Apple silicon implements x86-style total store ordering (TSO) in hardware, switched by the IMPDEF bit `ACTLR_EL1.TSOEN`. Rosetta uses it. For third parties the documented path is a VM: the 2024 Linux patch series exposing TSO to Linux guests on Apple silicon states it "must be done on Apple Silicon MacOS Sonoma version 14.4 or later, since earlier versions do not support modification of the TSOEN bit," and FEX-Emu uses it. No public API exposes TSO to a native macOS process (unverified beyond the absence of documentation; treat as an open question to test, not a fact).

Combine that with section 8's own finding: `hv_vm_config_set_ipa_granule` with `HV_IPA_GRANULE_4KB` is public from macOS 26, and UTM upstreamed QEMU support for it in December 2025. The VM backend gets both 4 KB guest pages and hardware TSO. A native process gets neither without software enforcement on every load and store. Section 8 says neither backend should be chosen "merely from the page-size ratio." Correct, but page size plus memory ordering together is not "merely." The first CPU backend should be a Hypervisor.framework VM running a compact guest runtime, with native-process lowering as a later optimization measured against it.

This host qualifies: macOS 26.5.1, Apple M5 Max, 16 KB host pages.

### 2.3 DRM, packers, and anti-cheat need an explicit scope line

The document never mentions them. SteamStub-wrapped executables decrypt `.text` at runtime, so AOT sees ciphertext. Denuvo runs virtualized, self-hashing code; any translation that changes code bytes or timing can trip it, and section 4's "retain original PE bytes" handles only naive self-inspection. Kernel-mode anti-cheat (EAC kernel mode, BattlEye, Vanguard) cannot be satisfied by any user-space translator. The scope statement needs a sentence such as: *v0 targets DRM-free or SteamStub-only titles with no kernel anti-cheat; Denuvo and kernel anti-cheat are out of scope.*

### 2.4 The Steam bridge has a known shape; name it

Section 4 says the system owns "the bridge to legitimate Steam services." The precedent is Valve's own Proton `lsteamclient`: per-interface-version shims (`cppISteamApps_STEAMAPPS_INTERFACE_VERSION001.cpp` through `...004.cpp` and so on) that marshal the Windows `steam_api64.dll` calls onto the native `steamclient` library. A macOS bridge has the same shape against the Mac Steam client. The interface-version count is large and versioned per SDK release, which makes it a good fit for contract-driven generation and a bad fit for hand-writing. Acquisition via DepotDownloader is confirmed: Homebrew install on macOS, `-os windows` selects Windows depots, `-qr` / `-remember-password` cover Steam Guard.

### 2.5 The shader frontend already exists from Apple

Section 10 plans a DXBC/DXIL/HLSL frontend. Apple ships Metal Shader Converter (`metal-shaderconverter`, `libmetalirconverter`): DXIL in, Metal IR / metallib out, covering vertex through mesh, tessellation, geometry, compute, and ray-tracing stages. It is a closed binary, but it is an Apple developer tool, not Wine, Rosetta, or D3DMetal, so it does not violate section 1's exclusions. It covers DXIL (D3D12-era, SM 6.x) only; DXBC (D3D10/11, SM 4/5) still needs a frontend. The DXMT citation also needs correcting: DXMT describes itself as "a Metal-based translation layer for Direct3D 11 and 10 which allows running 3D applications on macOS using Wine." It is a precedent for the D3D-to-Metal command boundary, and it lives inside Wine.

## 3. Sharpenings

- **The oracle is hardware.** Section 11 checks contracts "through the reference model, the Windows probe, ..." The probe needs a real x86-64 Windows machine. Windows on ARM in a Mac VM runs x86 code under Microsoft's own emulator and is not a valid oracle for memory ordering, timing, or exact fault behavior. This is a money/hardware decision (user-only).
- **The API surface is the schedule.** kernel32, ntdll, user32, gdi32, dxgi, d3d11/12, xaudio2, xinput, dinput8, winmm, ws2_32, bcrypt, ole32/COM, shell32. One real game exercises a few hundred entry points. Scope the first contract to what one chosen game calls, measured by an import scan plus a dynamic trace on the oracle.
- **Baselines exist and should be measured, not adopted.** CrossOver and Apple's Game Porting Toolkit (Wine plus D3DMetal) are the compatibility and frame-time baselines to beat. Whisky is archived (last push 2025-05-11). FEX-Emu (pushed 2026-10-03) and Box64 are the live open-source x86-64 dynamic translators to study; Remill (pushed 2026-08-27) is maintained.
- **Section 3's "deterministic observations match exactly" needs a clock clause.** `QueryPerformanceCounter`, `GetTickCount`, and `rdtsc` are observations. They belong in the allowed-outcome or numerical-envelope class with a monotonicity obligation, or every trace comparison fails on timing.
- **Sections 13-14 are sound.** No change.

## 4. What maps onto substrate

Substrate (PyPI `substrate-kernel`, pyproject version 1.1.1) is the generation-and-verification harness section 1 assumes. Concretely:

| Research-direction need | Substrate mechanism |
|---|---|
| Generate an implementation of a contract, check it, feed the failure back | `best_of_n_verified` with a deterministic `check(response) -> (passed, reason)` verifier; `coding_flow` writer + checker |
| Failures as structured discrepancies, not "the game failed" | Typed events on the run record; `assert_event`, `first_divergence` (D-8) |
| Reproducible reference vs target runs (section 14) | Run records with replay Levels 1, 2, 3(a); `baseline()` manifest metadata |
| Find unknown failures in real games | Addendum D discovery harness: a real driver run wild, crashes harvested from the record |

Substrate does not compile anything, run x86 code, or talk to Metal. It orchestrates the producers that do. The compiler, runtime, and probes are new code.

Two facts to settle before depending on it: the README's Status section says "Apache-2.0" while `pyproject.toml` and the Licensing section say PolyForm Noncommercial 1.0.0, and the license states that an AI-mediated reimplementation is a derivative work under the noncommercial terms. Whether w2m is noncommercial is the user's call.

## 5. Proposed scope for v0

One vertical slice before any generality: one DRM-free, D3D11, x86-64 Windows game the user owns, run in a Hypervisor.framework VM backend with 4 KB IPA granule and TSO, with only the Win32/D3D11 entry points that game calls implemented against a contract mined from a Windows oracle. Acquisition, Steam bridge, AOT caching, native-process lowering, D3D12, and DXIL all follow the slice.

---

Sources: [Apple Hypervisor updates](https://developer.apple.com/documentation/updates/hypervisor.md); [QEMU hvf IPA-granule patch, Dec 2025](https://patchew.org/QEMU/20251219183716.4379-1-j@getutm.app); [Linux TSO-for-VMs patch series, Apr 2024](https://lkml.iu.edu/hypermail/linux/kernel/2404.1/04656.html); [LWN on arm64 TSO](https://lwn.net/Articles/969480); [Metal Shader Converter](https://developer.apple.com/metal/shader-converter); [DXMT](https://github.com/3Shain/dxmt); [DepotDownloader](https://github.com/SteamRE/DepotDownloader); [Proton lsteamclient](https://github.com/ValveSoftware/Proton/tree/HEAD/lsteamclient); GitHub API repo metadata for Remill, FEX-Emu, Whisky, DXMT (2026-10-03).
