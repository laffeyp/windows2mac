// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 146's judge: the arena (`crates/virtual_memory`) with the host's layout, held to the rows Windows printed.
//! The probes' calls are replayed in their order against the arena, each row derived as the probe prints it
//! (`probes/virtual_memory/virtual_memory.c`, `probes/thread_stack/thread_stack.c`) and as `scripts/family_run.py`
//! derives it, then compared with the contract's row. And `tests/test_virtual_arena.py`'s values.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use guest_runtime::layout::{MONITOR_SIZE, TRACE_OFF};
use pe_image::PeImage;
use pe_loader::{Loaded, load};
use serde_json::{Map, Value, json};
use virtual_memory::{
    GuestPages, Info, MEM_COMMIT, MEM_DECOMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_EXECUTE_READ, PAGE_NOACCESS,
    PAGE_READONLY, PAGE_READWRITE, VirtualMemory,
};
use w2m_vm::process::{Areas, PARAMETERS_ROOM, Start, TRACE_SIZE, write};
use w2m_vm::{VIRTUAL_SIZE, VIRTUAL_VA};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn binary(sha: &str) -> Vec<u8> {
    fs::read(root().join("store/binaries").join(format!("{sha}.exe"))).unwrap()
}

/// The contract's rows by their key `s`.
fn contract_rows(name: &str) -> BTreeMap<String, Value> {
    let v: Value = serde_json::from_str(&fs::read_to_string(root().join("contracts").join(name)).unwrap()).unwrap();
    v["acceptance"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["s"].as_str().unwrap().to_string(), r.clone()))
        .collect()
}

/// Guest memory for the replay: the loaded image's regions, and the arena's pages, sparse, zero until written.
struct Pages {
    loaded: Loaded,
    arena: BTreeMap<u64, Vec<u8>>,
    zeroed: Vec<(u64, u64)>,
}

impl Pages {
    fn write(&mut self, va: u64, data: &[u8]) {
        if self.loaded.memory.write(va, data).is_ok() {
            return;
        }
        for (i, b) in data.iter().enumerate() {
            let a = va + i as u64;
            self.arena.entry(a & !0xFFF).or_insert_with(|| vec![0; 0x1000])[(a & 0xFFF) as usize] = *b;
        }
    }
}

impl GuestPages for Pages {
    fn zero(&mut self, va: u64, n: u64) {
        self.zeroed.push((va, n));
        self.write(va, &vec![0; n as usize]);
    }

    fn read(&self, va: u64, n: u64) -> Vec<u8> {
        if let Ok(b) = self.loaded.memory.read(va, n) {
            return b;
        }
        (va..va + n)
            .map(|a| self.arena.get(&(a & !0xFFF)).map_or(0, |p| p[(a & 0xFFF) as usize]))
            .collect()
    }
}

const SENTINEL: u64 = 0xDEAD;

fn row(s: &str) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("s".into(), json!(s));
    m
}

/// virtual_memory.c `query` 35-50.
fn query_row(v: &VirtualMemory, p: &Pages, name: &str, at: u64, reservation: u64) -> Value {
    let i: Info = v.query(p, at).unwrap();
    let mut m = row(name);
    m.insert("ret".into(), json!(48));
    m.insert("err".into(), json!(SENTINEL));
    m.insert("state".into(), json!(i.state));
    m.insert("protect".into(), json!(i.protect));
    m.insert("allocation_protect".into(), json!(i.allocation_protect));
    m.insert("type".into(), json!(i.kind));
    m.insert("region_size".into(), json!(i.region_size));
    m.insert("base_is_page_of_address".into(), json!(i.base == at & !0xFFF));
    m.insert(
        "allocation_base_is_reservation".into(),
        json!(i.allocation_base == reservation),
    );
    Value::Object(m)
}

/// virtual_memory.c `alloc` 52-60.
fn alloc_row(name: &str, (p, err): (u64, u32), expect: u64) -> Value {
    let mut m = row(name);
    m.insert("non_null".into(), json!(p != 0));
    m.insert("err".into(), json!(if p != 0 { SENTINEL } else { err as u64 }));
    if p != 0 {
        m.insert("aligned_64k".into(), json!(p & 0xFFFF == 0));
        m.insert("above_4gb".into(), json!(p > 0xFFFF_FFFF));
        m.insert("is_expected".into(), json!(p == expect));
    }
    Value::Object(m)
}

