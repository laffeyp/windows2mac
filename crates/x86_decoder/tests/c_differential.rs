// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 143's judge: the Rust decoder against `tools/vm_compiler/x86_decode.c`, the decoder the VM runs, built for
//! the host by clang and called through `dlopen`, over every instruction of `store/stencil/record.txt.gz` and every
//! instruction reached linearly from the `.pdata` function starts of the executables under `store/binaries/`. Length,
//! iclass, the spec row and the fields must be equal, and so must the refusals. The same library's `vmc_compile`
//! counts the record's calls the C emitter gives words for, the design's revisit figure. Runs where clang is found.

use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use x86_decoder::{DecodeError, Encoding, decode};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

unsafe extern "C" {
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}

const RTLD_NOW: c_int = 2;

/// x86_decode.h `struct x86_fields`.
#[repr(C)]
#[derive(Clone, Copy)]
struct CFields {
    prefixes: [u8; 15],
    nprefixes: u8,
    rex: u8,
    enc: u8,
    opcode: u8,
    modrm: i16,
    sib: i16,
    disp: i64,
    disp_size: u8,
    imm: [u8; 8],
    imm_size: u8,
    opsize: u8,
    asize: u8,
    ops: u16,
    vvvv: i8,
    vl: u8,
}

/// x86_decode.h `struct x86_decoded`.
#[repr(C)]
#[derive(Clone, Copy)]
struct CDecoded {
    length: u8,
    iclass: u16,
    row: u16,
    f: CFields,
}

/// vm_compiler.h `struct vmc_flags`.
#[repr(C)]
struct Flags {
    nzcv_from: c_int,
    live_after: c_int,
    nzcv_out: c_int,
}

type Decode = unsafe extern "C" fn(*const u8, c_int, *mut CDecoded) -> c_int;
type Compile =
    unsafe extern "C" fn(*const u8, c_int, u64, u64, u64, *const Flags, *const u64, *mut u32, c_int) -> c_int;

struct C {
    decode: Decode,
    compile: Compile,
    iclass_name: *const *const c_char,
    template_name: *const *const c_char,
}

/// The C compiler built for the host, or `None` when clang is not found.
fn load() -> Option<C> {
    let clang = Command::new("xcrun").args(["--find", "clang"]).output().ok()?;
    if !clang.status.success() {
        return None;
    }
    // through xcrun, which gives the linker the SDK (clang run by its path alone finds no libSystem)
    let dir = root().join("target/x86_decoder_differential");
    fs::create_dir_all(&dir).unwrap();
    let lib = dir.join(format!("libvmc-{}.dylib", std::process::id()));
    let vmc = root().join("tools/vm_compiler");
    let built = Command::new("xcrun")
        .args(["clang", "-shared", "-O2", "-DX86_DECODE_NAMES", "-I"])
        .arg(&vmc)
        .args(["x86_decode.c", "vm_compiler.c", "a64_encoder.c"].map(|f| vmc.join(f)))
        .arg("-o")
        .arg(&lib)
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let path = CString::new(lib.to_str().unwrap()).unwrap();
    unsafe {
        let h = dlopen(path.as_ptr(), RTLD_NOW);
        assert!(!h.is_null(), "{:?}", CStr::from_ptr(dlerror()));
        let sym = |n: &str| {
            let p = dlsym(h, CString::new(n).unwrap().as_ptr());
            assert!(!p.is_null(), "{n}");
            p
        };
        Some(C {
            decode: std::mem::transmute::<*mut c_void, Decode>(sym("x86_decode")),
            compile: std::mem::transmute::<*mut c_void, Compile>(sym("vmc_compile")),
            iclass_name: sym("x86_iclass_name").cast(),
            template_name: sym("x86_template_name").cast(),
        })
    }
}

impl C {
    fn name(table: *const *const c_char, i: u16) -> String {
        unsafe { CStr::from_ptr(*table.add(usize::from(i))).to_str().unwrap().to_string() }
    }

