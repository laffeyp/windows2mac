// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The C guest runtime's image, built from Rust (Sprint 133) as `src/w2m/guest_runtime.py` built it: every file of
//! `tools/guest_runtime/` and the compiler's three (`tools/vm_compiler`) compiled by Xcode's clang for AArch64 and
//! linked by Xcode's `ld -preload` at a fixed address, an MH_PRELOAD Mach-O whose segments are copied into the VM as
//! they are. The inputs, the headers the build writes and the cache key are the Python's, so the image for the same
//! sources is the same file, byte for byte; Rust builds under `build/guest_runtime_rust/<key>/` and never writes the
//! Python's `build/guest_runtime/`.

pub mod headers;
pub mod layout;
pub mod services;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `guest_runtime.py` `CFLAGS`, in its order.
pub const CFLAGS: [&str; 12] = [
    "--target=arm64-apple-macos14",
    "-O2",
    "-std=c11",
    "-Wall",
    "-Werror",
    "-ffreestanding",
    "-fno-builtin",
    "-fno-stack-protector",
    "-fno-exceptions",
    "-fno-unwind-tables",
    "-fno-asynchronous-unwind-tables",
    "-mgeneral-regs-only",
];
/// The compiler's files that join the image (`guest_runtime.py` `COMPILER_FILES`).
pub const COMPILER_FILES: [&str; 3] = ["vm_compiler.c", "x86_decode.c", "a64_encoder.c"];

/// The runtime's sources: each `tools/guest_runtime/*.c` and the compiler's three, by file name.
pub fn texts(root: &Path) -> Result<BTreeMap<String, String>, String> {
    let source = root.join("tools/guest_runtime");
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(&source).map_err(|e| format!("{}: {e}", source.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|x| x == "c") {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            out.insert(
                name,
                fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?,
            );
        }
    }
    for name in COMPILER_FILES {
        let path = root.join("tools/vm_compiler").join(name);
        out.insert(
            name.to_string(),
            fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?,
        );
    }
    Ok(out)
}

/// The headers the build writes into its work directory, by name, in the order the key takes them.
pub fn generated_headers(root: &Path) -> Result<Vec<(&'static str, String)>, String> {
    Ok(vec![
        ("cpuid_table.h", headers::cpuid(root)?),
        ("code_page_tables.h", headers::code_pages(root)?),
        ("system_messages.h", headers::system_messages(root)?),
        ("approximation_tables.h", headers::approximations(root)?),
    ])
}

/// `-D<name>=0x<value>ULL` for each define, in name order.
pub fn define_flags(defines: &[(String, u64)]) -> Vec<String> {
    let mut d: Vec<&(String, u64)> = defines.iter().collect();
    d.sort();
    d.iter().map(|(k, v)| format!("-D{k}=0x{v:x}ULL")).collect()
}

/// The cache key: sha256 over every input, joined by NUL as `guest_runtime._compile` joins them (each source as
/// `name NUL text`, `runtime.h` and the compiler's headers, the four generated headers, the flags, the defines, the
/// link address).
pub fn cache_key(
    root: &Path,
    texts: &BTreeMap<String, String>,
    defines: &[(String, u64)],
    base_va: u64,
) -> Result<String, String> {
    let mut header = fs::read_to_string(root.join("tools/guest_runtime/runtime.h")).map_err(|e| e.to_string())?;
    let mut compiler_headers: Vec<PathBuf> = fs::read_dir(root.join("tools/vm_compiler"))
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "h"))
        .collect();
    compiler_headers.sort();
    for p in compiler_headers {
        header += &fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    let mut parts: Vec<String> = texts.iter().map(|(k, v)| format!("{k}\0{v}")).collect();
    parts.push(header);
    parts.extend(generated_headers(root)?.into_iter().map(|(_, text)| text));
    parts.push(CFLAGS.join(" "));
    parts.push(define_flags(defines).join(" "));
    parts.push(format!("{base_va:x}"));
    Ok(sha256::hex(parts.join("\0").as_bytes()))
}

