// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The run loop reaches the run's shared state only as a vCPU thread would (Sprint 324; the run-loop design's "How the
//! threads share `Guest`"): the loop is `run.rs`'s `vcpu_loop`, which holds the run's lock by `&`, so the compiler
//! refuses `GuestLock::get_mut`, which needs `&mut`, inside it or in anything it hands the lock to (E0596;
//! reviewer-3's Low 1 at 324: a scan of the loop's text for `get_mut` held a rule the borrow checker holds). What the
//! compiler does not hold is the signature itself: `vcpu_loop` taking `&mut GuestLock`, or `run_on_vcpu` (the set-up
//! around the loop; Sprint 326 named it, `run` now making its thread) keeping a loop of its own, would let a whole-lock
//! borrow back in. This test holds both.

use std::fs;
use std::path::Path;

/// The text of `name`'s signature in `text`: from `fn name(` to its `{`.
fn signature<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let start = text.find(&format!("\nfn {name}("))? + 1;
    let end = start + text[start..].find('{')?;
    Some(&text[start..end])
}

/// The body of `run_on_vcpu` in `text`: from `fn run_on_vcpu(` to the first line `}` after it.
fn run_body(text: &str) -> Option<&str> {
    let start = text.find("\nfn run_on_vcpu(")? + 1;
    let end = start + text[start..].find("\n}\n")?;
    Some(&text[start..end])
}

/// Whether `text` is a `run.rs` whose loop holds the lock by `&`: `vcpu_loop` takes `lock: &GuestLock<'_>`, and
/// `run_on_vcpu` calls it with `&lock` and has no `loop {` of its own. Err names what breaks the rule.
fn the_loop_holds_the_lock_by_ref(text: &str) -> Result<(), String> {
    let sig = signature(text, "vcpu_loop").ok_or("no `fn vcpu_loop(`")?;
    if !sig.contains("lock: &GuestLock<'_>,") {
        return Err(format!("`vcpu_loop` does not take `lock: &GuestLock<'_>`: {sig}"));
    }
    let run = run_body(text).ok_or("no `fn run_on_vcpu(`")?;
    if !run.contains("vcpu_loop(&lock,") {
        return Err("`run_on_vcpu` does not call `vcpu_loop(&lock, …)`".into());
    }
    if run
        .lines()
        .any(|l| l.split("//").next().unwrap_or("").contains("loop {"))
    {
        return Err("`run_on_vcpu` keeps a loop of its own".into());
    }
    Ok(())
}

#[test]
fn the_run_loop_holds_the_lock_by_ref() {
    let text = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/run.rs")).unwrap();
    assert_eq!(the_loop_holds_the_lock_by_ref(&text), Ok(()));
}

#[test]
fn a_mut_lock_or_a_loop_in_run_is_named() {
    let good = "\nfn run_on_vcpu(x: u8) {\n    let s = vcpu_loop(&lock, vcpu)?;\n}\n\nfn vcpu_loop(\n    lock: &GuestLock<'_>,\n) -> R {\n    loop {}\n}\n";
    assert_eq!(the_loop_holds_the_lock_by_ref(good), Ok(()));
    let by_mut = good.replace("lock: &GuestLock<'_>,", "lock: &mut GuestLock<'_>,");
    assert!(
        the_loop_holds_the_lock_by_ref(&by_mut)
            .unwrap_err()
            .contains("does not take")
    );
    let looped = good.replace(
        "    let s = vcpu_loop(&lock, vcpu)?;",
        "    let s = vcpu_loop(&lock, vcpu)?;\n    loop {}",
    );
    assert!(
        the_loop_holds_the_lock_by_ref(&looped)
            .unwrap_err()
            .contains("a loop of its own")
    );
}
