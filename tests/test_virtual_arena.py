# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The VirtualAlloc arena's size (Sprint 047): UnityPlayer.dll reserves 0x1FFFF000 bytes at a time, and its
ninth such reservation filled the 4 GB arena of Sprint 046 (scripts/frontier_lookahead.py, 462,293
instructions). Windows gives a 64-bit process 128 TB; w2m's arena is 512 GB of sparse address space."""

from __future__ import annotations

from w2m.virtual_memory import MEM_COMMIT, MEM_RESERVE, PAGE_NOACCESS, PAGE_READWRITE, VirtualMemory
from w2m.vm_backend import VIRTUAL_SIZE, VIRTUAL_VA


def test_unityplayer_s_reservations_fit_many_times_over() -> None:
    touched: list[tuple[int, int]] = []
    v = VirtualMemory(VIRTUAL_VA, VIRTUAL_SIZE, lambda va, n: touched.append((va, n)))
    bases = [v.alloc(0, 0x1FFFF000, MEM_RESERVE, PAGE_NOACCESS) for _ in range(64)]
    assert all(err == 0 for _, err in bases)
    assert [b for b, _ in bases] == [VIRTUAL_VA + i * 0x2000_0000 for i in range(64)]
    last = bases[-1][0]
    assert v.alloc(last, 0x2000, MEM_COMMIT, PAGE_READWRITE) == (last, 0)
    assert touched == [(last, 0x1000), (last + 0x1000, 0x1000)]    # a commit zeroes only its own pages
    assert VIRTUAL_SIZE // 0x2000_0000 == 1024


def test_a_reservation_at_an_address_a_loaded_dll_holds_is_refused() -> None:
    # Sprint 065: steam_api64.dll was placed at 0x1222ff0000 and mono's hinted reservation there was granted, so its
    # JIT code overwrote the image; Windows answers ERROR_INVALID_ADDRESS (487) for an address in use
    from types import SimpleNamespace
    v = VirtualMemory(VIRTUAL_VA, VIRTUAL_SIZE, lambda va, n: None)
    pe = SimpleNamespace(image_base=VIRTUAL_VA, size_of_image=0x5B000, sections=())
    base = v.place_image(pe)  # type: ignore[arg-type]
    assert v.alloc(base, 0x10000, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) == (0, 487)
    assert v.alloc(base + 0x50000, 0x10000, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) == (0, 487)
    assert v.alloc(base + 0x60000, 0x10000, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) == (base + 0x60000, 0)