/// The linked image: its segments (address, bytes with zero fill) and its symbols (C name, address).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub segments: Vec<(u64, Vec<u8>)>,
    pub symbols: BTreeMap<String, u64>,
    /// The linked file, as `ld` wrote it.
    pub file: PathBuf,
}

fn run(cmd: &mut Command, what: &str) -> Result<(), String> {
    let out = cmd.output().map_err(|e| format!("{what}: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("{what}: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

/// Builds the runtime for `defines` at `base_va` under `root/build/guest_runtime_rust/`: the image's key, and the
/// image read back. Each source and generated header is written into a work directory of this call's own and compiled
/// there, and the linked file renamed onto `<key>.bin`, so builds of one key at once (the test threads of a suite, the
/// first time a key is met) neither write each other's files nor read a file half linked (Sprint 272: three tests
/// failed together on the first build of a changed key and passed alone).
pub fn build(root: &Path, defines: &[(String, u64)], base_va: u64) -> Result<(String, Image), String> {
    static BUILDS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let texts = texts(root)?;
    let key = cache_key(root, &texts, defines, base_va)?;
    let cache = root.join("build/guest_runtime_rust");
    let out = cache.join(format!("{key}.bin"));
    if !out.exists() {
        let call = BUILDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let work = cache.join(format!("{key}.work-{}-{call}", std::process::id()));
        let linked = work.join("image.bin");
        fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
        for (name, text) in generated_headers(root)? {
            fs::write(work.join(name), text).map_err(|e| e.to_string())?;
        }
        fs::write(work.join("api_ids.h"), services::api_header(&texts)?).map_err(|e| e.to_string())?;
        let dflags = define_flags(defines);
        let mut objects = Vec::new();
        for (name, text) in &texts {
            let src = work.join(name);
            fs::write(&src, text).map_err(|e| e.to_string())?;
            let stem = name.trim_end_matches(".c");
            let obj = work.join(format!("{stem}.o"));
            run(
                Command::new("clang")
                    .args(CFLAGS)
                    .args(&dflags)
                    .arg(format!("-I{}", root.join("tools/guest_runtime").display()))
                    .arg(format!("-I{}", root.join("tools/vm_compiler").display()))
                    .arg(format!("-I{}", work.display()))
                    .args(["-MD", "-MF"])
                    .arg(work.join(format!("{stem}.d")))
                    .arg("-c")
                    .arg(&src)
                    .arg("-o")
                    .arg(&obj),
                name,
            )?;
            objects.push(obj);
        }
        run(
            Command::new("ld")
                .args([
                    "-arch",
                    "arm64",
                    "-preload",
                    "-e",
                    "_w2m_image_entry",
                    "-segaddr",
                    "__TEXT",
                ])
                .arg(format!("0x{base_va:x}"))
                .arg("-o")
                .arg(&linked)
                .args(&objects),
            "ld",
        )?;
        // atomic on one filesystem: a reader sees no file or the whole one, and a second builder's rename puts the same
        // bytes in place
        fs::rename(&linked, &out).map_err(|e| format!("{} to {}: {e}", linked.display(), out.display()))?;
        let _ = fs::remove_dir_all(&work);
    }
    let data = fs::read(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let (segments, symbols) = macho::parse_preload(&data)?;
    Ok((
        key,
        Image {
            segments,
            symbols,
            file: out,
        },
    ))
}

/// Reading the linked MH_PRELOAD Mach-O (`guest_runtime._parse`).
pub mod macho {
    use std::collections::BTreeMap;

    const MH_MAGIC_64: u32 = 0xFEED_FACF;
    const MH_PRELOAD: u32 = 0x5;
    const LC_SEGMENT_64: u32 = 0x19;
    const LC_SYMTAB: u32 = 0x2;

    fn u32_at(d: &[u8], p: usize) -> Result<u32, String> {
        d.get(p..p + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .ok_or(format!("truncated at {p}"))
    }

    fn u64_at(d: &[u8], p: usize) -> Result<u64, String> {
        d.get(p..p + 8)
            .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
            .ok_or(format!("truncated at {p}"))
    }

    /// Segments with their zero fill, and the symbols defined in a section (N_SECT), the leading underscore dropped.
    #[allow(clippy::type_complexity, reason = "the pair the Python returned")]
    pub fn parse_preload(data: &[u8]) -> Result<(Vec<(u64, Vec<u8>)>, BTreeMap<String, u64>), String> {
        let (magic, filetype, ncmds) = (u32_at(data, 0)?, u32_at(data, 12)?, u32_at(data, 16)?);
        if magic != MH_MAGIC_64 || filetype != MH_PRELOAD {
            return Err(format!(
                "not an MH_PRELOAD Mach-O: magic {magic:#x}, filetype {filetype}"
            ));
        }
        let (mut segments, mut symbols) = (Vec::new(), BTreeMap::new());
        let mut p = 32;
        for _ in 0..ncmds {
            let (cmd, size) = (u32_at(data, p)?, u32_at(data, p + 4)? as usize);
            if cmd == LC_SEGMENT_64 {
                let (vmaddr, vmsize) = (u64_at(data, p + 24)?, u64_at(data, p + 32)?);
                let (fileoff, filesize) = (u64_at(data, p + 40)? as usize, u64_at(data, p + 48)? as usize);
                if vmsize > 0 {
                    let mut body = data
                        .get(fileoff..fileoff + filesize)
                        .ok_or("segment past the file")?
                        .to_vec();
                    body.resize(vmsize as usize, 0);
                    segments.push((vmaddr, body));
                }
            } else if cmd == LC_SYMTAB {
                let (symoff, nsyms, stroff) = (
                    u32_at(data, p + 8)? as usize,
                    u32_at(data, p + 12)? as usize,
                    u32_at(data, p + 16)? as usize,
                );
                for i in 0..nsyms {
                    let e = symoff + 16 * i;
                    let (strx, ntype, value) = (u32_at(data, e)? as usize, data[e + 4], u64_at(data, e + 8)?);
                    if ntype & 0x0E == 0x0E {
                        let start = stroff + strx;
                        let end = start
                            + data[start..]
                                .iter()
                                .position(|b| *b == 0)
                                .ok_or("an unterminated name")?;
                        let name = String::from_utf8_lossy(&data[start..end]);
                        symbols.insert(name.strip_prefix('_').unwrap_or(&name).to_string(), value);
                    }
                }
            }
            p += size;
        }
        Ok((segments, symbols))
    }
}

/// SHA-256 (FIPS 180-4), for the cache key: no crate (Decision 2026-10-24), and `shasum` would be a process a key.
pub mod sha256 {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98,
        0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
        0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8,
        0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819,
        0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
        0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    /// The digest of `data` as 64 lowercase hex digits.
    pub fn hex(data: &[u8]) -> String {
        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
        ];
        let mut msg = data.to_vec();
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
        for chunk in msg.chunks(64) {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes(chunk[4 * i..4 * i + 4].try_into().unwrap());
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
            }
            let mut v = h;
            for i in 0..64 {
                let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
                let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
                let t1 = v[7]
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
                let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
                let t2 = s0.wrapping_add(maj);
                v = [
                    t1.wrapping_add(t2),
                    v[0],
                    v[1],
                    v[2],
                    v[3].wrapping_add(t1),
                    v[4],
                    v[5],
                    v[6],
                ];
            }
            for (a, b) in h.iter_mut().zip(v) {
                *a = a.wrapping_add(b);
            }
        }
        h.iter().map(|x| format!("{x:08x}")).collect()
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn the_fips_examples_hash_to_their_published_digests() {
            assert_eq!(
                super::hex(b"abc"),
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            );
            assert_eq!(
                super::hex(b""),
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            );
            assert_eq!(
                super::hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
            );
        }
    }
}