/// virtual_memory.c `protect` 62-68.
fn protect_row(name: &str, (ok, old, err): (u64, u32, u32)) -> Value {
    let mut m = row(name);
    m.insert("ret".into(), json!(ok));
    m.insert("err".into(), json!(if ok != 0 { SENTINEL } else { err as u64 }));
    m.insert("old".into(), json!(old));
    Value::Object(m)
}

/// virtual_memory.c `release` 70-75.
fn release_row(name: &str, (ok, err): (u64, u32)) -> Value {
    let mut m = row(name);
    m.insert("ret".into(), json!(ok));
    m.insert("err".into(), json!(if ok != 0 { SENTINEL } else { err as u64 }));
    Value::Object(m)
}

/// The arena as `VM.__init__` makes it (vm_backend 2498-2505) for a loaded program: the image at its base, the
/// writable pages snapshot.
fn arena_for(loaded: Loaded) -> (VirtualMemory, Pages) {
    let pe: PeImage = loaded.image.clone().unwrap();
    let mut v = VirtualMemory::new(VIRTUAL_VA, VIRTUAL_SIZE, vec![(pe.image_base, pe)]);
    let pages = Pages {
        loaded,
        arena: BTreeMap::new(),
        zeroed: Vec::new(),
    };
    v.snapshot(&pages);
    (v, pages)
}

