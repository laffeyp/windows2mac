// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! PEB.Ldr's three lists as a LoadLibrary rewrites them (Sprint 291; reviewer-3's Low 1 at 290): every state between
//! two of `process::write_loader_data`'s writes is a whole list, so a guest thread on another vCPU walking `PEB->Ldr`
//! while the request is served never follows a link not yet written.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use guest_runtime::layout::{LOADER_DATA_OFF, LOADER_DATA_SIZE, PEB_OFF, PROCESS_VA};
use w2m_vm::process::{LoaderList, ProcessArea, ProcessBytes, write_loader_data};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Each write in the order made: the offset in the process area and the bytes.
#[derive(Default)]
struct Recorded(Vec<(u64, Vec<u8>)>);

impl ProcessBytes for Recorded {
    fn put(&mut self, off: u64, bytes: &[u8]) {
        self.0.push((off, bytes.to_vec()));
    }
}

/// An image at `base`, its headers a stored binary's, as `virtual_memory.rs` builds one.
fn image(name: &str, base: u64) -> pe_loader::Image {
    let raw =
        fs::read(root().join("store/binaries/8385bc74e8db79d6ef5c62d4ebd0dc161e7985f85f4d763dc73dd42bed2a137d.exe"))
            .unwrap();
    pe_loader::Image {
        name: name.into(),
        pe: pe_image::parse(&raw, false).unwrap(),
        base,
    }
}

fn word(area: &[u8], va: u64) -> u64 {
    let at = (va - PROCESS_VA) as usize;
    u64::from_le_bytes(area[at..at + 8].try_into().unwrap())
}

/// Each list walked forward and back from its head: None when every walk returns to its head through entries whose
/// `DllBase` is one of `bases`, else what the walker met.
fn broken(area: &[u8], bases: &BTreeSet<u64>) -> Option<String> {
    let va0 = PROCESS_VA + LOADER_DATA_OFF;
    for (head, link) in [(0x10, 0x00), (0x20, 0x10), (0x30, 0x20)] {
        for (way, step) in [("forward", 0), ("back", 8)] {
            let head = va0 + head;
            let mut node = word(area, head + step);
            for _ in 0..64 {
                if node == head {
                    break;
                }
                if !(va0..va0 + LOADER_DATA_SIZE).contains(&node) {
                    return Some(format!(
                        "list 0x{head:x} {way}: a link to 0x{node:x}, outside the loader data"
                    ));
                }
                let dll_base = word(area, node - link + 0x30);
                if !bases.contains(&dll_base) {
                    return Some(format!(
                        "list 0x{head:x} {way}: an entry at 0x{node:x} with DllBase 0x{dll_base:x}"
                    ));
                }
                node = word(area, node + step);
            }
            if node != head {
                return Some(format!("list 0x{head:x} {way}: no way back to the head in 64 steps"));
            }
        }
    }
    None
}

#[test]
fn every_state_of_a_loadlibrarys_lists_is_a_whole_list() {
    let area = ProcessArea {
        stack: (0, 0, 0),
        environment_bytes: 0,
        system: BTreeMap::new(),
        kernel32: PROCESS_VA + 0x10_0000,
        kernelbase: PROCESS_VA + 0x11_0000,
        ntdll: PROCESS_VA + 0x12_0000,
        process_id: 4,
    };
    let images = [
        image("program.exe", 0x1_4000_0000),
        image("first.dll", 0x1_8000_0000),
        image("loaded.dll", 0x1_9000_0000),
    ];
    let paths = BTreeMap::new();
    let list = |n: usize, order: &'static [usize]| LoaderList {
        images: &images[..n],
        image_entry: 0x1_4000_1000,
        image_name: "program.exe",
        dependency_order: order,
        paths: &paths,
    };
    // the lists as they stood: the program and one DLL; then a LoadLibrary of a third, placed first in init order to
    // insert it mid-list there and at the tail of the other two
    let mut before = vec![0u8; (LOADER_DATA_OFF + LOADER_DATA_SIZE).max(PEB_OFF + 0x20) as usize];
    write_loader_data(&mut before, &list(2, &[1]), &area).unwrap();
    let mut writes = Recorded::default();
    write_loader_data(&mut writes, &list(3, &[2, 1]), &area).unwrap();
    let bases: BTreeSet<u64> = images
        .iter()
        .map(|i| i.base)
        .chain([area.kernel32, area.kernelbase, area.ntdll])
        .collect();
    assert_eq!(broken(&before, &bases), None, "the lists before the load");
    let mut state = before;
    for (k, (off, bytes)) in writes.0.iter().enumerate() {
        state.put(*off, bytes);
        if let Some(why) = broken(&state, &bases) {
            panic!("after write {k} of {} (0x{off:x}): {why}", writes.0.len());
        }
    }
    // the load done: the lists as a load into empty memory writes them, the new DLL in each, and PEB.Ldr set
    let mut whole = vec![0u8; state.len()];
    write_loader_data(&mut whole, &list(3, &[2, 1]), &area).unwrap();
    let range = LOADER_DATA_OFF as usize..(LOADER_DATA_OFF + LOADER_DATA_SIZE) as usize;
    assert!(
        state[range.clone()] == whole[range],
        "the rewritten lists differ from the lists written whole"
    );
    for (head, link) in [(0x10, 0x00), (0x20, 0x10), (0x30, 0x20)] {
        let order = forward(&state, PROCESS_VA + LOADER_DATA_OFF + head, link);
        assert!(
            order.contains(&0x1_9000_0000),
            "list 0x{head:x} lacks the loaded DLL: {order:x?}"
        );
    }
    assert_eq!(word(&state, PROCESS_VA + PEB_OFF + 0x18), PROCESS_VA + LOADER_DATA_OFF);
}

/// The `DllBase`s of a whole list, walked forward from its head.
fn forward(area: &[u8], head: u64, link: u64) -> Vec<u64> {
    let mut out = Vec::new();
    let mut node = word(area, head);
    while node != head {
        out.push(word(area, node - link + 0x30));
        node = word(area, node);
    }
    out
}
