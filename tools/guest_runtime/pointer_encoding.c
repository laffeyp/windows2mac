// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* pointer_encoding.c — EncodePointer and DecodePointer (Sprint 046, family "pointer encoding").
 *
 * contracts/windows.pointer.encoding records Windows on sixteen runners (32 identical outputs) as
 * relations, because the encoding uses a per-process secret: every value round-trips, encodes the same
 * twice and encodes to something else (NULL included); no single XOR mask explains three encodings; the
 * last error is left. w2m keeps one fixed secret and rotates after the XOR, which meets every row; the
 * rows do not fix Windows's own algorithm, and a program that compares encodings across processes
 * would see w2m's secret never change.
 */
#include "runtime.h"

#define SECRET 0x6A09E667F3BCC909ull          /* fractional bits of sqrt(2), any value with nonzero low six bits */

static u64 ror(u64 v, u32 n) { return n ? v >> n | v << (64 - n) : v; }
static u64 rol(u64 v, u32 n) { return n ? v << n | v >> (64 - n) : v; }

u64 k32_EncodePointer(u64 p) {
    return w2m_logged(W2M_API_EncodePointer, p, ror(p ^ SECRET, SECRET & 63));
}

u64 k32_DecodePointer(u64 p) {
    return w2m_logged(W2M_API_DecodePointer, p, rol(p, SECRET & 63) ^ SECRET);
}
