// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The CPU cases and their Windows results (Sprint 144), carried from `scripts/semantics_differential.py` and
//! `scripts/vm_oracle.py`. A case is one instruction's bytes and the state it starts from: the 16 GPRs, RFLAGS, MXCSR,
//! XMM0-15, for format 3 the YMM upper halves, for format 4 the x87 state, then three memory windows. A result is the
//! state Windows on Intel left, or the fault it took. The files are written by the frozen Python and read here byte for
//! byte. Each item cites the lines it carries.

use std::fmt;

/// semantics_differential 65-81.
pub const CODE_VA: u64 = 0x2000_0000;
pub const DATA_VA: u64 = 0x3000_0000;
pub const STACK_VA: u64 = 0x4000_0000;
pub const DATA_OFF: u64 = 0x40;
pub const STACK_OFF: u64 = 0x87C0;
pub const WINDOW: usize = 64;
pub const CODE_BYTES: usize = 32;
/// the last 32 bytes of the data page
pub const EDGE_OFF: u64 = 0xFE0;
pub const EDGE: usize = 32;
pub const PAGE_END: u64 = DATA_VA + 0x1000;
pub const STACK_SIZE: u64 = 0x10000;
pub const RET_TARGET: u64 = CODE_VA + 0x100;
pub const FLAG_BITS: [(char, u32); 6] = [('C', 0), ('P', 2), ('A', 4), ('Z', 6), ('S', 7), ('O', 11)];
pub const DF_BIT: u32 = 10;
pub const CASE_SIZE: usize = 8 + CODE_BYTES + 8 * 16 + 8 + 8 + 16 * 16 + WINDOW + WINDOW + EDGE;
pub const RESULT_SIZE: usize = 8 * 16 + 8 * 5 + 16 * 16 + WINDOW + WINDOW + EDGE;
/// format 4: FCW, FSW, abridged tags, padding, ST0-ST7
pub const X87_BLOCK: usize = 16 + 8 * 10;
/// x86fp.MXCSR_DEFAULT: every exception masked, round to nearest.
pub const MXCSR_DEFAULT: u64 = 0x1F80;
/// Windows reports a page fault, and #GP on a misaligned SSE operand, as an access violation (ntstatus.h).
pub const ACCESS_VIOLATION: u64 = 0xC000_0005;

/// A file that is not what its header or its size says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatError(pub String);

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FormatError {}

fn u16_at(raw: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(raw[at..at + 2].try_into().unwrap())
}

fn u32_at(raw: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(raw[at..at + 4].try_into().unwrap())
}

fn u64_at(raw: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(raw[at..at + 8].try_into().unwrap())
}

fn u128_at(raw: &[u8], at: usize) -> u128 {
    u128::from_le_bytes(raw[at..at + 16].try_into().unwrap())
}

/// The x87 state as FXSAVE holds it (semantics_differential 118-134): the control and status words, the abridged tags
/// (bit p: physical register p in use) and ST0-ST7 in stack order, each an 80-bit pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X87State {
    pub cw: u16,
    pub sw: u16,
    pub tags: u8,
    pub st: [u128; 8],
}

impl X87State {
    /// `pack`: `<HHB11x` then each register's ten bytes.
    pub fn pack(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(X87_BLOCK);
        out.extend(self.cw.to_le_bytes());
        out.extend(self.sw.to_le_bytes());
        out.push(self.tags);
        out.extend([0; 11]);
        for r in &self.st {
            out.extend(&r.to_le_bytes()[..10]);
        }
        out
    }

    pub fn unpack(raw: &[u8], at: usize) -> X87State {
        let mut st = [0u128; 8];
        for (i, r) in st.iter_mut().enumerate() {
            let p = at + 16 + 10 * i;
            let mut b = [0u8; 16];
            b[..10].copy_from_slice(&raw[p..p + 10]);
            *r = u128::from_le_bytes(b);
        }
        X87State {
            cw: u16_at(raw, at),
            sw: u16_at(raw, at + 2),
            tags: raw[at + 4],
            st,
        }
    }
}

