// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 151's first judge: for every call of the stencil record (`store/stencil/record.txt.gz`, the batch run's
//! 1,186,511 compile calls) whose instruction the Rust rules lift, the Rust compiler's words at the record's address,
//! exit and flag arguments equal the words of the VM's C emitter (`tools/vm_compiler`, built for the host and called
//! through `dlopen`), where the C gives words. The C stencils were derived from the frozen Python lowering, so equality
//! is the expectation. It needs no VM and no untracked input, so it runs everywhere, CI's Linux included; a missing C
//! compiler fails it (Sprint 154: it had skipped on CI, reviewer-2's review of Sprints 151-152).

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use a64_encoder::assembler::Assembler;
use guest_runtime::layout::{PROCESS_VA, STATE_OFF};
use translator::lift::lift;
use translator::lower::{NZCV_KINDS, compile_insn};

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

/// vm_compiler.h `struct vmc_flags`.
#[repr(C)]
struct Flags {
    nzcv_from: c_int,
    live_after: c_int,
    nzcv_out: c_int,
}

/// vm_compiler.h `vmc_compile`: decode, emit, words or a negative status.
type Compile =
    unsafe extern "C" fn(*const u8, c_int, u64, u64, u64, *const Flags, *const u64, *mut u32, c_int) -> c_int;