/// The probe `virtual_memory.c` `start` (77-139), call by call, its rows derived as `family_run.virtual_memory_rows`.
fn virtual_memory_probe(exe: &[u8]) -> Vec<Value> {
    let (mut v, mut p) = arena_for(load(exe, None, "probe.exe").unwrap());
    let mut out = Vec::new();
    let a = v.alloc(&mut p, 0, 0x1FFF_F000, MEM_RESERVE, PAGE_NOACCESS).unwrap();
    out.push(alloc_row("reserve", a, 0));
    let r = a.0;
    out.push(query_row(&v, &p, "query.reserved", r, r));
    let c = v.alloc(&mut p, r, 0x10000, MEM_COMMIT, PAGE_READWRITE).unwrap();
    out.push(alloc_row("commit_inside", c, r));
    let c = c.0;
    let zero = p.read(c, 0x10000).iter().all(|b| *b == 0);
    p.write(c + 5, &[7]);
    let mut m = row("commit_inside.zeroed");
    m.insert("value".into(), json!(zero));
    out.push(Value::Object(m));
    out.push(query_row(&v, &p, "query.committed", c + 0x1234, r));
    out.push(query_row(&v, &p, "query.after_committed", c + 0x10000, r));
    let c2 = v.alloc(&mut p, r + 0x20123, 0x10, MEM_COMMIT, PAGE_READWRITE).unwrap();
    out.push(alloc_row("commit_unaligned_inside", c2, r + 0x20000));
    out.push(query_row(&v, &p, "query.unaligned_commit", r + 0x20000, r));
    out.push(protect_row(
        "protect.readonly",
        v.protect(&p, c, 0x1000, PAGE_READONLY).unwrap(),
    ));
    out.push(query_row(&v, &p, "query.readonly", c, r));
    out.push(protect_row(
        "protect.back",
        v.protect(&p, c, 0x1000, PAGE_READWRITE).unwrap(),
    ));
    out.push(protect_row(
        "protect.two_protections",
        v.protect(&p, c, 0x2000, PAGE_EXECUTE_READ).unwrap(),
    ));
    out.push(query_row(&v, &p, "query.after_two", c, r));
    out.push(protect_row(
        "protect.reserved_page",
        v.protect(&p, r + 0x100000, 0x1000, PAGE_READWRITE).unwrap(),
    ));
    let both = MEM_RESERVE | MEM_COMMIT;
    out.push(alloc_row(
        "alloc.size_zero",
        v.alloc(&mut p, 0, 0, both, PAGE_READWRITE).unwrap(),
        0,
    ));
    out.push(alloc_row(
        "alloc.reserve_taken",
        v.alloc(&mut p, r, 0x10000, MEM_RESERVE, PAGE_READWRITE).unwrap(),
        0,
    ));
    out.push(alloc_row(
        "alloc.bad_protect",
        v.alloc(&mut p, 0, 0x1000, both, 0x12345).unwrap(),
        0,
    ));
    let little = v.alloc(&mut p, 0, 100, both, PAGE_READWRITE).unwrap();
    out.push(alloc_row("alloc.little_block", little, 0));
    let little = little.0;
    out.push(query_row(&v, &p, "query.little_block", little, little));
    let guarded = v.alloc(&mut p, 0, 0x3000, MEM_COMMIT, PAGE_NOACCESS).unwrap();
    out.push(alloc_row("alloc.commit_null", guarded, 0));
    let g = guarded.0;
    if g != 0 {
        out.push(query_row(&v, &p, "query.commit_null", g, g));
        out.push(protect_row(
            "protect.commit_null_middle",
            v.protect(&p, g + 0x1000, 0x1000, PAGE_READWRITE).unwrap(),
        ));
        out.push(query_row(&v, &p, "query.commit_null_middle", g + 0x1000, g));
        out.push(query_row(&v, &p, "query.commit_null_last", g + 0x2000, g));
        out.push(release_row(
            "free.commit_null",
            v.free(&mut p, g, 0, MEM_RELEASE).unwrap(),
        ));
    }
    out.push(release_row(
        "free.decommit",
        v.free(&mut p, c, 0x10000, MEM_DECOMMIT).unwrap(),
    ));
    out.push(query_row(&v, &p, "query.decommitted", c, r));
    out.push(release_row(
        "free.release_with_size",
        v.free(&mut p, r, 0x10000, MEM_RELEASE).unwrap(),
    ));
    out.push(release_row(
        "free.release_inside",
        v.free(&mut p, r + 0x10000, 0, MEM_RELEASE).unwrap(),
    ));
    out.push(release_row("free.release", v.free(&mut p, r, 0, MEM_RELEASE).unwrap()));
    // family_run.virtual_memory_rows (231-239): the released region's size kept as covering the reservation
    let mut released = query_row(&v, &p, "query.released", r, 0);
    let m = released.as_object_mut().unwrap();
    let size = m.remove("region_size").unwrap().as_u64().unwrap();
    m.insert("region_covers_reservation".into(), json!(size >= 0x1FFF_F000));
    out.push(released);
    out.push(release_row(
        "free.release_again",
        v.free(&mut p, r, 0, MEM_RELEASE).unwrap(),
    ));
    out.push(release_row(
        "free.little_block",
        v.free(&mut p, little, 0, MEM_RELEASE).unwrap(),
    ));

    // the image: its headers, its first six sections, and .rdata's first page made read-write and back
    let pe = v.images[0].1.clone();
    let base = v.images[0].0;
    out.push(query_row(&v, &p, "image.headers", base, base));
    for s in pe.sections.iter().take(6) {
        let name = format!("image.section.{}", s.name.replace('.', "_"));
        out.push(query_row(&v, &p, &name, base + s.virtual_address as u64, base));
    }
    if let Some(rdata) = pe.sections.iter().rev().find(|s| s.name.starts_with(".rd")) {
        let at = base + rdata.virtual_address as u64;
        out.push(protect_row(
            "image.protect_rdata_rw",
            v.protect(&p, at, 0x1000, PAGE_READWRITE).unwrap(),
        ));
        out.push(query_row(&v, &p, "image.rdata_after", at, base));
        out.push(protect_row(
            "image.protect_rdata_back",
            v.protect(&p, at, 0x1000, PAGE_READONLY).unwrap(),
        ));
    }
    out
}

/// The rows that depend on which data pages the running probe wrote: judged at the probe's run in the VM.
const NEEDS_THE_RUN: [&str; 2] = ["image.section._data", "image.rdata_after"];

