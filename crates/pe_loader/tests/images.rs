// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 138's judge: `tests/test_loader_images.py` carried (several images, Sprint 046), the rows Windows showed for
//! a DLL's placement and static TLS (`contracts/windows.loader.static_import`, `contracts/windows.loader.static_tls`),
//! and the loader stub against clang's assembly. The images are probe binaries MSVC built on the Windows runners
//! (`store/binaries`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pe_image::parse;
use pe_loader::{DLL_BASE, DLL_INIT_FAILED, LOADER_STUB, LoadError, Loaded, Memory, TLS_SLOTS, load, map_image};

fn binary(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../store/binaries")
            .join(name),
    )
    .unwrap()
}

// test_loader_images.py 18-23: family "loader", run 38047927781: loader.exe imports loader_dll.dll, which has no
// relocations; family "static TLS", windows-2022: static_tls_dll.dll is built on the static C runtime, 780 DIR64 sites
const LOADER_EXE: &str = "2bc372b71143f8a74323f8ecac7f5a73ec622d6a329e9000647f5e779d6d8e04.exe";
const LOADER_DLL: &str = "6a3dd26e2da2b4f25525c3f0940e7d03b6c3b1bf3ce325808d738ce2a0c62ff7.dll";
const TLS_EXE: &str = "f9d9233c10ced3784aecdeba7de1de96b7931296d3548217383c4f6529069ada.exe";
const TLS_DLL: &str = "3f6b51cc422038c706dda36ef9d49002c8869402d7161f6d31e8f46aa0ba1ff2.dll";

fn load_with(exe: &str, dll_name: &str, dll: Vec<u8>) -> Result<Loaded, LoadError> {
    let supply = move |n: &str| (n == dll_name).then(|| dll.clone());
    load(&binary(exe), Some(&supply), "probe.exe")
}

fn u64_at(data: &[u8], off: u64) -> u64 {
    u64::from_le_bytes(data[off as usize..off as usize + 8].try_into().unwrap())
}

/// test_loader_images.py 26-43: static_tls_dll.dll at its preferred base and at `DLL_BASE` differs exactly at its
/// DIR64 sites (780), each by the delta.
#[test]
fn relocations_move_every_site_by_the_delta_and_nothing_else() {
    let raw = binary(TLS_DLL);
    let pe = parse(&raw, false).unwrap();
    let (mut a, mut b) = (Memory::default(), Memory::default());
    map_image(&mut a, &raw, &pe, pe.image_base, "a").unwrap();
    map_image(&mut b, &raw, &pe, DLL_BASE, "b").unwrap();
    let (x, y) = (&a.regions[0].data, &b.regions[0].data);
    let delta = DLL_BASE.wrapping_sub(pe.image_base);
    let sites: Vec<u64> = pe
        .relocation_sites
        .iter()
        .filter(|(_, k)| *k == 10)
        .map(|(r, _)| u64::from(*r))
        .collect();
    assert_eq!((sites.len(), pe.relocation_sites.len()), (780, 780));
    for rva in &sites {
        assert_eq!(
            u64_at(y, *rva).wrapping_sub(u64_at(x, *rva)),
            delta,
            "the site at RVA 0x{rva:x}"
        );
    }
    let mut covered = vec![false; x.len()];
    for rva in &sites {
        covered[*rva as usize..*rva as usize + 8].fill(true);
    }
    let moved: Vec<usize> = (0..x.len()).filter(|i| !covered[*i] && x[*i] != y[*i]).collect();
    assert!(
        moved.is_empty(),
        "bytes moved outside the sites: {:x?}",
        &moved[..moved.len().min(8)]
    );
}

/// test_loader_images.py 46-56.
#[test]
fn the_program_binds_the_dll_exports_and_starts_the_dll_first() {
    let ld = load_with(LOADER_EXE, "loader_dll.dll", binary(LOADER_DLL)).unwrap();
    let names: Vec<&str> = ld.images.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["probe.exe", "loader_dll.dll"]);
    let library = &ld.images[1];
    assert_eq!(library.base, library.pe.image_base, "preferred base, free here");
    let region = ld.memory.regions.iter().find(|r| r.name == "image").unwrap();
    for export in ["loader_dll_call", "loader_dll_count"] {
        let (slot, _) = ld
            .image
            .iat
            .iter()
            .find(|(_, n)| **n == format!("loader_dll.dll!{export}"))
            .unwrap();
        assert_eq!(
            u64_at(&region.data, u64::from(*slot)),
            library.export(export).unwrap().unwrap(),
            "{export}"
        );
    }
    assert_eq!((ld.entry, ld.init_order.as_slice()), (LOADER_STUB, [1].as_slice()));
    assert!(ld.thunks.values().all(|n| !n.starts_with("loader_dll.dll!")));
}

/// test_loader_images.py 59-64: Windows refuses to start a process whose import a DLL does not export; so does the
/// loader.
#[test]
fn a_missing_export_refuses_the_load() {
    let mut broken = binary(LOADER_DLL);
    let at = broken.windows(16).position(|w| w == b"loader_dll_call\0").unwrap();
    broken[at..at + 16].copy_from_slice(b"loader_dll_cell\0");
    let e = load_with(LOADER_EXE, "loader_dll.dll", broken).unwrap_err();
    assert!(e.to_string().contains("loader_dll_call"), "{e}");
}

