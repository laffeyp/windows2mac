// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `tests/test_x86dec.py`'s cases, carried by hand and cited by line. Its Capstone walk (20-40) and its XED differential
//! (110-125) run Python tools; the C decoder's differential (`c_differential.rs`) stands in their place.

use x86_decoder::{DecodeError, decode};

fn padded(hex: &str) -> Vec<u8> {
    let mut b: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    b.extend([0; 15]);
    b
}

fn bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

/// test_x86dec.py 43-64.
#[test]
fn known_encodings() {
    for (hex, length, mnemonic, iclass) in [
        ("90", 1, "nop", "NOP"),
        ("4190", 2, "xchg", "XCHG"),
        ("f390", 2, "pause", "PAUSE"),
        ("48b88877665544332211", 10, "movabs", "MOV"),
        ("b844332211", 5, "mov", "MOV"),
        ("f2410f114bd8", 6, "movsd", "MOVSD_XMM"),
        ("a5", 1, "movsd", "MOVSD"),
        ("f348ab", 3, "rep stosq", "REP_STOSQ"),
        ("f0ff05aabbccdd", 7, "lock inc", "INC_LOCK"),
        ("c5f877", 3, "vzeroupper", "VZEROUPPER"),
        ("c5fc77", 3, "vzeroall", "VZEROALL"),
        ("4898", 2, "cdqe", "CDQE"),
        ("6698", 2, "cbw", "CBW"),
        ("0f01d0", 3, "xgetbv", "XGETBV"),
        ("e800000000", 5, "call", "CALL_NEAR"),
        ("7400", 2, "je", "JZ"),
        ("40498b4078", 5, "mov", "MOV"),
        ("a3790800837b0800ed", 9, "movabs", "MOV"),
        ("f3a6", 2, "repe cmpsb", "REPE_CMPSB"),
        ("62e17e286f2557d10800", 10, "vmovdqu32", "VMOVDQU32"),
        ("62f1fd487f01", 6, "vmovdqa64", "VMOVDQA64"),
        ("62e3754825c496", 7, "vpternlogd", "VPTERNLOGD"),
        ("8fe878c2e410", 6, "vprotd", "VPROTD"),
        ("8f4878c3c838", 6, "vprotq", "VPROTQ"),
        ("8f00", 2, "pop", "POP"),
    ] {
        let d = decode(&padded(hex), None).unwrap_or_else(|e| panic!("{hex}: {e}"));
        assert_eq!(
            (d.length, d.mnemonic.as_str(), d.iclass.as_str()),
            (length, mnemonic, iclass),
            "{hex}"
        );
    }
}

/// test_x86dec.py 67-74.
#[test]
fn decoding_depends_on_cpu_features() {
    let cases = ["f30fbcc1", "f30fbdc1", "f30f1efa", "f3490f1eca"];
    let all: Option<&[&str]> = None;
    for (features, expected) in [
        (all, ["TZCNT", "LZCNT", "ENDBR64", "RDSSPQ"]),
        (Some(&["BMI1", "LZCNT"][..]), ["TZCNT", "LZCNT", "NOP", "NOP"]),
        (Some(&[][..]), ["BSF", "BSR", "NOP", "NOP"]),
    ] {
        let got: Vec<String> = cases
            .iter()
            .map(|h| decode(&padded(h), features).unwrap().iclass)
            .collect();
        assert_eq!(got, expected, "{features:?}");
    }
}

/// test_x86dec.py 77-79.
#[test]
fn unknown_opcode_is_reported() {
    let e = decode(&padded("0f04"), None).unwrap_err();
    assert!(matches!(&e, DecodeError::NoEntry(m) if m.contains("no entry")), "{e:?}");
}

/// test_x86dec.py 82-97.
#[test]
fn invalid_encodings_are_refused() {
    for (hex, why) in [
        ("f04188a4d185e3ed61", "LOCK on MOV"),
        ("f00303", "LOCK with register-destination form 03 (add eax, [rbx])"),
        ("f001c0", "LOCK on register form of ADD"),
        ("c5e877c0", "VZEROUPPER needs VEX.vvvv=1111"),
        ("c5fd73348574", "VEX shift-by-immediate group is register-only"),
        (
            "f2660f6f00",
            "F2 takes precedence over 66 on 0F 6F, and F2 0F 6F has no row",
        ),
        ("660f5200", "RSQRTPS is NP"),
        ("660f01d0", "XGETBV is NP"),
        ("8dc0", "LEA with a register operand (mod = 3) is #UD"),
        ("f30f38f000", "MOVBE refuses F3"),
    ] {
        assert!(decode(&padded(hex), None).is_err(), "{hex}: {why}");
    }
}

/// test_x86dec.py 100-107.
#[test]
fn edge_rules_from_generated_differential() {
    for (hex, iclass) in [
        ("f30f1234", "MOVSLDUP"),
        ("660f38f000", "MOVBE"),
        ("f20fbc00", "BSF"),
        ("f30fc70f", "CMPXCHG8B"),
        ("66cf", "IRET"),
        ("67e300", "JECXZ"),
        ("f0490fc708", "CMPXCHG16B_LOCK"),
        ("f00100", "ADD_LOCK"),
        ("f0870a", "XCHG"),
        ("c0f401", "SHL"),
        ("f2a4", "REP_MOVSB"),
        ("f2a6", "REPNE_CMPSB"),
        ("f34390", "PAUSE"),
        ("0f18c0", "NOP"),
        ("660fb6c0", "MOVZX"),
    ] {
        assert_eq!(decode(&padded(hex), None).unwrap().iclass, iclass, "{hex}");
    }
}

/// test_x86dec.py 128-140.
#[test]
fn prefix_and_length_faults_are_invalid_encodings() {
    let long = "66".repeat(15);
    for hex in [
        "66c5f877",
        "f3c5f877",
        "f2c4e17d58c0",
        "6662f17c4858c0",
        "40c5f877",
        long.as_str(),
    ] {
        let e = decode(&bytes(hex), None).unwrap_err();
        assert!(matches!(e, DecodeError::InvalidEncoding(_)), "{hex}: {e:?}");
    }
}

/// test_x86dec.py 143-145.
#[test]
fn segment_and_address_size_prefixes_before_vex_are_allowed() {
    assert_eq!(decode(&bytes("2ec5f877"), None).unwrap().iclass, "VZEROUPPER");
    assert_eq!(decode(&bytes("67c5f877"), None).unwrap().length, 4);
}

/// test_x86dec.py 148-152.
#[test]
fn short_input_is_truncated() {
    for hex in ["", "66", "0f", "48", "c5", "e8000000", "488b0425"] {
        let e = decode(&bytes(hex), None).unwrap_err();
        assert!(matches!(e, DecodeError::Truncated(_)), "{hex:?}: {e:?}");
    }
}
