// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 133's judge: the runtime image Rust builds equals, byte for byte, the one the frozen Python host's build
//! cached under the same key in `build/guest_runtime/`. Xcode's clang and `ld` build it, so the test runs on macOS.
#![cfg(target_os = "macos")]

use std::fs;
use std::path::{Path, PathBuf};

use guest_runtime::layout::{RUNTIME_IMAGE_VA, runtime_defines};
use guest_runtime::{build, cache_key, services, texts};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn the_image_rust_builds_is_the_python_builds_byte_for_byte() {
    let root = root();
    let (key, image) = build(&root, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    let python = root.join(format!("build/guest_runtime/{key}.bin"));
    assert!(
        python.exists(),
        "no Python image under key {key}: a define, a header or a source differs from the Python build's"
    );
    let ours = fs::read(&image.file).unwrap();
    let theirs = fs::read(&python).unwrap();
    assert_eq!(ours.len(), theirs.len(), "sizes differ");
    assert!(
        ours == theirs,
        "the images differ at byte {:?}",
        ours.iter().zip(&theirs).position(|(a, b)| a != b)
    );
    let texts = texts(&root).unwrap();
    let missing: Vec<String> = services::services(&texts)
        .unwrap()
        .into_iter()
        .filter(|s| !image.symbols.contains_key(&s.symbol))
        .map(|s| s.symbol)
        .collect();
    assert!(missing.is_empty(), "the image does not define {missing:?}");
    println!(
        "python_image: key {key}, {} bytes, {} segments, {} symbols",
        ours.len(),
        image.segments.len(),
        image.symbols.len()
    );
}

#[test]
fn a_define_changed_by_one_names_an_image_python_never_built() {
    let root = root();
    let mut defines = runtime_defines();
    for (k, v) in &mut defines {
        if k == "W2M_TIME_SLICE" {
            *v += 1;
        }
    }
    let key = cache_key(&root, &texts(&root).unwrap(), &defines, RUNTIME_IMAGE_VA).unwrap();
    assert!(!root.join(format!("build/guest_runtime/{key}.bin")).exists());
    let right = cache_key(&root, &texts(&root).unwrap(), &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    assert_ne!(key, right);
}

#[test]
fn a_source_byte_changed_names_another_key() {
    let root = root();
    let mut t = texts(&root).unwrap();
    let right = cache_key(&root, &t, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    t.get_mut("runtime.c").unwrap().push(' ');
    let key = cache_key(&root, &t, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    assert_ne!(key, right);
    assert!(!root.join(format!("build/guest_runtime/{key}.bin")).exists());
}