/// The C emitter built for the host and loaded (Sprint 154: on CI's Linux too, where it once skipped). The compiler is
/// `$CC` when set, else `xcrun clang` on macOS (which gives the linker the SDK) and `cc` elsewhere; a compiler that is
/// missing or fails fails the test.
fn load() -> Compile {
    let mut cmd: Vec<String> = match std::env::var("CC") {
        Ok(cc) => cc.split_whitespace().map(str::to_string).collect(),
        Err(_) if cfg!(target_os = "macos") => vec!["xcrun".into(), "clang".into()],
        Err(_) => vec!["cc".into()],
    };
    let dir = root().join("target/translator_c_words");
    fs::create_dir_all(&dir).unwrap();
    let ext = if cfg!(target_os = "macos") { "dylib" } else { "so" };
    let lib = dir.join(format!("libvmc-{}.{ext}", std::process::id()));
    let vmc = root().join("tools/vm_compiler");
    let program = cmd.remove(0);
    let built = Command::new(&program)
        .args(&cmd)
        .args(["-shared", "-fPIC", "-O2", "-I"])
        .arg(&vmc)
        .args(["x86_decode.c", "vm_compiler.c", "a64_encoder.c"].map(|f| vmc.join(f)))
        .arg("-o")
        .arg(&lib)
        .output()
        .unwrap_or_else(|e| panic!("the C compiler `{program}` did not run: {e}"));
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let path = CString::new(lib.to_str().unwrap()).unwrap();
    unsafe {
        let h = dlopen(path.as_ptr(), 2);
        assert!(!h.is_null(), "{:?}", CStr::from_ptr(dlerror()));
        let p = dlsym(h, c"vmc_compile".as_ptr());
        assert!(!p.is_null());
        std::mem::transmute::<*mut c_void, Compile>(p)
    }
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// One record call: its address, bytes and the three flag arguments (nzcv_from, live_after, nzcv_out).
type Call = (u64, Vec<u8>, [c_int; 3]);

/// The record's header line and its calls.
fn record() -> (serde_json::Value, Vec<Call>) {
    let gz = fs::read(root().join("store/stencil/record.txt.gz")).unwrap();
    let mut child = Command::new("gzip")
        .arg("-dc")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || {
        use std::io::Write;
        stdin.write_all(&gz).unwrap();
    });
    let mut text = String::new();
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

/// The block base the words are made for, as the decoder differential's coverage count uses.
const BASE: u64 = 0x7F00_0006_0000;

#[test]
fn group_1_s_words_equal_the_c_emitter_s() {
    let c_compile = load();
    let (head, calls) = record();
    let table = fs::read_to_string(root().join("tools/vm_compiler/stencil_table.h")).unwrap();
    let body = &table[table.find("vmc_symbol_name[").unwrap()..];
    let names: Vec<&str> = body[..body.find("};").unwrap()].split('"').skip(1).step_by(2).collect();
    // a name the record's header lacks (x87_fast.c's five) gets a distinct placeholder address, the same for both
    // compilers: at address 0 `mov_imm` is one word, not three, and the C stencil's branch past the call, fixed for
    // three, lands two words past the model step (Sprint 177)
    let placeholder = |i: usize| 0x7F01_0001_1000 + 0x100 * i as u64;
    let symbols: Vec<u64> = names
        .iter()
        .enumerate()
        .map(|(i, n)| head["symbols"][*n].as_u64().unwrap_or(placeholder(i)))
        .collect();
    let exit_va = head["exit_va"].as_u64().unwrap();
    let state_va = PROCESS_VA + STATE_OFF;
    // the runtime's functions the words call (CPUID's, PCMPISTRI's, the approximations', x87_fast.c's): the addresses the
    // C emitter is given in its symbol table, as `symbols` above gives them (Sprint 177: the header has no `w2m_x87_*`,
    // and the C emits `_x87_fast` with the table's names)
    let runtime = translator::lower::RuntimeCalls::from_symbols(|s| match names.iter().position(|n| *n == s) {
        Some(i) => Some(symbols[i]),
        None => head["symbols"][s].as_u64(),
    });

    let mut lifted = 0usize;
    let mut compared: BTreeMap<String, usize> = BTreeMap::new();
    let mut c_refused: BTreeMap<String, usize> = BTreeMap::new();
    let mut rust_refused: BTreeMap<String, usize> = BTreeMap::new();
    let mut lowering_refused: BTreeMap<String, usize> = BTreeMap::new();
    let mut differ: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let mut words = [0u32; 512];
    // the calls no Rust rule lifts yet, by decoded class: the later groups' reach (Sprint 160)
    let mut not_lifted: BTreeMap<String, usize> = BTreeMap::new();
    for (address, bytes, flags) in &calls {
        let Ok(ins) = lift(bytes, *address) else {
            let class = x86_decoder::decode(bytes, None).map_or_else(|_| "undecoded".to_string(), |d| d.iclass);
            *not_lifted.entry(class).or_default() += 1;
            continue; // a class of a later group
        };
        lifted += 1;
        let f = Flags {
            nzcv_from: flags[0],
            live_after: flags[1],
            nzcv_out: flags[2],
        };
        let n = unsafe {
            c_compile(
                bytes.as_ptr(),
                bytes.len() as c_int,
                *address,
                BASE,
                exit_va,
                &f,
                symbols.as_ptr(),
                words.as_mut_ptr(),
                words.len() as c_int,
            )
        };
        if n < 0 {
            *c_refused.entry(format!("{} status {n}", ins.iclass)).or_default() += 1;
            continue;
        }
        let nzcv_from = (flags[0] > 0).then(|| NZCV_KINDS[flags[0] as usize - 1]);
        let mut a = Assembler::new(BASE);
        if let Err(e) = compile_insn(
            &mut a,
            &ins,
            exit_va,
            state_va,
            &runtime,
            nzcv_from,
            flags[1] as u32,
            flags[2] != 0,
        ) {
            // a lowering the Rust refuses by name, as a later group (Sprint 177: the MMX ops the one lifter emits, which
            // the model steps), is held to the census; any other refusal fails
            if e.to_string().contains("is a later group") {
                *lowering_refused.entry(ins.iclass.clone()).or_default() += 1;
            } else {
                *rust_refused.entry(format!("{}: {e}", ins.iclass)).or_default() += 1;
            }
            continue;
        }
        let rust = a.finish();
        let c = &words[..n as usize];
        *compared.entry(ins.iclass.clone()).or_default() += 1;
        if rust != c {
            let shown = |w: &[u32]| w.iter().map(|x| format!("{x:08x}")).collect::<Vec<_>>().join(" ");
            let key = format!("{} flags {:?}", ins.iclass, flags);
            let e = differ.entry(key).or_insert((
                0,
                format!(
                    "0x{address:x} {}: rust [{}] c [{}]",
                    bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
                    shown(&rust),
                    shown(c)
                ),
            ));
            e.0 += 1;
        }
    }
    let total: usize = compared.values().sum();
    println!(
        "record: {} calls; lifted by the Rust rules {lifted}; compared with the C {total} in {} classes",
        calls.len(),
        compared.len()
    );
    println!("compared by class: {compared:?}");
    let mut by_count: Vec<_> = not_lifted.iter().collect();
    by_count.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    println!("not lifted by the Rust rules, by class: {by_count:?}");
    println!("C refusals of lifted calls: {c_refused:?}");
    println!("Rust refusals: {rust_refused:?}");
    println!("lowering refused by name, by class: {lowering_refused:?}");
    for (k, (n, example)) in differ.iter().take(30) {
        println!("  {n:7} {k}: {example}");
    }
    // the classes the Rust rules do not lift, held to the committed census (Sprint 176; reviewer-2's High 1 of
    // 2026-10-28): a class that stops lifting, or comes to lift, changes `store/census/c-words-unlifted-2026-10-28.tsv`
    // in the same commit; from Sprint 177 each row names its stage, "not lifted" or "lowering refused" (lifted, its
    // lowering refused by name, the model stepping it)
    let held: BTreeMap<String, usize> = fs::read_to_string(root().join("store/census/c-words-unlifted-2026-10-28.tsv"))
        .expect("store/census/c-words-unlifted-2026-10-28.tsv")
        .lines()
        .skip(1)
        .map(|l| {
            let p: Vec<&str> = l.split('\t').collect();
            assert_eq!(p.len(), 3, "class<TAB>calls<TAB>stage: {l}");
            (format!("{} ({})", p[0], p[2]), p[1].parse().expect("a call count"))
        })
        .collect();
    let now: BTreeMap<String, usize> = not_lifted
        .iter()
        .map(|(c, n)| (format!("{c} (not lifted)"), *n))
        .chain(
            lowering_refused
                .iter()
                .map(|(c, n)| (format!("{c} (lowering refused)"), *n)),
        )
        .collect();
    let moved: Vec<String> = held
        .keys()
        .chain(now.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|c| held.get(*c) != now.get(*c))
        .map(|c| format!("{c}: held {:?}, now {:?}", held.get(c), now.get(c)))
        .collect();
    assert!(
        moved.is_empty(),
        "the unlifted and unlowered classes moved from the census: {}",
        moved.join("; ")
    );
    assert!(rust_refused.is_empty(), "the Rust compiler refused lifted calls");
    assert!(differ.is_empty(), "{} difference classes", differ.len());
    assert!(total > 0, "nothing compared");
}