/// `contracts/windows.loader.static_import` row `dll.base`: at the preferred base, above 4 GB, 64 KB aligned.
#[test]
fn the_dll_sits_where_windows_put_it() {
    let ld = load_with(LOADER_EXE, "loader_dll.dll", binary(LOADER_DLL)).unwrap();
    let base = ld.images[1].base;
    assert_eq!(base, ld.images[1].pe.image_base);
    assert!(base > 1 << 32 && base.is_multiple_of(0x10000), "0x{base:x}");
}

/// `contracts/windows.loader.static_tls` rows `tls.index` and `tls.array`: a program without TLS importing a DLL with
/// it; the DLL's index is 1, the array's slot 1 holds the template with its zero fill, the program's slot is NULL.
#[test]
fn static_tls_as_windows_sets_it() {
    let ld = load_with(TLS_EXE, "static_tls_dll.dll", binary(TLS_DLL)).unwrap();
    assert!(ld.images[0].pe.tls.is_none());
    let dll = &ld.images[1];
    let tls = dll.pe.tls.unwrap();
    let region = ld.memory.regions.iter().find(|r| r.base == dll.base).unwrap();
    let off = |va: u64| (va - dll.pe.image_base) as usize;
    let index = u32::from_le_bytes(region.data[off(tls.index)..off(tls.index) + 4].try_into().unwrap());
    assert_eq!(index, 1, "tls.index");
    let array = ld.memory.read(ld.tls_array, 8 * TLS_SLOTS as u64).unwrap();
    assert_eq!(u64_at(&array, 0), 0, "the program's slot");
    let block_va = u64_at(&array, 8);
    let mut template = region.data[off(tls.start)..off(tls.end)].to_vec();
    template.resize(template.len() + tls.zero_fill as usize, 0);
    assert_eq!(
        ld.memory.read(block_va, template.len() as u64).unwrap(),
        template,
        "tls.array"
    );
}

fn tool(name: &str) -> Option<PathBuf> {
    let out = Command::new("xcrun").args(["--find", name]).output().ok()?;
    let path = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (out.status.success() && !path.is_empty()).then(|| PathBuf::from(path))
}

/// The `.text` section's bytes of a COFF object (PE Format, "COFF File Header" and "Section Table").
fn coff_text(obj: &[u8]) -> Vec<u8> {
    let sections = u16::from_le_bytes([obj[2], obj[3]]) as usize;
    let optional = u16::from_le_bytes([obj[16], obj[17]]) as usize;
    let u32_at = |o: usize| u32::from_le_bytes(obj[o..o + 4].try_into().unwrap()) as usize;
    for i in 0..sections {
        let h = 20 + optional + 40 * i;
        if &obj[h..h + 8] == b".text\0\0\0" {
            let (size, at) = (u32_at(h + 16), u32_at(h + 20));
            return obj[at..at + size].to_vec();
        }
    }
    panic!("no .text section");
}

/// The loader stub for loader.exe (loader.py 180-203) equals clang's assembly of the instructions its comments name.
#[test]
fn the_stub_is_the_instructions_it_names() {
    let Some(clang) = tool("clang") else {
        eprintln!("clang not found: the stub is not checked here");
        return;
    };
    let ld = load_with(LOADER_EXE, "loader_dll.dll", binary(LOADER_DLL)).unwrap();
    let dll = &ld.images[1];
    let failed = *ld.thunks.iter().find(|(_, n)| *n == DLL_INIT_FAILED).unwrap().0;
    let source = format!(
        ".intel_syntax noprefix\n.text\n\
         sub rsp, 0x28\nmovabs rcx, {base:#x}\nmov edx, 1\nmovabs r8, {stub:#x}\nmovabs rax, {entry:#x}\ncall rax\n\
         add rsp, 0x28\ntest eax, eax\njne 1f\nmovabs rax, {failed:#x}\ncall rax\n1:\n\
         movabs rax, {program:#x}\njmp rax\n",
        base = dll.base,
        stub = LOADER_STUB,
        entry = dll.entry().unwrap(),
        program = ld.image_entry,
    );
    let dir = std::env::temp_dir().join(format!("w2m-pe-loader-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let (src, obj) = (dir.join("stub.s"), dir.join("stub.o"));
    fs::write(&src, source).unwrap();
    let built = Command::new(clang)
        .args(["-c", "--target=x86_64-pc-windows-msvc", "-x", "assembler"])
        .arg(&src)
        .arg("-o")
        .arg(&obj)
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let clangs = coff_text(&fs::read(&obj).unwrap());
    let _ = fs::remove_dir_all(&dir);
    let ours = ld.memory.read(LOADER_STUB, clangs.len() as u64 + 16).unwrap();
    assert_eq!(&ours[..clangs.len()], clangs.as_slice());
    assert!(
        ours[clangs.len()..].iter().all(|b| *b == 0),
        "the stub runs past clang's bytes"
    );
    println!("the loader stub: {} bytes equal clang's", clangs.len());
}
