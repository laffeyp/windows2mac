// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 126's planted faults for the generator: a committed generated file one character off, and a vocabulary that
//! breaks a rule. Each runs the built binary on a copy of the repository's vocabulary and generated files.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use vocabulary_generator::{Vocabulary, check, current_vocabulary_path};

const COPIED: [&str; 3] = [
    "signals/rust-current.json",
    "crates/events/src/generated.rs",
    "harness/schemas/harness-events.json",
];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A scratch root holding the files the generator reads and checks; removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("w2m-vocabulary-generator-{}-{name}", std::process::id()));
        let vocabulary = current_vocabulary_path(&repository()).unwrap();
        let vocabulary = vocabulary
            .strip_prefix(repository())
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        for rel in COPIED.iter().copied().chain([vocabulary.as_str()]) {
            let to = root.join(rel);
            fs::create_dir_all(to.parent().unwrap()).unwrap();
            fs::copy(repository().join(rel), to).unwrap();
        }
        Scratch(root)
    }

    fn generator(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_vocabulary_generator"))
            .args(["--root", self.0.to_str().unwrap()])
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_committed_files_are_up_to_date_in_a_copy() {
    let scratch = Scratch::new("fresh");
    let out = scratch.generator(&["--check"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn one_character_changed_in_either_generated_file_fails_check_naming_it() {
    for rel in &COPIED[1..] {
        let scratch = Scratch::new(&rel.replace('/', "-"));
        let path = scratch.0.join(rel);
        let mut text = fs::read_to_string(&path).unwrap();
        let at = text.find("SESSION_INIT").unwrap();
        text.replace_range(at..at + 1, "Z");
        fs::write(&path, text).unwrap();
        let out = scratch.generator(&["--check"]);
        assert_eq!(out.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&out.stderr).contains(&format!("stale: {rel}")));
    }
}

#[test]
fn a_hand_written_crate_naming_a_deprecated_tag_fails_check_naming_both() {
    let scratch = Scratch::new("deprecated-emit-site");
    let src = scratch.0.join("crates/planted/src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.rs"), "pub const TAG: &str = \"GUEST_THREAD_EXITED\";\n").unwrap();
    fs::write(
        src.join("other.rs"),
        "// GUEST_THREAD_EXITEDX is a longer word, not the tag\n",
    )
    .unwrap();
    let out = scratch.generator(&["--check"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&out.stderr).trim(),
        "crates/planted/src/lib.rs: names GUEST_THREAD_EXITED, deprecated in vocabulary 0.11"
    );
}

#[test]
fn a_deprecation_without_its_proposal_is_refused_by_name() {
    let errs = planted(|raw| {
        tag_mut(raw, "GUEST_THREAD_EXITED")["deprecated"]["proposal"] = Value::from("twenty-one");
    });
    assert_eq!(
        errs,
        ["GUEST_THREAD_EXITED: a deprecation names its date (YYYY-MM-DD), its proposal (Pn) and a reason"]
    );
}

/// The current vocabulary with one edit, checked.
fn planted(edit: impl FnOnce(&mut Value)) -> Vec<String> {
    let path = current_vocabulary_path(&repository()).unwrap();
    let mut raw: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    edit(&mut raw);
    check(&Vocabulary::parse(&raw.to_string()).unwrap()).0
}

fn tag_mut<'v>(raw: &'v mut Value, name: &str) -> &'v mut Value {
    raw["tags"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["name"] == name)
        .unwrap()
}

#[test]
fn a_tag_whose_verb_is_not_past_tense_is_refused_by_name() {
    let errors = planted(|raw| tag_mut(raw, "SESSION_COMPLETED")["name"] = Value::from("SESSION_COMPLETE"));
    assert!(
        errors.contains(&"SESSION_COMPLETE: verb COMPLETE is not past tense".to_string()),
        "{errors:?}"
    );
}

#[test]
fn a_field_of_an_unknown_type_is_refused_by_name() {
    let errors = planted(|raw| tag_mut(raw, "SESSION_INIT")["types"]["profile"] = Value::from("uuid"));
    assert_eq!(errors, ["SESSION_INIT.profile: unknown type uuid"]);
}

#[test]
fn the_binary_refuses_a_planted_vocabulary_and_writes_nothing() {
    let scratch = Scratch::new("planted");
    let path = current_vocabulary_path(&scratch.0).unwrap();
    let text = fs::read_to_string(&path)
        .unwrap()
        .replacen("\"enum:session_purpose\"", "\"enum:no_such_enum\"", 1);
    fs::write(&path, text).unwrap();
    let before = fs::read(scratch.0.join(COPIED[1])).unwrap();
    let out = scratch.generator(&[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("SESSION_INIT.purpose: unknown enum enum:no_such_enum"));
    assert_eq!(fs::read(scratch.0.join(COPIED[1])).unwrap(), before);
}

#[test]
fn names_rust_cannot_take_are_refused() {
    let errors = planted(|raw| {
        let t = tag_mut(raw, "SESSION_INIT");
        t["payload"][1] = Value::from("type");
        let ty = t["types"].as_object_mut().unwrap().remove("profile").unwrap();
        t["types"]["type"] = ty;
        raw["enums"]["session_purpose"][0] = Value::from("Contract-Construction");
    });
    assert!(
        errors.contains(&"SESSION_INIT.type: a field is a lowercase identifier and not a Rust keyword".to_string())
    );
    assert!(
        errors
            .iter()
            .any(|e| e.starts_with("enum:session_purpose: value \"Contract-Construction\"")),
        "{errors:?}"
    );
}

#[test]
fn camel_case_names_the_types() {
    assert_eq!(vocabulary_generator::camel("SESSION_INIT"), "SessionInit");
    assert_eq!(
        vocabulary_generator::camel("completed_with_discrepancies"),
        "CompletedWithDiscrepancies"
    );
}

#[test]
fn a_tag_named_only_inside_a_longer_word_is_not_named() {
    let errors = planted(|raw| {
        for c in raw["evidence_constraints"]["constraints"].as_array_mut().unwrap() {
            if c["id"] == "E8" {
                let spec = c["spec"]
                    .as_str()
                    .unwrap()
                    .replace("SESSION_COMPLETED.outcome", "XSESSION_COMPLETEDX.outcome");
                c["spec"] = Value::from(spec);
            }
        }
    });
    assert_eq!(
        errors,
        ["SESSION_COMPLETED: summary tag without an outcome constraint (E8)"]
    );
}
