// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 147: the x86 stubs' page equals the frozen Python's (`guest_stubs.page(0x7ffe00001230)`, run once on
//! 2026-10-24; its bytes below).

use w2m_vm::guest_stubs::{CALL, CALL_RETURN, RESTORE, SIZE, page};

const PYTHON: &str = concat!(
    "c3ccccccccccccccccccccccccccccccffd04889c148b830120000fe7f0000ffe0cccccccccccccccccccccccccccccccc",
    "cccccccccccccccccccccccccccccc0f1081a00100000f1089b00100000f1091c00100000f1099d00100000f10a1e001",
    "00000f10a9f00100000f10b1000200000f10b910020000440f108120020000440f108930020000440f109140020000440f",
    "109950020000440f10a160020000440f10a970020000440f10b180020000440f10b9900200000fae5134488ba1980000",
    "004883ec10488b81f800000048894424088b414448890424488b8178000000488b9188000000488b9990000000488ba9a0",
    "000000488bb1a8000000488bb9b00000004c8b81b80000004c8b89c00000004c8b91c80000004c8b99d00000004c8ba1",
    "d80000004c8ba9e00000004c8bb1e80000004c8bb9f0000000488b89800000009dc3",
);

#[test]
fn the_stub_page_is_the_python_s() {
    let got = page(0x7ffe_0000_1230);
    assert_eq!(got.len(), SIZE);
    let want: Vec<u8> = (0..PYTHON.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&PYTHON[i..i + 2], 16).unwrap())
        .collect();
    // the Python's page past its last stub is 0xCC filler to SIZE
    assert_eq!(&got[..want.len()], &want[..]);
    assert!(got[want.len()..].iter().all(|b| *b == 0xCC));
    assert_eq!((CALL, CALL_RETURN, RESTORE), (0x10, 0x12, 0x40));
}