#[test]
fn the_virtual_memory_probe_s_rows_are_windows_rows() {
    let want = contract_rows("windows.memory.virtual/0.2.0.json");
    // the Intel run's image (38172736319) and the AMD run's (38172734444)
    for sha in [
        "8385bc74e8db79d6ef5c62d4ebd0dc161e7985f85f4d763dc73dd42bed2a137d",
        "af77fd8015671e678be7df19b74160d4cc93ce8691c64127412da3572cf73955",
    ] {
        let got = virtual_memory_probe(&binary(sha));
        let names: Vec<&str> = got.iter().map(|r| r["s"].as_str().unwrap()).collect();
        assert_eq!(
            names.iter().copied().collect::<BTreeSet<_>>(),
            want.keys().map(String::as_str).collect::<BTreeSet<_>>(),
            "{sha}: the rows printed"
        );
        let mut judged = 0;
        for r in &got {
            let s = r["s"].as_str().unwrap();
            if NEEDS_THE_RUN.contains(&s) {
                continue;
            }
            assert_eq!(r, &want[s], "{sha}: row {s}");
            judged += 1;
        }
        assert_eq!(judged, 38);
    }
}

/// thread_stack.c `where` 37-45: an address named by the TEB's fields, or its distance in pages from StackLimit.
fn where_(v: u64, (dealloc, limit, base): (u64, u64, u64)) -> String {
    if v == base {
        "base".into()
    } else if v == limit {
        "limit".into()
    } else if v == dealloc {
        "dealloc".into()
    } else if v < limit {
        format!("limit-{}p", (limit - v) / 4096)
    } else {
        format!("limit+{}p", (v - limit) / 4096)
    }
}

/// The six query rows of `thread_stack.c` (66-71) with the stack the Rust process area writes for the probe
/// (`process::write`: DeallocationStack, StackLimit, StackBase), derived as `family_run.thread_stack_rows`: no
/// `base_address` for query.sp and query.base_minus_one.
#[test]
fn the_stack_queries_are_windows_rows() {
    let want = contract_rows("windows.memory.thread_stack/0.1.0.json");
    let modules: BTreeSet<String> = {
        let v: Value =
            serde_json::from_str(&fs::read_to_string(root().join("specs/windows_exports.json")).unwrap()).unwrap();
        v["modules"].as_object().unwrap().keys().cloned().collect()
    };
    // the Intel run's image (38114446614) and the AMD run's (38114444717)
    for sha in [
        "d39ae9f9aa57c21fdc3a440f5e0acc4df8b073df4def00ef03b21a5ad047e7dd",
        "09f0d8199eb9748f2ebd1855df859eeed84a7028982ffc6fe7091e7ccfb44091",
    ] {
        let loaded = load(&binary(sha), None, "probe.exe").unwrap();
        let (mut monitor, mut process, mut heap) = (
            vec![0u8; MONITOR_SIZE as usize],
            vec![0u8; (TRACE_OFF + TRACE_SIZE) as usize],
            vec![0u8; PARAMETERS_ROOM as usize],
        );
        let area = write(
            &mut Areas {
                monitor: &mut monitor,
                process: &mut process,
                heap: &mut heap,
            },
            &loaded,
            &Start::console("probe.exe"),
            &modules,
        )
        .unwrap();
        let stack = area.stack;
        let (dealloc, limit, base) = stack;
        let (mut v, p) = arena_for(loaded);
        v.stack = Some(stack);
        // the probe's stack pointer: the address of a local in `start`, in the committed stack; here the entry's
        // first slot below StackBase, which the row names the same way (its base_address is dropped)
        let sp = base - 8;
        let queries = [
            ("query.sp", sp),
            ("query.below_limit", limit - 1),
            ("query.limit", limit),
            ("query.below_guard", limit - 2 * 4096),
            ("query.dealloc", dealloc),
            ("query.base_minus_one", base - 1),
        ];
        for (name, at) in queries {
            let i = v.query(&p, at).unwrap();
            let mut m = row(name);
            m.insert("ret_is_size".into(), json!(1));
            m.insert("err".into(), json!(SENTINEL));
            if name != "query.sp" && name != "query.base_minus_one" {
                m.insert("base_address".into(), json!(where_(i.base, stack)));
            }
            m.insert("allocation_base".into(), json!(where_(i.allocation_base, stack)));
            m.insert("end".into(), json!(where_(i.base + i.region_size, stack)));
            m.insert("allocation_protect".into(), json!(i.allocation_protect));
            m.insert("state".into(), json!(i.state));
            m.insert("protect".into(), json!(i.protect));
            m.insert("type".into(), json!(i.kind));
            assert_eq!(Value::Object(m), want[name], "{sha}: row {name}");
        }
    }
}

