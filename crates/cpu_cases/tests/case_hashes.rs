// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `case_hashes` against the frozen Python's (Sprint 167): each committed case set's first case, its program and
//! initial state hashed as `scripts/class_run.py` `case_hashes` hashes them. The expected values were computed
//! 2026-10-28 by running the frozen Python (`semantics_differential.read_cases`, `class_run.case_hashes`) on each set.
//! Every committed set is format 2; the format-3 and format-4 sets (`probes/semantics_049`, `semantics_065`) are built,
//! not committed, and were checked against the Python once, at Sprint 167's close.

use std::fs;
use std::path::Path;

use cpu_cases::{case_hashes, read_cases};

/// (set, program, initial state) of each set's first case, from the frozen Python.
const PYTHON: [(&str, &str, &str); 13] = [
    (
        "semantics_042",
        "8ecbc6fbeac01bfccfd47a651f324353653818d79422509e16c13c9210159197",
        "6bf42f8ba380e8f453235b40ac2e5a64deeb994909622b8589e066a1580692ba",
    ),
    (
        "semantics_044",
        "ababfee53ef10b55bbfc1c0fb86a24aa04d67b1961629642a9ebd789c80a2ea4",
        "7537efe115732095727768b14de2a89faa7f37ccb95974b641755c9507055cf7",
    ),
    (
        "semantics_044b",
        "e1b60aa5c6c2c5b1d719ea116190a96abca45bed22ee73cbd101320266c6e520",
        "f411a68a046d271576a9a7eb378de81a855ab4ea15cfc1ab3ef5e0cda9f785f8",
    ),
    (
        "semantics_044c",
        "74d46cc014b4faef8bd101f3103ee7f144d6075136a00a7a574c500c62f628a9",
        "5b83034edda23fe551516ee7d6466a876bd67d6e4875b34f6aac8d2305c1b555",
    ),
    (
        "semantics_044d",
        "01a761f57d6a9e00be22a370bff4351abfe61602a244ffc9612ad90e39930b48",
        "aaecb1fc0f6011d2c83c6c5c51d4ae0efc11187764ff1899491cdeac5bbd8c39",
    ),
    (
        "semantics_044e",
        "b6cbececf1b07e74f37d57710c68c31f0b92cb08d51093252b321b07542e177b",
        "a8b41df06dbef80f2083e889ead877d99f6326d5f6e36b1e863a02d9cad17c2f",
    ),
    (
        "semantics_046",
        "db38e46926688511ff8f6b0a5b218fc28c457b1f7949a2fd4e6ae55e427859f6",
        "de1ed95efca417390808f42873044621bdea1f1fd794762c52688512078b459b",
    ),
    (
        "semantics_046b",
        "36fe7ad4290a164a51c890e0657c8ab999962134f23de5d980d25faae7b930b6",
        "6672da39ce7d24361a58677d291b2897af25bf6bba17666839c2cd8b39833362",
    ),
    (
        "semantics_046c",
        "6b6c55ad7a705816f0185734602203c9d52fb03cb40fe9bf48ea0144638c1cf3",
        "9c3d632942dfedfd743d8e4a0f40f30af8a68ff1cc731e9fef796b8717e07fd9",
    ),
    (
        "semantics_046d",
        "192f6b871c0a939607c1f6496250ddca7672eee688b94b1b9fc68ce214834f8a",
        "a183c85de1db2496988bd4c42f53af35e3470f5b64a3a2aa8ff984b339d473dc",
    ),
    (
        "semantics_046e",
        "6da8d2dad0b86d9794fffb926173dd021a717f33df2e7d001e4d0de85aef1948",
        "5203a11b019c339c6e43467b454ab7a08b45419c1b54090944a153f3c33c8678",
    ),
    (
        "semantics_047",
        "e3de2152aaa4a2cd10b6ec3a2d13b5b42f49f8b92881990725818bccc0b025ab",
        "9a20eeae3e2bd04403881eda0ec8897b53619ad7fb0d9ec6bbbda9afd44e5ef8",
    ),
    (
        "semantics_048",
        "73133d3db3988a7b345c0096c440b9c9382a63dda0fb0ce23221075415d57df2",
        "a19f9e1dc80442bd64c511bf78cfd9fee4df09dcfc77388cef1f66599a64b33d",
    ),
];

#[test]
fn each_sets_first_case_hashes_as_the_python_hashes_it() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (set, program, state) in PYTHON {
        let raw = fs::read(root.join("probes").join(set).join("cases.bin")).unwrap();
        let (_, cases) = read_cases(&raw).unwrap();
        let (p, s) = case_hashes(&cases[0]);
        assert_eq!(p, format!("sha256:{program}"), "{set}: program");
        assert_eq!(s, format!("sha256:{state}"), "{set}: initial state");
    }
}