/// semantics_differential 102-115. A committed case file carries no label; the class is the decoded iclass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    pub code: Vec<u8>,
    pub regs: [u64; 16],
    pub rflags: u64,
    pub data: Vec<u8>,
    pub stack: Vec<u8>,
    pub mxcsr: u64,
    pub xmm: [u128; 16],
    pub edge: Vec<u8>,
    /// 0 ordinary, 1 fault placement
    pub kind: u8,
    /// YMM bits 255:128 (format 3)
    pub ymm_hi: [u128; 16],
    /// format 4
    pub x87: Option<X87State>,
}

/// The size of one case record in `fmt`.
pub fn case_size(fmt: u32) -> usize {
    CASE_SIZE
        + match fmt {
            3 => 256,
            4 => X87_BLOCK,
            _ => 0,
        }
}

/// `case_format` and `read_cases` (semantics_differential 767-801): the format and the cases of a case file.
pub fn read_cases(raw: &[u8]) -> Result<(u32, Vec<Case>), FormatError> {
    if raw.len() < 12 || &raw[..4] != b"W2MS" {
        return Err(FormatError("not a case file: no W2MS header".into()));
    }
    let fmt = u32_at(raw, 4);
    if !(2..=4).contains(&fmt) {
        return Err(FormatError(format!("case format {fmt}")));
    }
    let count = u32_at(raw, 8) as usize;
    let size = case_size(fmt);
    if raw.len() != 12 + count * size {
        return Err(FormatError(format!(
            "{} bytes for {count} cases of {size} bytes and a 12-byte header",
            raw.len()
        )));
    }
    let mut out = Vec::with_capacity(count);
    for n in 0..count {
        let p = 12 + n * size;
        let code = raw[p + 8..p + 8 + usize::from(raw[p])].to_vec();
        let mut q = p + 8 + CODE_BYTES;
        let regs: [u64; 16] = std::array::from_fn(|i| u64_at(raw, q + 8 * i));
        let (rflags, mxcsr) = (u64_at(raw, q + 128), u64_at(raw, q + 136));
        q += 144;
        let xmm: [u128; 16] = std::array::from_fn(|i| u128_at(raw, q + 16 * i));
        q += 256;
        let mut ymm_hi = [0u128; 16];
        if fmt == 3 {
            ymm_hi = std::array::from_fn(|i| u128_at(raw, q + 16 * i));
            q += 256;
        }
        let mut x87 = None;
        if fmt == 4 {
            x87 = Some(X87State::unpack(raw, q));
            q += X87_BLOCK;
        }
        out.push(Case {
            code,
            regs,
            rflags,
            data: raw[q..q + WINDOW].to_vec(),
            stack: raw[q + WINDOW..q + 2 * WINDOW].to_vec(),
            mxcsr,
            xmm,
            edge: raw[q + 2 * WINDOW..q + 2 * WINDOW + EDGE].to_vec(),
            kind: raw[p + 1],
            ymm_hi,
            x87,
        });
    }
    Ok((fmt, out))
}

/// `write_cases` (semantics_differential 752-764).
pub fn write_cases(cases: &[Case], fmt: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + cases.len() * case_size(fmt));
    out.extend(b"W2MS");
    out.extend(fmt.to_le_bytes());
    out.extend((cases.len() as u32).to_le_bytes());
    for c in cases {
        out.push(c.code.len() as u8);
        out.push(c.kind);
        out.extend([0; 6]);
        let mut code = c.code.clone();
        code.resize(CODE_BYTES.max(code.len()), 0);
        out.extend(&code);
        for r in c.regs.iter().chain([&c.rflags, &c.mxcsr]) {
            out.extend(r.to_le_bytes());
        }
        for x in &c.xmm {
            out.extend(x.to_le_bytes());
        }
        if fmt == 3 {
            for x in &c.ymm_hi {
                out.extend(x.to_le_bytes());
            }
        }
        if fmt == 4 {
            out.extend(c.x87.expect("a format-4 case carries the x87 state").pack());
        }
        out.extend(&c.data);
        out.extend(&c.stack);
        out.extend(&c.edge);
    }
    out
}

/// semantics_differential 819-832.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseResult {
    pub regs: [u64; 16],
    pub rflags: u64,
    /// 0 or 1: the path the case completed by; 2: a fault
    pub marker: u64,
    pub fault: u64,
    pub data: Vec<u8>,
    pub stack: Vec<u8>,
    pub fault_rip: u64,
    pub mxcsr: u64,
    pub xmm: [u128; 16],
    pub edge: Vec<u8>,
    /// format 3: YMM bits 255:128 after the case
    pub ymm_hi: Option<[u128; 16]>,
    /// format 4: the x87 state after the case (at a fault, the CONTEXT's)
    pub x87: Option<X87State>,
}

