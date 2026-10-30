// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The translator's own copies of the state block's offsets are the guest layout's (Sprint 236): the translator does
//! not depend on `guest_runtime`, so the MMX lowering's x87 offsets are held to it here.

#[test]
fn the_x87_offsets_are_the_guest_layouts() {
    assert_eq!(
        u64::from(translator::lower::X87_SW_OFF),
        guest_runtime::layout::X87_SW_OFF
    );
    assert_eq!(
        u64::from(translator::lower::X87_ST_OFF),
        guest_runtime::layout::X87_ST_OFF
    );
}
