// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! No VEX.256 store reaches the model (Sprint 192; reviewer-2's Low 4 of its review of Sprints 185-189). The model's
//! 256-bit store branch (`w2m_vm::model`, runtime.py 750-753) is judged by a unit test only, because no lowered
//! instruction reaches it: every VEX move with a 256-bit store form lowers natively.
//!
//! From Sprint 195 (reviewer-2's Low 2 of its review of 190-192: Sprint 192 picked nine encodings by hand) the test
//! lifts every three-byte VEX encoding at L=1 with ModRM 0x00 (`[rax]`, register 0): each map (0F, 0F38, 0F3A), each
//! prefix (none, 66, F3, F2), each opcode, W 0 and 1, VEX.vvvv unused (1111) and register 15 (0000). Each one the
//! lifter takes whose VEX op writes memory is a 256-bit store, and it must not be routed to `hvc #10`; a class that
//! newly reaches the branch fails here and the case runner becomes its judge.

use translator::lift::{Op, lift};
use translator::lower::vex_to_model;

#[test]
fn no_vex_256_store_is_routed_to_the_model() {
    let mut stores = std::collections::BTreeSet::new();
    let mut tried = 0;
    for map in 1u8..=3 {
        for pp in 0u8..4 {
            for op in 0u8..=255 {
                for w in 0u8..2 {
                    for vvvv in [0b1111u8, 0b0000] {
                        // C4, then R X B inverted (1 1 1) and the map, then W, vvvv inverted, L=1 and pp
                        let mut bytes = vec![0xC4, 0xE0 | map, (w << 7) | ((!vvvv & 0xF) << 3) | 0b100 | pp, op, 0x00];
                        bytes.extend([0u8; 16]);
                        tried += 1;
                        let Ok(ins) = lift(&bytes, 0x1000) else {
                            continue; // not lifted: the translator refuses it by name, not to the model
                        };
                        for o in &ins.ops {
                            if let Op::Vex { base, d, vl, .. } = o
                                && d.addr().is_some()
                                && *vl != 0
                            {
                                assert!(
                                    !vex_to_model(base, true),
                                    "map {map} pp {pp} opcode {op:#04x} W{w} vvvv {vvvv:04b} (base {base}) at 256 bits \
                                     goes to the model's 256-bit store branch"
                                );
                                stores.insert(base.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    println!("{tried} VEX.256 encodings tried; the stores lifted, all lowered natively: {stores:?}");
    assert!(!stores.is_empty(), "no VEX.256 store lifted: the test holds nothing");
}
