// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! w2m's x86-64 decoder (Sprint 143), carried from `src/w2m/x86dec.py`: length, mnemonic and XED-style iclass, and the
//! fields the operand layer needs, driven by tables `build.rs` generates from `specs/x86_decode.json` (Intel SDM opcode
//! maps). Its structure follows Intel SDM Vol 2A 2.1-2.3: legacy prefixes, REX, VEX (C4/C5), EVEX, XOP, the opcode
//! (one-byte, 0F, 0F38, 0F3A), ModRM, SIB, displacement, immediate. Each item cites the `x86dec.py` lines it carries.

use std::fmt;

/// An SDM NP or NFx row (gen_decoder 56).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Np {
    No,
    /// 66, F2 and F3 are not allowed
    Np,
    /// F2 and F3 are not allowed; 66 is
    Nfx,
}

/// One spec row: the Python tables' `Entry` (gen_decoder 50-57).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub pfx: &'static str,
    pub reg: Option<u8>,
    pub has_modrm: bool,
    pub imm: Option<&'static str>,
    pub mn: &'static str,
    pub sizes: Option<&'static [(&'static str, &'static str)]>,
    pub modrm_byte: Option<u8>,
    pub w: Option<u8>,
    pub l: Option<u8>,
    pub form: Option<&'static str>,
    pub iclass: &'static str,
    pub iclass_sizes: Option<&'static [(&'static str, &'static str)]>,
    pub requires: Option<&'static str>,
    pub fallback: Option<(&'static str, &'static str)>,
    pub lock_ok: bool,
    pub vvvv: Option<&'static str>,
    pub ops: Option<&'static str>,
    pub opsize64: Option<&'static str>,
    pub row: &'static str,
    pub np: Np,
    pub reg_valid: Option<&'static [u8]>,
}

include!(concat!(env!("OUT_DIR"), "/tables.rs"));

/// x86dec 17-23.
const LEGACY: [u8; 11] = [0xF0, 0xF2, 0xF3, 0x2E, 0x36, 0x3E, 0x26, 0x64, 0x65, 0x66, 0x67];
pub const MAX_LEN: usize = 15;
const VEX_PFX: [&str; 4] = ["none", "66", "f3", "f2"];

fn vex_map(m: u8) -> Option<&'static str> {
    match m {
        1 => Some("0f"),
        2 => Some("0f38"),
        3 => Some("0f3a"),
        _ => None,
    }
}

fn xop_map(m: u8) -> Option<&'static str> {
    match m {
        8 => Some("x8"),
        9 => Some("x9"),
        10 => Some("xa"),
        _ => None,
    }
}

/// The encoding class (x86dec 117; the tables' key).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Legacy,
    Vex,
    Evex,
    Xop,
}

impl Encoding {
    pub fn name(self) -> &'static str {
        match self {
            Encoding::Legacy => "legacy",
            Encoding::Vex => "vex",
            Encoding::Evex => "evex",
            Encoding::Xop => "xop",
        }
    }
}

/// x86dec 26-43: the decoded fields the operand layer needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fields {
    pub prefixes: Vec<u8>,
    pub rex: u8,
    pub enc: Encoding,
    pub opcode: u8,
    pub modrm: Option<u8>,
    pub sib: Option<u8>,
    /// sign-extended displacement (0 when absent)
    pub disp: i64,
    pub disp_size: u8,
    /// raw immediate bytes, in encoding order
    pub imm: Vec<u8>,
    /// 16, 32 or 64
    pub opsize: u8,
    /// 32 or 64
    pub asize: u8,
    /// the row's operand template (SDM Appendix A notation)
    pub ops: Option<&'static str>,
    /// as encoded (inverted): the register is vvvv ^ 15
    pub vvvv: Option<u8>,
    /// VEX.L: 0 for 128-bit, 1 for 256-bit operands
    pub vl: u8,
}

/// x86dec 46-52.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub length: usize,
    pub mnemonic: String,
    pub iclass: String,
    pub fields: Fields,
    /// the row's index in `ROWS`
    pub index: usize,
    /// sha256 of the spec row that decoded it
    pub row: &'static str,
}

