// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 137's judge, carried from `tests/test_pe.py`: the reader on the executables under `store/binaries/`, checked
//! against llvm-objdump (an independent reader), plus malformed inputs. Each test cites the Python test's lines.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pe_image::{CodeTarget, PeImage, parse};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// test_pe.py 20.
fn binaries() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(root().join("store/binaries"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "exe"))
        .collect();
    out.sort();
    out
}

/// test_pe.py 21-22: the DLLs the probes build.
fn probe_dlls() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for e in fs::read_dir(root().join("probes")).unwrap() {
        let p = e.unwrap().path().join("probe.json");
        if let Ok(text) = fs::read_to_string(&p) {
            let v: serde_json::Value = serde_json::from_str(&text).unwrap();
            if let Some(name) = v["dll"]["name"].as_str() {
                out.insert(format!("{name}.dll"));
            }
        }
    }
    out
}

/// test_pe.py 23-24: system DLLs a family probes (command_line_args, generate_random; known_folders;
/// integrity_level).
const SYSTEM_DLLS: [&str; 8] = [
    "SHELL32.dll",
    "bcrypt.dll",
    "ole32.dll",
    "ADVAPI32.dll",
    "OLEAUT32.dll",
    "WS2_32.dll",
    "WINMM.dll",
    "USER32.dll",
];

/// test_pe.py 25-27.
fn objdump() -> Option<String> {
    let found = |cmd: &mut Command| {
        cmd.output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    };
    found(Command::new("which").arg("llvm-objdump"))
        .or_else(|| found(Command::new("xcrun").args(["--find", "llvm-objdump"])))
}

fn objdump_p(tool: &str, path: &Path) -> String {
    let out = Command::new(tool).arg("-p").arg(path).output().unwrap();
    assert!(out.status.success(), "llvm-objdump -p {}", path.display());
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// `\s+` then the rest: the line's tokens when it starts with whitespace and does not end with it.
fn indented_tokens(line: &str) -> Option<Vec<&str>> {
    let starts = line.chars().next().is_some_and(char::is_whitespace);
    let ends = line.chars().last().is_some_and(|c| !c.is_whitespace());
    (starts && ends).then(|| line.split_whitespace().collect())
}

fn digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// test_pe.py 30-48.
fn objdump_imports(text: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut imports: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut dll: Option<String> = None;
    for line in text.lines() {
        if let Some(name) = line.trim_start().strip_prefix("DLL Name: ")
            && line.starts_with(char::is_whitespace)
            && let Some(name) = name.split_whitespace().next()
        {
            dll = Some(name.to_string());
            imports.insert(name.to_string(), BTreeSet::new());
            continue;
        }
        if let (Some(d), Some(t)) = (&dll, indented_tokens(line)) {
            match t.as_slice() {
                [n, name] if digits(n) => {
                    imports.get_mut(d).unwrap().insert(name.to_string());
                }
                // an import by ordinal: the number alone (Sprint 052)
                [n] if digits(n) => {
                    imports.get_mut(d).unwrap().insert(format!("#{n}"));
                }
                _ => {}
            }
        }
        if line.trim().starts_with("lookup ") {
            dll = None;
        }
    }
    imports
}

/// test_pe.py 51-55.
fn objdump_field(text: &str, name: &str) -> u64 {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix(name)
            && rest.starts_with(char::is_whitespace)
            && let Some(hex) = rest.split_whitespace().next()
            && let Ok(v) = u64::from_str_radix(hex, 16)
        {
            return v;
        }
    }
    panic!("{name}");
}

fn read(path: &Path) -> (Vec<u8>, PeImage) {
    let data = fs::read(path).unwrap();
    let img = parse(&data, false).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    (data, img)
}

/// test_pe.py 58-59.
#[test]
fn fixtures_present() {
    assert!(binaries().len() >= 2);
}

/// test_pe.py 62-68.
#[test]
fn matches_llvm_objdump() {
    let Some(tool) = objdump() else {
        println!("llvm-objdump not available: skipped, as test_pe.py 62 skips");
        return;
    };
    let paths = binaries();
    for path in &paths {
        let (_, img) = read(path);
        let text = objdump_p(&tool, path);
        let name = path.file_name().unwrap().to_string_lossy();
        assert_eq!(
            u64::from(img.entry_rva),
            objdump_field(&text, "AddressOfEntryPoint"),
            "{name}: the entry"
        );
        assert_eq!(
            img.image_base,
            objdump_field(&text, "ImageBase"),
            "{name}: the image base"
        );
        let ours: BTreeMap<String, BTreeSet<String>> = img
            .imports
            .iter()
            .map(|(d, n)| (d.clone(), n.iter().cloned().collect()))
            .collect();
        let theirs = objdump_imports(&text);
        for (dll, names) in &theirs {
            assert_eq!(ours.get(dll), Some(names), "{name}: {dll}'s imports");
        }
        assert_eq!(ours, theirs, "{name}: the imported DLLs");
    }
    println!(
        "matches_llvm_objdump: {} executables agree with llvm-objdump",
        paths.len()
    );
}