    /// The C's decoding as the Rust decoder's terms: Ok((length, iclass, row index, fields)) or the error's kind.
    fn decode(&self, b: &[u8]) -> Result<(usize, String, usize, String), &'static str> {
        let mut out = std::mem::MaybeUninit::<CDecoded>::zeroed();
        let status = unsafe { (self.decode)(b.as_ptr(), b.len() as c_int, out.as_mut_ptr()) };
        let d = unsafe { out.assume_init() };
        match status {
            0 => {
                let f = d.f;
                let ops = if f.ops == 0xFFFF {
                    "None".to_string()
                } else {
                    C::name(self.template_name, f.ops)
                };
                let fields = format!(
                    "pfx {:02x?} rex {:02x} enc {} op {:02x} modrm {:?} sib {:?} disp {} {} imm {:02x?} opsize {} \
                     asize {} ops {} vvvv {:?} vl {}",
                    &f.prefixes[..usize::from(f.nprefixes)],
                    f.rex,
                    ["legacy", "vex", "evex", "xop"][usize::from(f.enc)],
                    f.opcode,
                    (f.modrm >= 0).then_some(f.modrm as u8),
                    (f.sib >= 0).then_some(f.sib as u8),
                    f.disp,
                    f.disp_size,
                    &f.imm[..usize::from(f.imm_size)],
                    f.opsize,
                    f.asize,
                    ops,
                    (f.vvvv >= 0).then_some(f.vvvv as u8),
                    f.vl
                );
                Ok((
                    usize::from(d.length),
                    C::name(self.iclass_name, d.iclass),
                    usize::from(d.row),
                    fields,
                ))
            }
            1 => Err("invalid"),
            2 => Err("truncated"),
            3 => Err("no entry"),
            _ => Err("other"),
        }
    }
}

fn rust(b: &[u8]) -> Result<(usize, String, usize, String), &'static str> {
    match decode(b, None) {
        Ok(d) => {
            let f = d.fields;
            let enc = match f.enc {
                Encoding::Legacy => "legacy",
                Encoding::Vex => "vex",
                Encoding::Evex => "evex",
                Encoding::Xop => "xop",
            };
            let fields = format!(
                "pfx {:02x?} rex {:02x} enc {} op {:02x} modrm {:?} sib {:?} disp {} {} imm {:02x?} opsize {} asize \
                 {} ops {} vvvv {:?} vl {}",
                f.prefixes,
                f.rex,
                enc,
                f.opcode,
                f.modrm,
                f.sib,
                f.disp,
                f.disp_size,
                f.imm,
                f.opsize,
                f.asize,
                f.ops.unwrap_or("None"),
                f.vvvv,
                f.vl
            );
            Ok((d.length, d.iclass, d.index, fields))
        }
        Err(DecodeError::InvalidEncoding(_)) => Err("invalid"),
        Err(DecodeError::Truncated(_)) => Err("truncated"),
        Err(DecodeError::NoEntry(_)) => Err("no entry"),
    }
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// One record call: its address, bytes and the three flag arguments.
type Call = (u64, Vec<u8>, [c_int; 3]);