/// x86dec 55-67: the decode errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// the spec has no entry for these bytes
    NoEntry(String),
    /// the bytes break a validity rule of the spec: the guest observes a fault, not an instruction
    InvalidEncoding(String),
    /// the input ends inside the instruction (XED BUFFER_TOO_SHORT)
    Truncated(String),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::NoEntry(m) | DecodeError::InvalidEncoding(m) | DecodeError::Truncated(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for DecodeError {}

/// The input's bytes, refusing a read past its end as the Python's IndexError, which `decode` names Truncated.
struct Bytes<'a>(&'a [u8]);

/// The read past the end; `decode` turns it into `Truncated` (x86dec 84-87).
struct PastEnd;

impl Bytes<'_> {
    fn at(&self, i: usize) -> Result<u8, PastEnd> {
        self.0.get(i).copied().ok_or(PastEnd)
    }
}

enum Fail {
    PastEnd,
    Error(DecodeError),
}

impl From<PastEnd> for Fail {
    fn from(_: PastEnd) -> Self {
        Fail::PastEnd
    }
}

fn invalid<T>(m: String) -> Result<T, Fail> {
    Err(Fail::Error(DecodeError::InvalidEncoding(m)))
}

fn no_entry<T>(m: String) -> Result<T, Fail> {
    Err(Fail::Error(DecodeError::NoEntry(m)))
}

/// The rows for (map, opcode, encoding), with their first index.
fn candidates(map: &str, op: u8, enc: Encoding) -> Option<(usize, &'static [Row])> {
    let at = INDEX
        .binary_search_by(|((m, o, e), _, _)| (*m, *o, *e).cmp(&(map, op, enc.name())))
        .ok()?;
    let (_, start, count) = INDEX[at];
    Some((start, &ROWS[start..start + count]))
}