/// test_pe.py 71-86.
#[test]
fn structure() {
    let probes = probe_dlls();
    for path in &binaries() {
        let (_, img) = read(path);
        let name = path.file_name().unwrap().to_string_lossy();
        let text = img.sections.iter().find(|s| s.name == ".text").unwrap();
        assert_eq!(text.flags(), "r-x", "{name}");
        assert!(text.virtual_address <= img.entry_rva && img.entry_rva < text.virtual_address + text.virtual_size);
        assert!(
            img.relocations.keys().all(|t| [0, 10].contains(t)),
            "{name}: only DIR64 and ABSOLUTE padding"
        );
        assert!(
            img.code_targets()
                .contains(&CodeTarget::Static(u64::from(img.entry_rva)))
        );
        if img.function_starts.len() > 400 {
            // CRT-linked probes (hello, event_auto_reset): their .pdata
            let kernel32 = img.imports.iter().find(|(d, _)| d == "KERNEL32.dll").unwrap();
            assert!(kernel32.1.iter().any(|n| n == "WaitForSingleObject"), "{name}");
        } else {
            // no-CRT probes (Sprint 029): kernel32, and a DLL the probe builds (Sprint 046), or a system DLL a
            // family probes (Sprint 047)
            for (dll, _) in &img.imports {
                assert!(
                    dll == "KERNEL32.dll" || SYSTEM_DLLS.contains(&dll.as_str()) || probes.contains(dll),
                    "{name}: {dll}"
                );
            }
        }
    }
}

fn first_binary() -> Vec<u8> {
    fs::read(&binaries()[0]).unwrap()
}

/// test_pe.py 89-97.
#[test]
fn malformed() {
    let data = first_binary();
    let no_mz = [b"XX".as_slice(), &data[2..]].concat();
    let bad_sig = [&data[..0x3C], &0x200u32.to_le_bytes(), &data[0x40..]].concat();
    let truncated = data[..0x200].to_vec();
    for (name, bytes, needle) in [
        ("no MZ", no_mz, "MZ"),
        ("bad PE sig", bad_sig, "PE signature"),
        ("truncated", truncated, "beyond file size"),
    ] {
        let e = parse(&bytes, false).expect_err(name);
        assert!(e.0.contains(needle), "{name}: {e}");
    }
}

/// test_pe.py 100-105.
#[test]
fn wrong_machine_refused() {
    let mut data = first_binary();
    let lfanew = u32::from_le_bytes(data[0x3C..0x40].try_into().unwrap()) as usize;
    data[lfanew + 4..lfanew + 6].copy_from_slice(&0x14Cu16.to_le_bytes()); // i386
    let e = parse(&data, false).unwrap_err();
    assert!(e.0.contains("not AMD64"), "{e}");
}

/// test_pe.py 108-115: no invented bytes past a section's virtual extent (Sprint 023).
#[test]
fn read_rva_stops_at_section_end() {
    let data = first_binary();
    let img = parse(&data, false).unwrap();
    let s = &img.sections[0];
    let end = u64::from(s.virtual_address) + u64::from(s.virtual_size.max(s.raw_size));
    assert_eq!(img.read_rva(&data, end - 3, 15).unwrap().len(), 3);
    assert_eq!(img.read_rva(&data, u64::from(s.virtual_address), 15).unwrap().len(), 15);
}

/// test_pe.py 118-128: directory 13 read; every delay IAT slot is named (Sprint 026).
#[test]
fn delay_load_imports_resolve() {
    let path = root().join("corpus/python-3.14.8/_wmi.pyd");
    if !path.exists() {
        println!("corpus not downloaded: skipped, as test_pe.py 121 skips");
        return;
    }
    let (_, img) = read(&path);
    let names = [
        "CoInitializeEx",
        "CoCreateInstance",
        "CoSetProxyBlanket",
        "CoInitializeSecurity",
        "CoUninitialize",
    ];
    assert_eq!(
        img.delay_imports,
        vec![("ole32.dll".to_string(), names.iter().map(|n| n.to_string()).collect())]
    );
    assert_eq!(img.iat.values().filter(|v| v.starts_with("ole32.dll!")).count(), 5);
}

/// test_pe.py 131-135.
#[test]
fn relocation_sites_match_counts() {
    for path in &binaries() {
        let (_, img) = read(path);
        let counted: u64 = img.relocations.iter().filter(|(t, _)| **t != 0).map(|(_, n)| n).sum();
        assert_eq!(img.relocation_sites.len() as u64, counted);
        assert!(img.relocation_sites.iter().all(|(_, t)| *t == 10)); // IMAGE_REL_BASED_DIR64 only
    }
}