/// The record: its header (the exit address and the symbols) and its calls.
fn record() -> (serde_json::Value, Vec<Call>) {
    let gz = fs::read(root().join("store/stencil/record.txt.gz")).unwrap();
    let mut text = String::new();
    let mut child = Command::new("gzip")
        .args(["-dc"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || {
        use std::io::Write;
        stdin.write_all(&gz).unwrap();
    });
    child.stdout.take().unwrap().read_to_string(&mut text).unwrap();
    writer.join().unwrap();
    assert!(child.wait().unwrap().success());
    let mut lines = text.lines();
    let head: serde_json::Value = serde_json::from_str(lines.next().unwrap()).unwrap();
    let calls = lines
        .map(|l| {
            let p: Vec<&str> = l.split(' ').collect();
            let n = |i: usize| p[i].parse::<c_int>().unwrap();
            (u64::from_str_radix(p[0], 16).unwrap(), unhex(p[1]), [n(2), n(3), n(4)])
        })
        .collect();
    (head, calls)
}

#[test]
fn the_rust_decoder_equals_the_vms_c_decoder() {
    let Some(c) = load() else {
        eprintln!("clang not found: the differential is not run here");
        return;
    };
    let mut differ: Vec<String> = Vec::new();
    let mut compare = |what: &str, b: &[u8]| {
        let (ours, theirs) = (rust(b), c.decode(b));
        if ours != theirs && differ.len() < 10 {
            differ.push(format!("{what} {:02x?}:\n  rust {ours:?}\n  c    {theirs:?}", b));
        }
        ours.is_ok()
    };

    // the record's instructions
    let (head, calls) = record();
    let mut decoded = 0;
    for (address, bytes, _) in &calls {
        decoded += usize::from(compare(&format!("record 0x{address:x}"), bytes));
    }
    println!(
        "record: {} instructions, {decoded} decoded, compared with the C",
        calls.len()
    );
    // the record's iclasses, for the count of rule files the C emitter's calls reach
    let iclasses: std::collections::BTreeSet<String> = calls
        .iter()
        .filter_map(|(_, b, _)| decode(b, None).ok().map(|d| d.iclass))
        .collect();
    let list = root().join("target/x86_decoder_differential/record_iclasses.txt");
    fs::write(&list, iclasses.iter().cloned().collect::<Vec<_>>().join("\n")).unwrap();
    println!(
        "record: {} distinct iclasses, listed in {}",
        iclasses.len(),
        list.display()
    );

    // the .pdata sweep of store/binaries
    let mut swept = 0usize;
    let mut paths: Vec<PathBuf> = fs::read_dir(root().join("store/binaries"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "exe"))
        .collect();
    paths.sort();
    for path in &paths {
        let data = fs::read(path).unwrap();
        let img = pe_image::parse(&data, false).unwrap();
        for (start, end) in img.function_starts.iter().zip(&img.function_ends) {
            let mut rva = u64::from(*start);
            while rva < u64::from(*end) {
                let Ok(b) = img.read_rva(&data, rva, 15) else { break };
                swept += 1;
                let name = path.file_name().unwrap().to_string_lossy();
                if !compare(&format!("{} rva 0x{rva:x}", &name[..12]), &b) {
                    break;
                }
                rva += rust(&b).unwrap().0 as u64;
            }
        }
    }
    println!(
        "sweep: {swept} instructions from {} executables' .pdata, compared with the C",
        paths.len()
    );
    assert!(differ.is_empty(), "the decoders differ:\n{}", differ.join("\n"));

    // the C emitter's coverage of the record (the design's revisit figure)
    let table = fs::read_to_string(root().join("tools/vm_compiler/stencil_table.h")).unwrap();
    let body = &table[table.find("vmc_symbol_name[").unwrap()..];
    let names: Vec<&str> = body[..body.find("};").unwrap()].split('"').skip(1).step_by(2).collect();
    // the record predates the x87 stencils, so five of the table's symbols are not in its header; whether the C emits
    // does not depend on an address a word holds, so a missing one stands at 0
    let missing: Vec<&str> = names
        .iter()
        .copied()
        .filter(|n| head["symbols"][*n].is_null())
        .collect();
    let symbols: Vec<u64> = names
        .iter()
        .map(|n| head["symbols"][*n].as_u64().unwrap_or(0))
        .collect();
    println!("symbols not in the record's header, at 0: {missing:?}");
    let exit_va = head["exit_va"].as_u64().unwrap();
    let mut words = [0u32; 512];
    let mut by_status: BTreeMap<c_int, usize> = BTreeMap::new();
    for (address, bytes, flags) in &calls {
        let f = Flags {
            nzcv_from: flags[0],
            live_after: flags[1],
            nzcv_out: flags[2],
        };
        let r = unsafe {
            (c.compile)(
                bytes.as_ptr(),
                bytes.len() as c_int,
                *address,
                0x7F00_0006_0000,
                exit_va,
                &f,
                symbols.as_ptr(),
                words.as_mut_ptr(),
                words.len() as c_int,
            )
        };
        // a failure is a negative vmc_status; 0 is an instruction that emits no words
        *by_status.entry(if r >= 0 { 1 } else { r }).or_default() += 1;
    }
    let emitted = by_status.get(&1).copied().unwrap_or(0);
    println!(
        "the C emitter gives words for {emitted} of {} record calls; refusals by status: {:?}",
        calls.len(),
        by_status.iter().filter(|(k, _)| **k != 1).collect::<Vec<_>>()
    );
}