/// `result_size` (semantics_differential 840-841).
pub fn result_size(fmt: u32) -> usize {
    RESULT_SIZE
        + match fmt {
            3 => 256,
            4 => X87_BLOCK,
            _ => 0,
        }
}

/// `parse_result` (semantics_differential 844-860).
pub fn parse_result(rec: &[u8], fmt: u32) -> CaseResult {
    let regs: [u64; 16] = std::array::from_fn(|i| u64_at(rec, 8 * i));
    let w = |k: usize| u64_at(rec, 128 + 8 * k);
    let mut q = 168;
    let xmm: [u128; 16] = std::array::from_fn(|i| u128_at(rec, q + 16 * i));
    q += 256;
    let mut ymm_hi = None;
    if fmt == 3 {
        ymm_hi = Some(std::array::from_fn(|i| u128_at(rec, q + 16 * i)));
        q += 256;
    }
    let mut x87 = None;
    if fmt == 4 {
        x87 = Some(X87State::unpack(rec, q));
        q += X87_BLOCK;
    }
    CaseResult {
        regs,
        rflags: w(0),
        marker: w(1),
        fault: w(2),
        fault_rip: w(3),
        mxcsr: w(4),
        data: rec[q..q + WINDOW].to_vec(),
        stack: rec[q + WINDOW..q + 2 * WINDOW].to_vec(),
        xmm,
        edge: rec[q + 2 * WINDOW..q + 2 * WINDOW + EDGE].to_vec(),
        ymm_hi,
        x87,
    }
}

/// One result record, as the runner wrote it: `parse_result`'s inverse, for the round trip.
pub fn pack_result(r: &CaseResult) -> Vec<u8> {
    let mut out = Vec::new();
    for v in r
        .regs
        .iter()
        .chain([&r.rflags, &r.marker, &r.fault, &r.fault_rip, &r.mxcsr])
    {
        out.extend(v.to_le_bytes());
    }
    for x in &r.xmm {
        out.extend(x.to_le_bytes());
    }
    if let Some(y) = &r.ymm_hi {
        for x in y {
            out.extend(x.to_le_bytes());
        }
    }
    if let Some(x) = &r.x87 {
        out.extend(x.pack());
    }
    out.extend(&r.data);
    out.extend(&r.stack);
    out.extend(&r.edge);
    out
}

/// `read_results` (semantics_differential 835-837).
pub fn read_results(raw: &[u8], fmt: u32) -> Result<Vec<CaseResult>, FormatError> {
    let size = result_size(fmt);
    if !raw.len().is_multiple_of(size) {
        return Err(FormatError(format!(
            "{} bytes is not a whole number of {size}-byte results",
            raw.len()
        )));
    }
    Ok(raw.chunks(size).map(|rec| parse_result(rec, fmt)).collect())
}

/// `read_fault_results` (vm_oracle 70-83): a fault extract, case index -> the Windows result, for every case that
/// faulted on the CPU; the format follows from the record size (3 when it fits, else 2). Also returns the format.
pub fn read_fault_results(raw: &[u8]) -> Result<(u32, Vec<(u32, CaseResult)>), FormatError> {
    if raw.len() < 8 || &raw[..4] != b"W2MF" {
        return Err(FormatError("not a fault extract: no W2MF header".into()));
    }
    let count = u32_at(raw, 4) as usize;
    let fmt = if raw.len() > 8 && count != 0 && (raw.len() - 8) / count == 4 + result_size(3) {
        3
    } else {
        2
    };
    let step = 4 + result_size(fmt);
    if raw.len() != 8 + count * step {
        return Err(FormatError(format!(
            "{} bytes for {count} faults of {step} bytes",
            raw.len()
        )));
    }
    let out = (0..count)
        .map(|n| {
            let p = 8 + n * step;
            (u32_at(raw, p), parse_result(&raw[p + 4..p + step], fmt))
        })
        .collect();
    Ok((fmt, out))
}