/// `_pick` (x86dec 70-76): the most specific prefix present that has entries wins.
fn pick(cands: &[(usize, &'static Row)], mpfx: &[&str]) -> Vec<(usize, &'static Row)> {
    for p in mpfx {
        let hit: Vec<_> = cands.iter().filter(|(_, c)| c.pfx == *p).copied().collect();
        if !hit.is_empty() {
            return hit;
        }
    }
    Vec::new()
}

/// `decode` (x86dec 79-90): one instruction from the start of `b`. `features` is the CPU profile's feature set (XED
/// names such as BMI1, LZCNT); `None` means a CPU with every feature.
pub fn decode(b: &[u8], features: Option<&[&str]>) -> Result<Decoded, DecodeError> {
    match decode_inner(&Bytes(b), features) {
        Ok(d) if d.length > b.len() => Err(DecodeError::Truncated(format!(
            "instruction needs {} bytes; input has {}",
            d.length,
            b.len()
        ))),
        Ok(d) => Ok(d),
        Err(Fail::PastEnd) => Err(DecodeError::Truncated(format!(
            "input of {} bytes ends inside the instruction",
            b.len()
        ))),
        Err(Fail::Error(e)) => Err(e),
    }
}

fn sized(sizes: &[(&'static str, &'static str)], opsize: &str) -> &'static str {
    sizes.iter().find(|(k, _)| *k == opsize).map(|(_, v)| *v).unwrap()
}

/// `_decode` (x86dec 93-287).
fn decode_inner(b: &Bytes<'_>, features: Option<&[&str]>) -> Result<Decoded, Fail> {
    let mut i = 0;
    let mut pfx: Vec<u8> = Vec::new();
    let mut rex = 0u8;
    // Prefixes. A REX byte applies only if it immediately precedes the opcode (or VEX); a legacy prefix after a REX
    // cancels it (SDM Vol 2A 2.2.1).
    while i < MAX_LEN && (LEGACY.contains(&b.at(i)?) || (0x40..=0x4F).contains(&b.at(i)?)) {
        let x = b.at(i)?;
        if (0x40..=0x4F).contains(&x) {
            rex = x;
        } else {
            pfx.push(x);
            rex = 0;
        }
        i += 1;
    }
    if i >= MAX_LEN {
        return invalid("prefixes reach 15 bytes; length over 15 (#GP)".into());
    }
    let v = b.at(i)?;
    if [0x62, 0xC4, 0xC5].contains(&v) || (v == 0x8F && (b.at(i + 1)? & 0x1F) >= 8) {
        // In 64-bit mode C4/C5 are always VEX and 62 always EVEX. A 66/F2/F3/F0 or REX before VEX, EVEX or XOP causes
        // #UD (SDM Vol 2A 2.3.2; XED BAD_LEGACY_PREFIX, BAD_REX_PREFIX).
        let bad: Vec<String> = pfx
            .iter()
            .filter(|p| [0x66, 0xF2, 0xF3, 0xF0].contains(*p))
            .map(|p| format!("{p:02x}"))
            .collect();
        if !bad.is_empty() {
            return invalid(format!("legacy prefix {} before VEX/EVEX/XOP", bad.join(" ")));
        }
        if rex != 0 {
            return invalid(format!("REX {rex:02x} before VEX/EVEX/XOP"));
        }
    }
    let mut enc = Encoding::Legacy;
    let mut w = (rex >> 3) & 1;
    let mut vex_rex = 0u8; // VEX's W, R, X, B as a REX byte, for the operand layer
    let mut vl: Option<u8> = None;
    let mut vvvv: Option<u8> = None;
    let mp: &'static str;
    let mpfx: Vec<&str> = if b.at(i)? == 0x62 && rex == 0 {
        // EVEX (SDM Vol 2A 2.7): 62 P0 P1 P2; in 64-bit mode 62 is never BOUND
        let (p0, p1, p2) = (b.at(i + 1)?, b.at(i + 2)?, b.at(i + 3)?);
        let Some(m) = vex_map(p0 & 7) else {
            return no_entry(format!("EVEX map field {}", p0 & 7));
        };
        (enc, mp, w, vl) = (Encoding::Evex, m, p1 >> 7, Some((p2 >> 5) & 3));
        i += 4;
        vec![VEX_PFX[usize::from(p1 & 3)]]
    } else if b.at(i)? == 0x8F && (b.at(i + 1)? & 0x1F) >= 8 && rex == 0 {
        // XOP (AMD APM Vol 6): 8F RXB.mmmmm W.vvvv.L.pp; map field < 8 means POP r/m instead
        let (p1, p2) = (b.at(i + 1)?, b.at(i + 2)?);
        let Some(m) = xop_map(p1 & 0x1F) else {
            return no_entry(format!("XOP map field {}", p1 & 0x1F));
        };
        (enc, mp, w, vl) = (Encoding::Xop, m, p2 >> 7, Some((p2 >> 2) & 1));
        vvvv = Some((p2 >> 3) & 0xF);
        i += 3;
        vec![VEX_PFX[usize::from(p2 & 3)]]
    } else if [0xC4, 0xC5].contains(&b.at(i)?) && rex == 0 {
        enc = Encoding::Vex;
        let pp;
        if b.at(i)? == 0xC5 {
            let p = b.at(i + 1)?;
            (mp, pp, vl, vvvv) = ("0f", p & 3, Some((p >> 2) & 1), Some((p >> 3) & 0xF));
            vex_rex = 0x40 | ((!p >> 7) & 1) << 2; // R, stored inverted (SDM Vol 2A 2.3.5)
            i += 2;
        } else {
            let (p1, p2) = (b.at(i + 1)?, b.at(i + 2)?);
            let Some(m) = vex_map(p1 & 0x1F) else {
                return no_entry(format!("VEX map field {}", p1 & 0x1F));
            };
            (mp, pp, w, vl) = (m, p2 & 3, p2 >> 7, Some((p2 >> 2) & 1));
            vvvv = Some((p2 >> 3) & 0xF);
            vex_rex = 0x40 | w << 3 | ((!p1 >> 5) & 7); // W; R, X, B inverted
            i += 3;
        }
        vec![VEX_PFX[usize::from(pp)]]
    } else {
        if b.at(i)? == 0x0F {
            match b.at(i + 1)? {
                0x38 => {
                    mp = "0f38";
                    i += 2;
                }
                0x3A => {
                    mp = "0f3a";
                    i += 2;
                }
                _ => {
                    mp = "0f";
                    i += 1;
                }
            }
        } else {
            mp = "1";
        }
        let reps: Vec<u8> = pfx.iter().copied().filter(|x| *x == 0xF2 || *x == 0xF3).collect();
        let mut m = Vec::new();
        if let Some(r) = reps.last() {
            m.push(if *r == 0xF2 { "f2" } else { "f3" });
        }
        if pfx.contains(&0x66) {
            m.push("66");
        }
        m.push("none");
        m
    };
    let op = b.at(i)?;
    i += 1;
    let Some((start, rows)) = candidates(mp, op, enc) else {
        let tail = if enc == Encoding::Legacy {
            String::new()
        } else {
            format!(" ({})", enc.name())
        };
        return no_entry(format!("no entry for map {mp} opcode 0x{op:02x}{tail}"));
    };
    let cands: Vec<(usize, &'static Row)> = rows.iter().enumerate().map(|(k, r)| (start + k, r)).collect();
    let sse_family = mp != "1" && cands.iter().any(|(_, c)| c.pfx != "none");
    let legacy_opcode = LEGACY_PREFIX_OPCODES.iter().find(|((m, o), _)| *m == mp && *o == op);
    let fits = |c: &Row, m: u8| {
        c.reg.is_none_or(|r| r == (m >> 3) & 7)
            && c.modrm_byte.is_none_or(|x| x == m)
            && c.form.is_none_or(|f| (f == "reg") == (m >> 6 == 3))
    };
    let mut chosen: Vec<(usize, &'static Row)>;
    if enc == Encoding::Legacy
        && sse_family
        && let Some((_, legacy)) = legacy_opcode
    {
        // A general-purpose opcode with F2/F3 rows: a prefix with no row keeps its legacy meaning only where the spec
        // says so (MOVBE: 66 is the operand size, F3 is invalid; BSF: F2 is ignored). Walk the prefixes in precedence
        // order.
        chosen = Vec::new();
        for prefix in &mpfx {
            let mut rows: Vec<&Row> = cands
                .iter()
                .filter(|(_, c)| c.pfx == *prefix)
                .map(|(_, c)| *c)
                .collect();
            if !rows.is_empty() && rows.iter().any(|c| c.has_modrm) && i < b.0.len() {
                // a prefix's rows count only if one fits this ModRM (F3 0F C7 /1 is CMPXCHG8B: the F3 rows are /6
                // and /7)
                let m = b.at(i)?;
                rows.retain(|c| fits(c, m));
            }
            if !rows.is_empty() {
                chosen = cands.iter().filter(|(_, c)| c.pfx == *prefix).copied().collect();
                break;
            }
            if *prefix != "none" && !legacy.contains(prefix) {
                break;
            }
        }
    } else if enc == Encoding::Legacy && sse_family {
        // The mandatory prefix is the present prefix of highest precedence (the last F2/F3, then 66). With no row for
        // it the bytes are not an instruction: F2 66 0F 6F is not MOVDQA. No fallback to a lower prefix's row.
        chosen = cands.iter().filter(|(_, c)| c.pfx == mpfx[0]).copied().collect();
    } else {
        chosen = pick(&cands, &mpfx);
    }
    if chosen.is_empty() {
        return no_entry(format!(
            "no entry for map {mp} opcode 0x{op:02x} with prefixes {}",
            python_list(&mpfx)
        ));
    }
    let modrm = if chosen.iter().any(|(_, c)| c.has_modrm) {
        Some(b.at(i)?)
    } else {
        None
    };
    if let Some(m) = modrm {
        let (reg, is_reg) = ((m >> 3) & 7, m >> 6 == 3);
        chosen.retain(|(_, c)| c.modrm_byte.is_none_or(|x| x == m));
        chosen.retain(|(_, c)| c.reg.is_none_or(|r| r == reg));
        chosen.retain(|(_, c)| c.form.is_none_or(|f| (f == "reg") == is_reg));
        let exact: Vec<_> = chosen.iter().filter(|(_, c)| c.modrm_byte.is_some()).copied().collect();
        if !exact.is_empty() {
            chosen = exact;
        }
    }
    chosen.retain(|(_, c)| c.w.is_none_or(|x| x == w));
    chosen.retain(|(_, c)| {
        c.l.is_none_or(|x| matches!(enc, Encoding::Vex | Encoding::Xop) && Some(x) == vl)
    });
    if chosen.is_empty() {
        let m = modrm.map_or("none".to_string(), |m| format!("0x{m:x}"));
        let l = vl.map_or("None".to_string(), |l| l.to_string());
        return no_entry(format!("no entry for map {mp} opcode 0x{op:02x} modrm {m} w {w} l {l}"));
    }
    let (index, row) = chosen[0];
    let mut mn = row.mn.to_string();
    if row.np != Np::No && enc == Encoding::Legacy {
        let banned: &[u8] = if row.np == Np::Nfx {
            &[0xF2, 0xF3]
        } else {
            &[0x66, 0xF2, 0xF3]
        };
        if banned.iter().any(|p| pfx.contains(p)) {
            // SDM NP: 66, F2 and F3 are not allowed; NFx: F2 and F3 are not allowed, 66 is (RDRAND r16)
            let kind = if row.np == Np::Nfx { "NFx" } else { "NP" };
            return invalid(format!("{mn} is {kind}: prefix not allowed"));
        }
    }
    if let (Some(valid), Some(m)) = (row.reg_valid, modrm) {
        let r = (rex >> 2 & 1) << 3 | m >> 3 & 7;
        if !valid.contains(&r) {
            // ModRM.reg names a register that does not exist (CR1, DR8, segment register 6)
            return invalid(format!("{mn}: register {r} does not exist"));
        }
    }
    if pfx.contains(&0xF0) && !(row.lock_ok && modrm.is_some_and(|m| m >> 6 != 3)) {
        let form = if modrm.is_some_and(|m| m >> 6 == 3) {
            "register"
        } else {
            ""
        };
        return invalid(format!("LOCK prefix on {mn} {form} form"));
    }
    if matches!(enc, Encoding::Vex | Encoding::Xop) && row.vvvv == Some("1111") && vvvv != Some(0xF) {
        return invalid(format!("{}.vvvv must be 1111b for {mn}", enc.name().to_uppercase()));
    }
    let mut sib = None;
    let mut disp_size = 0usize;
    if row.has_modrm {
        let m = modrm.unwrap();
        i += 1;
        let (md, rm) = (m >> 6, m & 7);
        if md != 3 {
            if rm == 4 {
                let s = b.at(i)?;
                sib = Some(s);
                i += 1;
                if (s & 7) == 5 && md == 0 {
                    disp_size = 4;
                }
            }
            if md == 0 && rm == 5 {
                disp_size = 4;
            } else if md == 1 {
                disp_size = 1;
            } else if md == 2 {
                disp_size = 4;
            }
        }
    }
    let disp = if disp_size == 0 {
        0
    } else {
        // int.from_bytes(b[i:i + disp_size], signed=True): a slice past the input is short, read as it is
        let bytes: Vec<u8> = (i..i + disp_size).filter_map(|k| b.0.get(k).copied()).collect();
        signed(&bytes)
    };
    i += disp_size;
    let opsize = if w != 0 && enc == Encoding::Legacy {
        "64"
    } else if pfx.contains(&0x66) {
        "16"
    } else {
        "32"
    };
    let imm_size = match row.imm {
        None => 0,
        Some("b") => 1,
        Some("w") => 2,
        Some("d") => 4,
        Some("z") => {
            if opsize == "16" {
                2
            } else {
                4
            }
        }
        Some("v") => match opsize {
            "64" => 8,
            "16" => 2,
            _ => 4,
        },
        Some("moffs") => {
            if pfx.contains(&0x67) {
                4
            } else {
                8
            }
        }
        Some("wb") => 3, // ENTER iw ib (SDM ENTER)
        Some(other) => panic!("immediate kind {other}"),
    };
    let imm: Vec<u8> = (i..i + imm_size).filter_map(|k| b.0.get(k).copied()).collect();
    i += imm_size;
    if i > MAX_LEN {
        return no_entry(format!("length {i} exceeds 15"));
    }
    let mut iclass = row.iclass.to_string();
    if let Some(s) = row.sizes {
        mn = sized(s, opsize).to_string();
    }
    if let Some(s) = row.iclass_sizes {
        iclass = sized(s, opsize).to_string();
    }
    if let (Some(req), Some((fmn, fic)), Some(have)) = (row.requires, row.fallback, features)
        && !have.contains(&req)
    {
        (mn, iclass) = (fmn.to_string(), fic.to_string()); // same bytes, older CPU (spec `features` note)
    }
    if mp == "1" && op == 0xE3 && pfx.contains(&0x67) {
        (mn, iclass) = ("jecxz".into(), "JECXZ".into()); // E3 with the 67 prefix tests ECX (SDM Jcc: JECXZ)
    }
    if mp == "1" && op == 0x90 && rex & 1 != 0 && !pfx.contains(&0xF3) {
        (mn, iclass) = ("xchg".into(), "XCHG".into()); // 41 90: xchg r8, rax (SDM: NOP is 90 without REX.B)
    }
    if pfx.contains(&0xF0) && LOCKABLE.contains(&mn.as_str()) {
        mn = format!("lock {mn}");
        if iclass != "XCHG" {
            iclass.push_str("_LOCK"); // XED: XCHG with memory is locked either way
        }
    }
    // REP applies to the one-byte string opcodes only. "movsd" also names SSE F2 0F 10/11, where F2 is a mandatory
    // prefix, not a repeat (SDM Vol 2B MOVSD notes both instructions).
    if mp == "1" && REP_STRING.contains(&mn.as_str()) {
        if pfx.contains(&0xF3) {
            let word = if REPE_STRING.contains(&mn.as_str()) {
                "repe"
            } else {
                "rep"
            };
            (mn, iclass) = (format!("{word} {mn}"), format!("{}_{iclass}", word.to_uppercase()));
        } else if pfx.contains(&0xF2) {
            // REPNE has meaning only for cmps/scas; on other string ops F2 repeats like REP (XED REP_*)
            let base = mn.clone();
            mn = format!("repne {mn}");
            let word = if REPE_STRING.contains(&base.as_str()) {
                "REPNE_"
            } else {
                "REP_"
            };
            iclass = format!("{word}{iclass}");
        }
    }
    // SDM Table A-1: d64 defaults to 64 bits (66 selects 16); f64 forces 64 (66 ignored, Intel)
    let mut eff: u8 = if row.opsize64 == Some("f64") || (row.opsize64 == Some("d64") && opsize == "32") {
        64
    } else {
        opsize.parse().unwrap()
    };
    if enc == Encoding::Vex {
        eff = if vex_rex & 8 != 0 { 64 } else { 32 }; // VEX.W selects the 64-bit general-register form
    }
    let fields = Fields {
        prefixes: pfx.clone(),
        rex: if enc == Encoding::Vex { vex_rex } else { rex },
        enc,
        opcode: op,
        modrm,
        sib,
        disp,
        disp_size: disp_size as u8,
        imm,
        opsize: eff,
        asize: if pfx.contains(&0x67) { 32 } else { 64 },
        ops: row.ops,
        vvvv,
        vl: if enc == Encoding::Vex { vl.unwrap_or(0) } else { 0 },
    };
    Ok(Decoded {
        length: i,
        mnemonic: mn,
        iclass,
        fields,
        index,
        row: row.row,
    })
}

/// Little-endian bytes as a signed integer, as `int.from_bytes(..., signed=True)`.
fn signed(bytes: &[u8]) -> i64 {
    if bytes.is_empty() {
        return 0;
    }
    let mut v: u64 = 0;
    for (k, x) in bytes.iter().enumerate() {
        v |= u64::from(*x) << (8 * k);
    }
    let bits = 8 * bytes.len() as u32;
    if bits < 64 && v >> (bits - 1) & 1 == 1 {
        (v | !0u64 << bits) as i64
    } else {
        v as i64
    }
}

/// A Python list of strings as `str(list)` prints it, for the Python's message.
fn python_list(items: &[&str]) -> String {
    let quoted: Vec<String> = items.iter().map(|s| format!("'{s}'")).collect();
    format!("[{}]", quoted.join(", "))
}