struct Untouched;

impl GuestPages for Untouched {
    fn zero(&mut self, _: u64, _: u64) {}
    fn read(&self, _: u64, n: u64) -> Vec<u8> {
        vec![0; n as usize]
    }
}

/// `tests/test_virtual_arena.py` 13-22 (Sprint 047): UnityPlayer.dll reserves 0x1FFFF000 bytes at a time, and its
/// ninth such reservation filled the 4 GB arena of Sprint 046; the 512 GB arena holds 1,024.
#[test]
fn unityplayer_s_reservations_fit_many_times_over() {
    let mut p = Pages {
        loaded: Loaded::bare(pe_loader::Memory::default(), 0, 0),
        arena: BTreeMap::new(),
        zeroed: Vec::new(),
    };
    let mut v = VirtualMemory::new(VIRTUAL_VA, VIRTUAL_SIZE, Vec::new());
    let bases: Vec<(u64, u32)> = (0..64)
        .map(|_| v.alloc(&mut p, 0, 0x1FFF_F000, MEM_RESERVE, PAGE_NOACCESS).unwrap())
        .collect();
    assert!(bases.iter().all(|(_, e)| *e == 0));
    assert_eq!(
        bases.iter().map(|(b, _)| *b).collect::<Vec<_>>(),
        (0..64).map(|i| VIRTUAL_VA + i * 0x2000_0000).collect::<Vec<_>>()
    );
    let last = bases[63].0;
    assert_eq!(
        v.alloc(&mut p, last, 0x2000, MEM_COMMIT, PAGE_READWRITE).unwrap(),
        (last, 0)
    );
    // a commit zeroes only its own pages
    assert_eq!(p.zeroed, vec![(last, 0x1000), (last + 0x1000, 0x1000)]);
    assert_eq!(VIRTUAL_SIZE / 0x2000_0000, 1024);
}

/// `tests/test_virtual_arena.py` 25-34 (Sprint 065): steam_api64.dll was placed at 0x1222ff0000 and mono's hinted
/// reservation there was granted, so its JIT code overwrote the image; Windows answers ERROR_INVALID_ADDRESS (487)
/// for an address in use.
#[test]
fn a_reservation_at_an_address_a_loaded_dll_holds_is_refused() {
    let mut v = VirtualMemory::new(VIRTUAL_VA, VIRTUAL_SIZE, Vec::new());
    let pe = PeImage {
        image_base: VIRTUAL_VA,
        size_of_image: 0x5B000,
        ..load_any_image()
    };
    let base = v.place_image(pe).unwrap();
    let both = MEM_COMMIT | MEM_RESERVE;
    let mut u = Untouched;
    assert_eq!(v.alloc(&mut u, base, 0x10000, both, PAGE_READWRITE).unwrap(), (0, 487));
    assert_eq!(
        v.alloc(&mut u, base + 0x50000, 0x10000, both, PAGE_READWRITE).unwrap(),
        (0, 487)
    );
    assert_eq!(
        v.alloc(&mut u, base + 0x60000, 0x10000, both, PAGE_READWRITE).unwrap(),
        (base + 0x60000, 0)
    );
}

/// An image with no sections, for a placement test that reads only its base and size (the Python's SimpleNamespace).
fn load_any_image() -> PeImage {
    let mut pe = pe_image::parse(
        &binary("8385bc74e8db79d6ef5c62d4ebd0dc161e7985f85f4d763dc73dd42bed2a137d"),
        false,
    )
    .unwrap();
    pe.sections.clear();
    pe
}