/// A fault extract written back: `read_fault_results`' inverse, for the round trip.
pub fn write_fault_results(faults: &[(u32, CaseResult)]) -> Vec<u8> {
    let mut out = b"W2MF".to_vec();
    out.extend((faults.len() as u32).to_le_bytes());
    for (index, r) in faults {
        out.extend(index.to_le_bytes());
        out.extend(pack_result(r));
    }
    out
}

/// The SHA-256 a `results.sha256` or `cases.sha256` file names for `file` (vm_oracle 58-59): the first word of the line
/// ending in `file`, or of the only line.
pub fn recorded_sha256(sums: &str, file: &str) -> Option<String> {
    let lines: Vec<&str> = sums.lines().filter(|l| !l.trim().is_empty()).collect();
    let line = lines
        .iter()
        .find(|l| l.trim().ends_with(file))
        .or(if lines.len() == 1 { lines.first() } else { None })?;
    Some(line.split_whitespace().next()?.to_lowercase())
}

/// SHA-256 as 64 lowercase hex digits (the dependency-free one of `guest_runtime`).
pub fn sha256(data: &[u8]) -> String {
    guest_runtime::sha256::hex(data)
}

/// x86ops' register name -> (index, bits, high byte): `lift.REG` (lift.py 79-89).
fn register(name: &str) -> Option<(usize, u32)> {
    use x86_decoder::operands::{GPR8_LEGACY, GPR8_REX, GPR16, GPR32, GPR64};
    for (table, bits) in [(&GPR64[..], 64), (&GPR32[..], 32), (&GPR16[..], 16), (&GPR8_REX[..], 8)] {
        if let Some(i) = table.iter().position(|n| *n == name) {
            return Some((i, bits));
        }
    }
    GPR8_LEGACY[4..].iter().position(|n| *n == name).map(|i| (i, 8))
}

/// `undefined_memory` (semantics_differential 917-943): why a case cannot be judged, or `None`. BT, BTS, BTR and BTC
/// with a register offset address a bit string, which may lie outside the windows a case defines, where the runner's
/// page holds bytes the case never set; those cases are set aside, counted, not judged.
pub fn undefined_memory(c: &Case) -> Option<&'static str> {
    let d = x86_decoder::decode(&c.code, None).ok()?;
    if !["BT", "BTS", "BTR", "BTC"].contains(&d.iclass.as_str()) {
        return None;
    }
    let o = x86_decoder::operands::operands(&d.fields).unwrap_or_default();
    if o.len() < 2 || !o[0].starts_with('M') || !o[1].starts_with('R') {
        return None;
    }
    let p: Vec<&str> = o[0].split(':').collect();
    let wrap = if d.fields.prefixes.contains(&0x67) {
        (1u64 << 32) - 1
    } else {
        u64::MAX
    };
    let reg_value = |name: &str| register(name).map_or(0, |(i, _)| c.regs[i]);
    let mut ea =
        (p[5].parse::<i64>().unwrap() as u64).wrapping_add(if p[2] != "INVALID" { reg_value(p[2]) } else { 0 });
    if p[3] != "INVALID" {
        ea = ea.wrapping_add(reg_value(p[3]).wrapping_mul(p[4].parse::<u64>().unwrap()));
    }
    let (idx, obits) = register(o[1].split(':').nth(1).unwrap())?;
    let mask = if obits == 64 { u64::MAX } else { (1u64 << obits) - 1 };
    let raw = c.regs[idx] & mask;
    // the offset as a signed integer of `obits`
    let off: i128 = if raw >> (obits - 1) & 1 == 1 {
        i128::from(raw) - (1i128 << obits)
    } else {
        i128::from(raw)
    };
    let size: i128 = p[6].parse().unwrap();
    // Python's floor division: off // (8 * size)
    let step = off.div_euclid(8 * size);
    let addr = ((i128::from(ea) + size * step) as u64) & wrap;
    let size = size as u64;
    let windows = [
        (DATA_VA + DATA_OFF, WINDOW as u64),
        (DATA_VA + EDGE_OFF, EDGE as u64),
        (STACK_VA + STACK_OFF, WINDOW as u64),
    ];
    if windows.iter().any(|(lo, n)| *lo <= addr && addr + size <= lo + n) {
        return None;
    }
    if ![DATA_VA, STACK_VA].iter().any(|lo| *lo <= addr && addr < lo + 0x1000) {
        return None; // unmapped on both: the fault is judged
    }
    Some("bit string outside the case's windows (not judged)")
}
