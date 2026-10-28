// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! No VEX.256 store reaches the model (Sprint 192; reviewer-2's Low 4 of its review of Sprints 185-189). The model's
//! 256-bit store branch (`w2m_vm::model`, runtime.py 750-753) is judged by a unit test only, because no lowered
//! instruction reaches it: every VEX move with a 256-bit store form lowers natively. This test lifts each VEX.256 store
//! encoding of the SDM's moves (`vmovaps`, `vmovapd`, `vmovups`, `vmovupd`, `vmovdqa`, `vmovdqu`, `vmovntps`, `vmovntpd`,
//! `vmovntdq`, each `[rax], ymm0`) and holds that a lifted one is not routed to `hvc #10`, so a class that newly reaches
//! the branch fails here and the case runner becomes its judge.

use translator::lift::{Op, lift};
use translator::lower::vex_to_model;

const STORES: [(&str, &[u8]); 9] = [
    ("vmovaps", &[0xC5, 0xFC, 0x29, 0x00]),
    ("vmovapd", &[0xC5, 0xFD, 0x29, 0x00]),
    ("vmovups", &[0xC5, 0xFC, 0x11, 0x00]),
    ("vmovupd", &[0xC5, 0xFD, 0x11, 0x00]),
    ("vmovdqa", &[0xC5, 0xFD, 0x7F, 0x00]),
    ("vmovdqu", &[0xC5, 0xFE, 0x7F, 0x00]),
    ("vmovntps", &[0xC5, 0xFC, 0x2B, 0x00]),
    ("vmovntpd", &[0xC5, 0xFD, 0x2B, 0x00]),
    ("vmovntdq", &[0xC5, 0xFD, 0xE7, 0x00]),
];

#[test]
fn no_vex_256_store_is_routed_to_the_model() {
    let mut lifted = Vec::new();
    for (name, code) in STORES {
        let mut bytes = code.to_vec();
        bytes.extend([0u8; 16]);
        let Ok(ins) = lift(&bytes, 0x1000) else {
            continue; // not lifted: the translator refuses it by name, not to the model
        };
        for op in &ins.ops {
            if let Op::Vex { base, d, vl, .. } = op
                && d.addr().is_some()
                && *vl != 0
            {
                assert!(
                    !vex_to_model(base, true),
                    "{name} (base {base}) at 256 bits goes to the model's 256-bit store branch"
                );
                lifted.push(name);
            }
        }
    }
    println!("VEX.256 stores lifted and lowered natively: {lifted:?}");
    assert!(!lifted.is_empty(), "no VEX.256 store lifted: the test holds nothing");
}
