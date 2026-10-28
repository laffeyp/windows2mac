// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The model (Sprint 173; step 4's first component, `docs/step-4-census-2026-10-28.md` row 4): one lifted instruction
//! run on a thread's registers and flags over guest memory, carried from `src/w2m/runtime.py` `_exec` and `_exec_ops`
//! (995-1161) with the helpers they call (193-388) and `src/w2m/integer_model.py` (whole). The VM reaches it through
//! hvc 10 and 11 (`vm_backend.py` `_slow_sse`, 4268-4323), when a block's instruction is one the lowering leaves to it.
//!
//! This slice holds the core's integer and control operations and the integer classes. An `sse` operation is refused
//! by name: the SSE, MMX, VEX and x87 slices follow.

use translator::lift::{Lifted, Loc, MmxArg, Op, RAX, RBX, RCX, RDI, RDX, RSI, RSP, SseLoc, Temp};

const M64: u64 = u64::MAX;
/// ntstatus.h (runtime.py 26-28)
pub const STATUS_ACCESS_VIOLATION: u32 = 0xC000_0005;
pub const STATUS_INTEGER_DIVIDE_BY_ZERO: u32 = 0xC000_0094;
pub const STATUS_INTEGER_OVERFLOW: u32 = 0xC000_0095;
pub const STATUS_ILLEGAL_INSTRUCTION: u32 = 0xC000_001D;
/// XCR0: x87 | SSE | AVX (runtime.py 29)
const XCR0: u64 = 7;

/// Guest memory as the model reads and writes it (`loader.Memory`): an access wholly inside mapped memory, or a
/// fault.
pub trait Memory {
    fn read(&self, va: u64, n: usize) -> Result<Vec<u8>, Fault>;
    fn write(&mut self, va: u64, data: &[u8]) -> Result<(), Fault>;
}

/// Why a step stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// `GuestFault`: the guest sees exception `code` at the instruction
    Guest { code: u32, why: String },
    /// `UndefinedFlag`: a flag read while the SDM leaves it undefined
    Undefined(char),
    /// an operation this slice of the model does not run, or a form it refuses (`HandlerFailed`, `Unimplemented`)
    Refused(String),
}

fn guest(why: String) -> Fault {
    Fault::Guest {
        code: STATUS_ACCESS_VIOLATION,
        why,
    }
}

/// The x86 flags the model holds, None where undefined (`Thread.flags`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Flags {
    pub c: Option<bool>,
    pub p: Option<bool>,
    pub a: Option<bool>,
    pub z: Option<bool>,
    pub s: Option<bool>,
    pub o: Option<bool>,
}

impl Flags {
    fn get(&self, f: char) -> Option<bool> {
        match f {
            'C' => self.c,
            'P' => self.p,
            'A' => self.a,
            'Z' => self.z,
            'S' => self.s,
            _ => self.o,
        }
    }
}

/// The thread state one step reads and writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thread {
    pub regs: [u64; 16],
    pub flags: Flags,
    pub df: bool,
    pub mxcsr: u32,
    pub fcw: u16,
    pub rip: u64,
    /// XMM0-15 (Sprint 174: MOVDQ2Q reads one); 16 and 17 are `_vex`'s scratch, zero between steps (Sprint 183)
    pub xmm: [u128; 18],
    /// YMM0-15 bits 255:128 (`Thread.ymm_hi`; Sprint 183)
    pub ymm_hi: [u128; 16],
    /// the x87 status word, its abridged tags (bit p: physical register p in use) and the physical registers R0-R7,
    /// 80 bits each (`x87.Fpu`; Sprint 174: MMn is bits 63:0 of Rn)
    pub x87_sw: u16,
    pub x87_tags: u8,
    pub x87_regs: [u128; 8],
}

fn mask(bits: u32) -> u64 {
    if bits >= 64 { M64 } else { (1u64 << bits) - 1 }
}

fn mask128(bits: u32) -> u128 {
    if bits >= 128 { u128::MAX } else { (1u128 << bits) - 1 }
}

fn parity(v: u64) -> bool {
    (v & 0xFF).count_ones().is_multiple_of(2)
}

/// One step's state: the thread, memory, and the instruction's temporaries.
struct Step<'a, M: Memory> {
    th: &'a mut Thread,
    mem: &'a mut M,
    t: std::collections::BTreeMap<Temp, u64>,
    code: &'a [u8],
    /// `_defer_fp` and `_deferred_flags` (runtime.py 980-981): VEX.256's halves collect their flags for one check
    defer_fp: bool,
    deferred_flags: u32,
}

impl<M: Memory> Step<'_, M> {
    fn temp(&self, t: Temp) -> Result<u64, Fault> {
        self.t
            .get(&t)
            .copied()
            .ok_or_else(|| Fault::Refused(format!("temporary {t} read before it was written")))
    }

    /// `_rd` (125-127)
    fn rd(&self, idx: u8, bits: u32, hi: bool) -> u64 {
        let v = self.th.regs[usize::from(idx)];
        if hi { (v >> 8) & 0xFF } else { v & mask(bits) }
    }

    /// `_wr` (129-138): a 32-bit write zero-extends (SDM Vol 1 3.4.1.1); 8 and 16 keep the rest
    fn wr(&mut self, idx: u8, bits: u32, hi: bool, v: u64) {
        let old = self.th.regs[usize::from(idx)];
        self.th.regs[usize::from(idx)] = if hi {
            (old & !0xFF00) | ((v & 0xFF) << 8)
        } else if bits == 32 {
            v & 0xFFFF_FFFF
        } else if bits == 64 {
            v
        } else {
            (old & !mask(bits)) | (v & mask(bits))
        };
    }

    fn read_le(&self, va: u64, n: usize) -> Result<u64, Fault> {
        let b = self.mem.read(va, n)?;
        let mut w = [0u8; 8];
        w[..n.min(8)].copy_from_slice(&b[..n.min(8)]);
        Ok(u64::from_le_bytes(w))
    }

    /// `_load` (193-196)
    fn load(&self, loc: Loc) -> Result<u64, Fault> {
        match loc {
            Loc::R { idx, bits, hi } => Ok(self.rd(idx, bits, hi)),
            Loc::M { addr, bytes } => self.read_le(self.temp(addr)?, bytes as usize),
        }
    }

    /// `_store` (198-203)
    fn store(&mut self, loc: Loc, v: u64) -> Result<(), Fault> {
        match loc {
            Loc::R { idx, bits, hi } => {
                self.wr(idx, bits, hi, v);
                Ok(())
            }
            Loc::M { addr, bytes } => {
                let n = bytes as usize;
                let va = self.temp(addr)?;
                self.mem.write(va, &(v & mask(8 * bytes)).to_le_bytes()[..n])
            }
        }
    }

    /// `_flag` (206-210)
    fn flag(&self, f: char) -> Result<bool, Fault> {
        self.th.flags.get(f).ok_or(Fault::Undefined(f))
    }

    /// `_cond` (212-223)
    fn cond(&self, cc: &str) -> Result<bool, Fault> {
        if cc == "always" {
            return Ok(true);
        }
        let (neg, key) = match cc.strip_prefix('n') {
            Some(k) => (true, k),
            None => (false, cc),
        };
        let r = match key {
            "o" => self.flag('O')?,
            "b" => self.flag('C')?,
            "z" => self.flag('Z')?,
            "be" => self.flag('C')? || self.flag('Z')?,
            "s" => self.flag('S')?,
            "p" => self.flag('P')?,
            "l" => self.flag('S')? != self.flag('O')?,
            "le" => self.flag('Z')? || (self.flag('S')? != self.flag('O')?),
            _ => return Err(Fault::Refused(format!("condition {cc:?}"))),
        };
        Ok(r != neg)
    }

    /// `_set_flags` (225-296)
    fn set_flags(&mut self, kind: &str, a: u64, b: u64, r: u64, bits: u32) {
        let (m, sign) = (mask(bits), 1u64 << (bits - 1));
        let res = r & m;
        let f = &mut self.th.flags;
        if kind == "mul" {
            let c = Some(res != 0);
            *f = Flags {
                c,
                o: c,
                z: None,
                s: None,
                p: None,
                a: None,
            };
            return;
        }
        let cnt = b & if bits == 64 { 0x3F } else { 0x1F };
        if matches!(kind, "shr" | "shl" | "sar" | "rol" | "ror") && cnt == 0 {
            return;
        }
        let (zf, sf, pf) = (Some(res == 0), Some(res & sign != 0), Some(parity(res)));
        match kind {
            "shr" => {
                f.c = if cnt < u64::from(bits) {
                    Some((a >> (cnt - 1)) & 1 != 0)
                } else {
                    None
                };
                (f.z, f.s, f.p, f.a) = (zf, sf, pf, None);
                f.o = if cnt == 1 { Some(a & sign != 0) } else { None };
            }
            "shl" => {
                let c = if cnt < u64::from(bits) {
                    Some((a >> (u64::from(bits) - cnt)) & 1 != 0)
                } else {
                    None
                };
                f.c = c;
                (f.z, f.s, f.p, f.a) = (zf, sf, pf, None);
                f.o = match c {
                    Some(c) if cnt == 1 => Some((res & sign != 0) != c),
                    _ => None,
                };
            }
            "sar" => {
                let sa = if a & sign != 0 {
                    ((a & m) as i128) - (1i128 << bits)
                } else {
                    (a & m) as i128
                };
                f.c = Some((sa >> (cnt - 1)) & 1 != 0);
                (f.z, f.s, f.p, f.a) = (zf, sf, pf, None);
                f.o = if cnt == 1 { Some(false) } else { None };
            }
            "rol" => {
                f.c = Some(res & 1 != 0);
                f.o = if cnt == 1 {
                    Some((res & sign != 0) != (res & 1 != 0))
                } else {
                    None
                };
            }
            "ror" => {
                f.c = Some(res & sign != 0);
                f.o = if cnt == 1 {
                    Some((res & sign != 0) != (res & (sign >> 1) != 0))
                } else {
                    None
                };
            }
            "bt" => {
                f.c = Some(r & 1 != 0);
                (f.o, f.s, f.a, f.p) = (None, None, None, None);
            }
            "bsf" => {
                f.z = Some(a & m == 0);
                (f.c, f.o, f.s, f.a, f.p) = (None, None, None, None, None);
            }
            "popcnt" => {
                f.z = Some(a & m == 0);
                (f.c, f.o, f.s, f.a, f.p) = (Some(false), Some(false), Some(false), Some(false), Some(false));
            }
            "adc" => {
                (f.z, f.s, f.p) = (zf, sf, pf);
                f.a = Some((a ^ b ^ res) & 0x10 != 0);
                f.o = Some(!(a ^ b) & (a ^ res) & sign != 0);
                f.c = Some(((a & b) | ((a | b) & !res)) & sign != 0);
            }
            "sbb" => {
                (f.z, f.s, f.p) = (zf, sf, pf);
                f.a = Some((a ^ b ^ res) & 0x10 != 0);
                f.o = Some((a ^ b) & (a ^ res) & sign != 0);
                f.c = Some(((!a & b) | (!(a ^ b) & res)) & sign != 0);
            }
            "imul" => {
                let sx = signed(a & m, bits);
                let sy = signed(b & m, bits);
                let sr = signed(res, bits);
                let c = Some(sx * sy != sr);
                *f = Flags {
                    c,
                    o: c,
                    z: None,
                    s: None,
                    p: None,
                    a: None,
                };
            }
            "andn" => {
                (f.z, f.s, f.c, f.o, f.a, f.p) = (zf, sf, Some(false), Some(false), None, None);
            }
            _ => {
                (f.z, f.s, f.p) = (zf, sf, pf);
                if kind == "logic" {
                    (f.c, f.o, f.a) = (Some(false), Some(false), None);
                    return;
                }
                f.a = Some((a ^ b ^ res) & 0x10 != 0);
                if matches!(kind, "add" | "inc") {
                    f.o = Some(!(a ^ b) & (a ^ res) & sign != 0);
                    if kind == "add" {
                        f.c = Some(u128::from(a & m) + u128::from(b & m) > u128::from(m));
                    }
                } else if matches!(kind, "sub" | "dec") {
                    f.o = Some((a ^ b) & (a ^ res) & sign != 0);
                    if kind == "sub" {
                        f.c = Some(a & m < b & m);
                    }
                }
            }
        }
    }

    /// `_rep` (298-311): one element at a time, so a fault leaves RCX, RSI and RDI at the faulting iteration
    fn rep(&mut self, kind: &str, size: u32) -> Result<(), Fault> {
        let n = size as usize;
        let step = if self.th.df {
            (size as u64).wrapping_neg()
        } else {
            size as u64
        };
        while self.th.regs[usize::from(RCX)] != 0 {
            let value = if kind == "movs" {
                self.mem.read(self.th.regs[usize::from(RSI)], n)?
            } else {
                (self.th.regs[usize::from(RAX)] & mask(8 * size)).to_le_bytes()[..n].to_vec()
            };
            self.mem.write(self.th.regs[usize::from(RDI)], &value)?;
            let r = &mut self.th.regs;
            r[usize::from(RDI)] = r[usize::from(RDI)].wrapping_add(step);
            if kind == "movs" {
                r[usize::from(RSI)] = r[usize::from(RSI)].wrapping_add(step);
            }
            r[usize::from(RCX)] -= 1;
        }
        Ok(())
    }

    /// `_windows_divide_status` (313-341): Windows reads the divisor again by its own rules, a legacy high-byte
    /// register as SPL BPL SIL DIL and a memory operand with the full 64-bit registers; zero or unreadable is
    /// STATUS_INTEGER_DIVIDE_BY_ZERO, else STATUS_INTEGER_OVERFLOW (set 044, run 38039360955: all 127 DIV #DE cases)
    fn windows_divide_status(&self) -> u32 {
        let seen = (|| -> Option<u64> {
            let d = x86_decoder::decode(self.code, None).ok()?;
            let ops = x86_decoder::operands::operands(&d.fields).ok()?;
            let o: Vec<&str> = ops.first()?.split(':').collect();
            match *o.first()? {
                "R" => {
                    let (idx, bits, hi) = translator::lift::reg(o.get(1)?)?;
                    let idx = usize::from(idx);
                    Some(if hi {
                        self.th.regs[idx + 4] & 0xFF
                    } else {
                        self.th.regs[idx] & mask(bits)
                    })
                }
                "M" => {
                    let (base, index) = (*o.get(2)?, *o.get(3)?);
                    let (scale, disp, width): (u64, i64, usize) = (
                        o.get(4)?.parse().ok()?,
                        o.get(5)?.parse().ok()?,
                        o.get(6)?.parse().ok()?,
                    );
                    let mut addr = disp as u64;
                    if base != "INVALID" && base != "RIP" {
                        addr = addr.wrapping_add(self.th.regs[usize::from(translator::lift::reg(base)?.0)]);
                    }
                    if index != "INVALID" {
                        let i = self.th.regs[usize::from(translator::lift::reg(index)?.0)];
                        addr = addr.wrapping_add(i.wrapping_mul(scale));
                    }
                    if base == "RIP" {
                        addr = addr.wrapping_add(self.th.rip + d.length as u64);
                    }
                    self.read_le(addr, width).ok()
                }
                _ => None,
            }
        })();
        if seen.is_some_and(|v| v != 0) {
            STATUS_INTEGER_OVERFLOW
        } else {
            STATUS_INTEGER_DIVIDE_BY_ZERO
        }
    }

    /// `_div` (343-374): faults before any write; every flag undefined after
    fn div(&mut self, bits: u32, divisor: u64, signed_div: bool) -> Result<(), Fault> {
        let m = mask(bits);
        let divisor = divisor & m;
        let dividend: u128 = if bits == 8 {
            u128::from(self.rd(RAX, 16, false))
        } else {
            u128::from(self.rd(RDX, bits, false)) << bits | u128::from(self.rd(RAX, bits, false))
        };
        let (q, rem, overflow): (i128, i128, bool) = if signed_div {
            // a 128-bit dividend is its own two's complement as i128; a narrower one is sign-extended (`1 << 128` would
            // overflow the shift: set 046b's two IDIV r/m64 cases found it, RDX one off)
            let sd = if bits == 64 {
                dividend as i128
            } else if dividend >> (2 * bits - 1) & 1 == 1 {
                dividend as i128 - (1i128 << (2 * bits))
            } else {
                dividend as i128
            };
            let sv = signed(divisor, bits);
            if sv == 0 {
                (0, 0, false)
            } else {
                let q = (sd.abs() / sv.abs()) * if (sd < 0) == (sv < 0) { 1 } else { -1 };
                let rem = sd - q * sv;
                (q, rem, !(-(1i128 << (bits - 1)) <= q && q < 1i128 << (bits - 1)))
            }
        } else if divisor == 0 {
            (0, 0, false)
        } else {
            let q = dividend / u128::from(divisor);
            let rem = dividend % u128::from(divisor);
            (q as i128, rem as i128, q > u128::from(m))
        };
        if divisor == 0 || overflow {
            let why = if divisor == 0 {
                "by zero".to_string()
            } else {
                format!("quotient {q:#x} outside {bits} bits")
            };
            return Err(Fault::Guest {
                code: self.windows_divide_status(),
                why: format!(
                    "#DE: {} {why} at 0x{:x}",
                    if signed_div { "IDIV" } else { "DIV" },
                    self.th.rip
                ),
            });
        }
        let (q, rem) = ((q as u128 & mask128(bits)) as u64, (rem as u128 & mask128(bits)) as u64);
        if bits == 8 {
            self.wr(RAX, 8, false, q);
            self.wr(RAX, 8, true, rem);
        } else {
            self.wr(RAX, bits, false, q);
            self.wr(RDX, bits, false, rem);
        }
        self.th.flags = Flags::default();
        Ok(())
    }

    /// `_cmpxchg16b` (376-388)
    fn cmpxchg16b(&mut self, addr: u64) -> Result<(), Fault> {
        if !addr.is_multiple_of(16) {
            return Err(guest(format!(
                "#GP: CMPXCHG16B operand 0x{addr:x} not 16-byte aligned at 0x{:x}",
                self.th.rip
            )));
        }
        let b = self.mem.read(addr, 16)?;
        let old = u128::from_le_bytes(b.try_into().unwrap());
        let r = &self.th.regs;
        let equal = old == (u128::from(r[usize::from(RDX)]) << 64 | u128::from(r[usize::from(RAX)]));
        let new = if equal {
            u128::from(r[usize::from(RCX)]) << 64 | u128::from(r[usize::from(RBX)])
        } else {
            old
        };
        self.mem.write(addr, &new.to_le_bytes())?;
        if !equal {
            self.th.regs[usize::from(RAX)] = old as u64;
            self.th.regs[usize::from(RDX)] = (old >> 64) as u64;
        }
        self.th.flags.z = Some(equal);
        Ok(())
    }

    /// `_exec_ops` (1015-1161): returns whether the instruction branched (RIP set)
    fn exec_ops(&mut self, ins: &Lifted) -> Result<(), Fault> {
        let nxt = ins.address + ins.length;
        for op in &ins.ops {
            match op {
                Op::Imm { t, value } => {
                    self.t.insert(*t, *value);
                }
                Op::Addr {
                    t,
                    base,
                    index,
                    scale,
                    disp,
                    abits,
                    gs,
                } => {
                    if *gs {
                        return Err(Fault::Refused(format!(
                            "gs-relative access at 0x{:x}: the model has no TEB",
                            ins.address
                        )));
                    }
                    let mut v = *disp as u64;
                    if let Some(b) = base {
                        v = v.wrapping_add(self.th.regs[usize::from(*b)]);
                    }
                    if let Some(i) = index {
                        v = v.wrapping_add(self.th.regs[usize::from(*i)].wrapping_mul(u64::from(*scale)));
                    }
                    self.t.insert(*t, v & mask(*abits));
                }
                Op::Div { bits, t, signed } => {
                    let d = self.temp(*t)?;
                    self.div(*bits, d, *signed)?;
                }
                Op::Cx16 { addr } => {
                    let a = self.temp(*addr)?;
                    self.cmpxchg16b(a)?;
                }
                Op::Rep { kind, size } => self.rep(kind, *size)?,
                Op::Mxcsr { store, loc } => {
                    if *store {
                        let v = u64::from(self.th.mxcsr);
                        self.store(*loc, v)?;
                    } else {
                        let v = self.load(*loc)?;
                        if v & !0xFFFF != 0 {
                            return Err(guest(format!(
                                "#GP: LDMXCSR 0x{v:x} sets reserved bits at 0x{:x}",
                                ins.address
                            )));
                        }
                        self.th.mxcsr = v as u32;
                    }
                }
                Op::Xgetbv => {
                    if self.th.regs[usize::from(RCX)] & 0xFFFF_FFFF != 0 {
                        return Err(guest(format!(
                            "#GP: XGETBV ECX 0x{:x} at 0x{:x}",
                            self.th.regs[usize::from(RCX)] & 0xFFFF_FFFF,
                            ins.address
                        )));
                    }
                    self.th.regs[usize::from(RAX)] = XCR0;
                    self.th.regs[usize::from(RDX)] = 0;
                }
                Op::Fcw { loc } => {
                    let v = u64::from(self.th.fcw);
                    self.store(*loc, v)?;
                }
                Op::Fence { .. } | Op::Pause => {}
                Op::Ud => {
                    return Err(Fault::Guest {
                        code: STATUS_ILLEGAL_INSTRUCTION,
                        why: format!("#UD: {} at 0x{:x}", ins.iclass, ins.address),
                    });
                }
                Op::Ld { t, loc } => {
                    let v = self.load(*loc)?;
                    self.t.insert(*t, v);
                }
                Op::St { loc, t } => {
                    let v = self.temp(*t)?;
                    self.store(*loc, v)?;
                }
                Op::Alu { t, op, a, b, bits } => {
                    let v = self.alu(op, self.temp(*a)?, self.temp(*b)?, *bits)?;
                    self.t.insert(*t, v & mask(*bits));
                }
                Op::Flags { kind, a, b, r, bits } => {
                    let (a, b, r) = (self.temp(*a)?, self.temp(*b)?, self.temp(*r)?);
                    self.set_flags(kind, a, b, r, *bits);
                }
                Op::Set { cond, loc } => {
                    let v = u64::from(self.cond(cond)?);
                    self.store(*loc, v)?;
                }
                Op::Df { set } => self.th.df = *set != 0,
                Op::Sel { t, c, x, y } => {
                    let v = if self.temp(*c)? != 0 {
                        self.temp(*x)?
                    } else {
                        self.temp(*y)?
                    };
                    self.t.insert(*t, v);
                }
                Op::Cst { c, loc, t } | Op::Cstz { c, loc, t } => {
                    let when = matches!(op, Op::Cst { .. });
                    if (self.temp(*c)? != 0) == when {
                        let v = self.temp(*t)?;
                        self.store(*loc, v)?;
                    }
                }
                Op::Cmov { cond, loc, t } => {
                    if self.cond(cond)? {
                        let v = self.temp(*t)?;
                        self.store(*loc, v)?;
                    } else if let Loc::R { idx, bits: 32, .. } = loc {
                        self.th.regs[usize::from(*idx)] &= 0xFFFF_FFFF;
                    }
                }
                Op::Cpuid => {
                    return Err(Fault::Refused(
                        "cpuid: the model's CPUID table comes with a later slice".into(),
                    ));
                }
                // `_sse` (runtime.py 398-496; Sprint 178)
                Op::Sse {
                    ic,
                    dst,
                    src,
                    align,
                    imm,
                } => self.sse(ic, *dst, *src, *align, *imm)?,
                // VEX: `_vex`, `_vex_cross`, `_fma` (runtime.py 668-827, 1044-1051; Sprint 183)
                Op::Vex {
                    base,
                    d,
                    s1,
                    s2,
                    vl,
                    align,
                    imm,
                } => self.vex(base, *d, *s1, *s2, *vl != 0, *align, *imm)?,
                Op::Vexs { ic, d, s1, s2, vl, imm } => self.vex_cross(ic, *d, *s1, *s2, *vl != 0, *imm)?,
                Op::Fma { ic, d, s2, s3, vl } => self.fma(ic, *d, *s2, *s3, *vl != 0)?,
                Op::Vzeroupper => self.th.ymm_hi = [0; 16],
                // `_mmx` and `_x87` (runtime.py 140-191), from the one lifter's ops (Sprint 177)
                Op::Mmx { ic, dst, src, imm } => self.mmx(ic, *dst, *src, *imm)?,
                Op::X87 {
                    ic,
                    opcode,
                    reg,
                    st,
                    mem,
                } => self.x87(ic, *opcode, *reg, *st, *mem, ins.address)?,
                Op::X86 { ic, dst, srcs, bits } => {
                    let vals: Vec<u64> = srcs.iter().map(|s| self.temp(*s)).collect::<Result<_, _>>()?;
                    if let Some(target) = self.integer(ic, *dst, &vals, *bits)? {
                        self.th.rip = target;
                        return Ok(());
                    }
                }
                Op::Br { cond, t } => {
                    if self.cond(cond)? {
                        self.th.rip = self.temp(*t)?;
                        return Ok(());
                    }
                }
            }
        }
        self.th.rip = nxt;
        Ok(())
    }

    /// `integer_model._zero_count_write` (43-47): a count of 0 changes no flag, but a 32-bit register destination is
    /// still written, its upper half zeroed (set 050 on the Xeon 8573C, run 38122967607)
    fn zero_count_write(&mut self, dst: Option<Loc>, a: u64) -> Result<(), Fault> {
        if let Some(loc @ Loc::R { bits: 32, .. }) = dst {
            self.store(loc, a)?;
        }
        Ok(())
    }

    /// `integer_model.execute` (50-157): the integer classes the model computes whole; a branch target when the
    /// instruction branches
    fn integer(&mut self, ic: &str, dst: Option<Loc>, vals: &[u64], bits: u32) -> Result<Option<u64>, Fault> {
        let (m, sign) = (mask(bits), 1u64 << (bits - 1));
        let need = |n: usize| -> Result<(), Fault> {
            if vals.len() < n {
                Err(Fault::Refused(format!("{ic}: {} sources, {n} needed", vals.len())))
            } else {
                Ok(())
            }
        };
        let store_dst = |s: &mut Self, v: u64| -> Result<(), Fault> {
            match dst {
                Some(loc) => s.store(loc, v),
                None => Err(Fault::Refused(format!("{ic}: no destination"))),
            }
        };
        match ic {
            "SHLD" | "SHRD" => {
                need(3)?;
                let (a, b) = (vals[0] & m, vals[1] & m);
                let count = vals[2] & if bits == 64 { 0x3F } else { 0x1F };
                if count == 0 {
                    self.zero_count_write(dst, a)?;
                    return Ok(None);
                }
                let (r, cf) = if bits == 16 && count > 16 {
                    // the SDM leaves it undefined; the Xeon 8573C shifts the 48 bits dst:src:dst (set 050, run
                    // 38122967607)
                    let v = u128::from(a) << 32 | u128::from(b) << 16 | u128::from(a);
                    if ic == "SHLD" {
                        (((v << count >> 32) as u64) & m, (v >> (48 - count)) & 1 == 1)
                    } else {
                        (((v >> count) as u64) & m, (v >> (count - 1)) & 1 == 1)
                    }
                } else if ic == "SHLD" {
                    // the high half of (dst:src) << count
                    let wide = (u128::from(a) << bits | u128::from(b)) << count;
                    (((wide >> bits) as u64) & m, (a >> (u64::from(bits) - count)) & 1 == 1)
                } else {
                    // the low half of (src:dst) >> count
                    let wide = (u128::from(b) << bits | u128::from(a)) >> count;
                    ((wide as u64) & m, (a >> (count - 1)) & 1 == 1)
                };
                store_dst(self, r)?;
                let f = &mut self.th.flags;
                (f.c, f.z, f.s, f.p, f.a) = (Some(cf), Some(r == 0), Some(r & sign != 0), Some(parity(r)), None);
                f.o = if count == 1 {
                    Some((r & sign != 0) != (a & sign != 0))
                } else {
                    None
                };
                Ok(None)
            }
            "RCR" => {
                need(2)?;
                let a = vals[0] & m;
                let masked = vals[1] & if bits == 64 { 0x3F } else { 0x1F };
                let count = match bits {
                    8 => masked % 9,
                    16 => masked % 17,
                    _ => masked,
                };
                if masked == 0 {
                    self.zero_count_write(dst, a)?;
                    return Ok(None);
                }
                let carry = u128::from(self.flag('C')?);
                let of = if masked == 1 {
                    Some((a & sign != 0) != (carry != 0))
                } else {
                    None
                };
                // bits + 1 wide: CF above the operand
                let mut wide = u128::from(a) | carry << bits;
                for _ in 0..count {
                    wide = (wide >> 1) | (wide & 1) << bits;
                }
                store_dst(self, (wide as u64) & m)?;
                self.th.flags.c = Some((wide >> bits) & 1 == 1);
                self.th.flags.o = of;
                Ok(None)
            }
            "STC" => {
                self.th.flags.c = Some(true);
                Ok(None)
            }
            "CLC" => {
                self.th.flags.c = Some(false);
                Ok(None)
            }
            "CMC" => {
                self.th.flags.c = Some(!self.flag('C')?);
                Ok(None)
            }
            "SAHF" => {
                let ah = self.th.regs[usize::from(RAX)] >> 8 & 0xFF;
                let f = &mut self.th.flags;
                (f.s, f.z, f.a, f.p, f.c) = (
                    Some(ah & 0x80 != 0),
                    Some(ah & 0x40 != 0),
                    Some(ah & 0x10 != 0),
                    Some(ah & 0x04 != 0),
                    Some(ah & 0x01 != 0),
                );
                Ok(None)
            }
            "PUSHFQ" => {
                // a flag the model holds undefined pushes as 0, IF (bit 9) and bit 1 as 1
                let f = self.th.flags;
                let mut v: u64 = 1 << 1 | 1 << 9 | u64::from(self.th.df) << 10;
                for (flag, bit) in [(f.c, 0), (f.p, 2), (f.a, 4), (f.z, 6), (f.s, 7), (f.o, 11)] {
                    if flag == Some(true) {
                        v |= 1 << bit;
                    }
                }
                let sp = self.th.regs[usize::from(RSP)].wrapping_sub(8);
                self.mem.write(sp, &v.to_le_bytes())?;
                self.th.regs[usize::from(RSP)] = sp;
                Ok(None)
            }
            "POPFQ" => {
                let sp = self.th.regs[usize::from(RSP)];
                let v = self.read_le(sp, 8)?;
                if v & (1 << 8 | 1 << 18 | 1 << 14 | 1 << 21) != 0 {
                    return Err(Fault::Refused(format!(
                        "POPFQ of 0x{v:x}: TF, AC, NT or ID set, not in the rows"
                    )));
                }
                let bit = |b: u32| Some(v >> b & 1 == 1);
                self.th.flags = Flags {
                    c: bit(0),
                    p: bit(2),
                    a: bit(4),
                    z: bit(6),
                    s: bit(7),
                    o: bit(11),
                };
                self.th.df = v >> 10 & 1 == 1;
                self.th.regs[usize::from(RSP)] = sp.wrapping_add(8);
                Ok(None)
            }
            "POPCNT" => {
                need(1)?;
                let v = vals[0] & m;
                store_dst(self, u64::from(v.count_ones()))?;
                let f = &mut self.th.flags;
                (f.z, f.c, f.o, f.s, f.a, f.p) = (
                    Some(v == 0),
                    Some(false),
                    Some(false),
                    Some(false),
                    Some(false),
                    Some(false),
                );
                Ok(None)
            }
            "LOOP" | "LOOPE" | "LOOPNE" => {
                need(1)?;
                let rcx = self.th.regs[usize::from(RCX)].wrapping_sub(1);
                self.th.regs[usize::from(RCX)] = rcx;
                let taken = rcx != 0 && (ic == "LOOP" || self.flag('Z')? == (ic == "LOOPE"));
                Ok(taken.then_some(vals[0]))
            }
            "JRCXZ" => {
                need(1)?;
                Ok((self.th.regs[usize::from(RCX)] == 0).then_some(vals[0]))
            }
            "LODSB" | "STOSB" | "MOVSB" | "MOVSD" | "SCASB" | "SCASD" | "CMPSB" | "CMPSD" => {
                let (kind, size): (&str, u64) = match ic {
                    "LODSB" => ("lods", 1),
                    "STOSB" => ("stos", 1),
                    "MOVSB" => ("movs", 1),
                    "MOVSD" => ("movs", 4),
                    "SCASB" => ("scas", 1),
                    "SCASD" => ("scas", 4),
                    "CMPSB" => ("cmps", 1),
                    _ => ("cmps", 4),
                };
                let n = size as usize;
                let step = if self.th.df { size.wrapping_neg() } else { size };
                let mk = mask(8 * size as u32);
                let (rsi, rdi) = (self.th.regs[usize::from(RSI)], self.th.regs[usize::from(RDI)]);
                match kind {
                    "lods" => {
                        let v = self.read_le(rsi, n)?;
                        let rax = self.th.regs[usize::from(RAX)];
                        self.th.regs[usize::from(RAX)] = if size < 4 { rax & !mk | v } else { v };
                        self.th.regs[usize::from(RSI)] = rsi.wrapping_add(step);
                    }
                    "stos" => {
                        let v = self.th.regs[usize::from(RAX)] & mk;
                        self.mem.write(rdi, &v.to_le_bytes()[..n])?;
                        self.th.regs[usize::from(RDI)] = rdi.wrapping_add(step);
                    }
                    "movs" => {
                        let b = self.mem.read(rsi, n)?;
                        self.mem.write(rdi, &b)?;
                        self.th.regs[usize::from(RSI)] = rsi.wrapping_add(step);
                        self.th.regs[usize::from(RDI)] = rdi.wrapping_add(step);
                    }
                    _ => {
                        let a = if kind == "scas" {
                            self.th.regs[usize::from(RAX)] & mk
                        } else {
                            self.read_le(rsi, n)?
                        };
                        let b = self.read_le(rdi, n)?;
                        let r = a.wrapping_sub(b) & mk;
                        let w = 8 * size;
                        let f = &mut self.th.flags;
                        f.c = Some(a < b);
                        f.z = Some(r == 0);
                        f.s = Some(r >> (w - 1) & 1 == 1);
                        f.p = Some(parity(r));
                        f.a = Some((a ^ b ^ r) & 0x10 != 0);
                        f.o = Some(((a ^ b) & (a ^ r)) >> (w - 1) & 1 == 1);
                        if kind == "cmps" {
                            self.th.regs[usize::from(RSI)] = rsi.wrapping_add(step);
                        }
                        self.th.regs[usize::from(RDI)] = rdi.wrapping_add(step);
                    }
                }
                Ok(None)
            }
            _ => Err(Fault::Refused(format!("integer_model: {ic}"))),
        }
    }

    /// The `alu` operations (1070-1123)
    fn alu(&self, o: &str, x: u64, y: u64, bits: u32) -> Result<u64, Fault> {
        let m = mask(bits);
        let count = y & if bits == 64 { 0x3F } else { 0x1F };
        Ok(match o {
            "add" => x.wrapping_add(y),
            "sub" => x.wrapping_sub(y),
            "and" => x & y,
            "or" => x | y,
            "xor" => x ^ y,
            "shr" => (x & m) >> count,
            "mul_lo" => x.wrapping_mul(y),
            "mul_hi" => ((u128::from(x) * u128::from(y)) >> 64) as u64,
            "sext32" => {
                if x & 0x8000_0000 != 0 {
                    x.wrapping_sub(1 << 32)
                } else {
                    x
                }
            }
            "sext8" | "sext16" => {
                let w = if o == "sext8" { 8 } else { 16 };
                let xw = x & mask(w);
                if xw >> (w - 1) != 0 {
                    xw.wrapping_sub(1 << w)
                } else {
                    xw
                }
            }
            "bswap" => {
                let n = (bits / 8) as usize;
                let b = (x & m).to_le_bytes();
                let mut out = 0u64;
                for (i, byte) in b[..n].iter().enumerate() {
                    out |= u64::from(*byte) << (8 * (n - 1 - i));
                }
                out
            }
            "popcnt" => u64::from((x & m).count_ones()),
            "sbb" => x.wrapping_sub(y).wrapping_sub(u64::from(self.flag('C')?)),
            "adc" => x.wrapping_add(y).wrapping_add(u64::from(self.flag('C')?)),
            "bsf" | "bsr" => {
                let xm = x & m;
                if xm == 0 {
                    0
                } else if o == "bsf" {
                    u64::from(xm.trailing_zeros())
                } else {
                    63 - u64::from(xm.leading_zeros())
                }
            }
            "shl" => {
                if count >= 64 {
                    0
                } else {
                    (x & m) << count
                }
            }
            "sar" => {
                let xm = x & m;
                let sx = signed(xm, bits);
                (sx >> count) as u64
            }
            "rol" | "ror" => {
                let xm = x & m;
                let k = count % u64::from(bits);
                if k == 0 {
                    xm
                } else if o == "rol" {
                    (xm << k) | (xm >> (u64::from(bits) - k))
                } else {
                    (xm >> k) | (xm << (u64::from(bits) - k))
                }
            }
            "smul_lo" | "smul_hi" => {
                let p = signed(x & m, bits) * signed(y & m, bits);
                if o == "smul_lo" { p as u64 } else { (p >> bits) as u64 }
            }
            _ => return Err(Fault::Refused(format!("alu {o}"))),
        })
    }
}

/// A value of `bits` bits as signed.
fn signed(v: u64, bits: u32) -> i128 {
    if bits < 64 && v >> (bits - 1) & 1 == 1 {
        v as i128 - (1i128 << bits)
    } else if bits == 64 {
        i128::from(v as i64)
    } else {
        v as i128
    }
}

const M64_128: u128 = u64::MAX as u128;
/// bits 79:64 of a register an MMX instruction writes (mmx.py 22)
const ONES: u128 = 0xFFFF << 64;
/// FSW.ES, an x87 exception pending (x87.py 35)
const ES: u16 = 128;

fn lanes(v: u64, bits: u32) -> Vec<u64> {
    (0..64 / bits).map(|i| v >> (i * bits) & mask(bits)).collect()
}

fn join(xs: &[i128], bits: u32) -> u64 {
    xs.iter().enumerate().fold(0u64, |acc, (i, x)| {
        acc | ((*x as u64) & mask(bits)) << (i as u32 * bits)
    })
}

fn clamp(v: i128, lo: i128, hi: i128) -> i128 {
    v.clamp(lo, hi)
}

/// `mmx.compute` (45-80): the 64-bit result of a lane operation on destination d and source s.
fn mmx_compute(ic: &str, d: u64, s: u64, count: u64) -> Result<u64, Fault> {
    let sg = |x: u64, bits: u32| signed(x, bits);
    let pair = |bits: u32| -> Vec<(u64, u64)> { lanes(d, bits).into_iter().zip(lanes(s, bits)).collect() };
    Ok(match ic {
        "PXOR" => d ^ s,
        "PADDW" | "PADDD" | "PSUBW" => {
            let bits = if ic == "PADDD" { 32 } else { 16 };
            let v: Vec<i128> = pair(bits)
                .iter()
                .map(|(x, y)| {
                    if ic == "PSUBW" {
                        *x as i128 - *y as i128
                    } else {
                        *x as i128 + *y as i128
                    }
                })
                .collect();
            join(&v, bits)
        }
        "PADDSW" => {
            let v: Vec<i128> = pair(16)
                .iter()
                .map(|(x, y)| clamp(sg(*x, 16) + sg(*y, 16), -0x8000, 0x7FFF))
                .collect();
            join(&v, 16)
        }
        "PMULLW" | "PMULHW" => {
            let shift = if ic == "PMULHW" { 16 } else { 0 };
            let v: Vec<i128> = pair(16)
                .iter()
                .map(|(x, y)| (sg(*x, 16) * sg(*y, 16)) >> shift)
                .collect();
            join(&v, 16)
        }
        "PMADDUBSW" => {
            // DEST's bytes unsigned, SRC's signed; adjacent products summed
            let (db, sb): (Vec<i128>, Vec<i128>) = (
                lanes(d, 8).iter().map(|x| *x as i128).collect(),
                lanes(s, 8).iter().map(|x| sg(*x, 8)).collect(),
            );
            let v: Vec<i128> = (0..4)
                .map(|i| clamp(db[2 * i] * sb[2 * i] + db[2 * i + 1] * sb[2 * i + 1], -0x8000, 0x7FFF))
                .collect();
            join(&v, 16)
        }
        "PACKUSWB" => {
            let v: Vec<i128> = lanes(d, 16)
                .into_iter()
                .chain(lanes(s, 16))
                .map(|x| clamp(sg(x, 16), 0, 0xFF))
                .collect();
            join(&v, 8)
        }
        _ if ic.starts_with("PUNPCK") => {
            let bits = match &ic[ic.len() - 2..] {
                "BW" => 8,
                "WD" => 16,
                _ => 32,
            };
            let half = (32 / bits) as usize;
            let (a, b) = (lanes(d, bits), lanes(s, bits));
            let lo = ic.as_bytes()[6] == b'L';
            let (a, b) = if lo {
                (&a[..half], &b[..half])
            } else {
                (&a[half..], &b[half..])
            };
            let v: Vec<i128> = a.iter().zip(b).flat_map(|(x, y)| [*x as i128, *y as i128]).collect();
            join(&v, bits)
        }
        "PSRAW" | "PSRAD" => {
            let bits = if ic == "PSRAW" { 16 } else { 32 };
            let n = count.min(u64::from(bits) - 1);
            let v: Vec<i128> = lanes(d, bits).iter().map(|x| sg(*x, bits) >> n).collect();
            join(&v, bits)
        }
        "PSRLQ" => {
            if count > 63 {
                0
            } else {
                d >> count
            }
        }
        // the sum of |d - s| over the 8 bytes, in bits 15:0
        "PSADBW" => lanes(d, 8)
            .iter()
            .zip(lanes(s, 8))
            .map(|(x, y)| (*x as i64 - y as i64).unsigned_abs())
            .sum(),
        // word i of the result is the source's word imm[2i+1:2i]
        "PSHUFW" => {
            let w = lanes(s, 16);
            let v: Vec<i128> = (0..4).map(|i| w[(count >> (2 * i) & 3) as usize] as i128).collect();
            join(&v, 16)
        }
        _ => return Err(Fault::Refused(format!("MMX {ic} is not modelled"))),
    })
}

impl<M: Memory> Step<'_, M> {
    fn mmx_get(&self, o: MmxArg) -> Result<u64, Fault> {
        match o {
            MmxArg::Mm(n) => Ok((self.th.x87_regs[usize::from(n)] & M64_128) as u64),
            MmxArg::X(n) => Ok(self.th.xmm[usize::from(n)] as u64),
            MmxArg::Loc(loc) => self.load(loc),
            MmxArg::Imm | MmxArg::None => Err(Fault::Refused("an MMX operand that holds no value".into())),
        }
    }

    fn mmx_put(&mut self, o: MmxArg, v: u64) -> Result<(), Fault> {
        match o {
            MmxArg::Mm(n) => {
                self.th.x87_regs[usize::from(n)] = ONES | u128::from(v);
                Ok(())
            }
            MmxArg::Loc(loc) => self.store(loc, v),
            _ => Err(Fault::Refused("an MMX destination the model does not write".into())),
        }
    }

    /// TOP 0 after the reads, before the write (set 065b, Xeon 8573C: a faulting load leaves TOP, a faulting store has
    /// set it); EMMS also marks every register empty
    fn mmx_transition(&mut self, ic: &str) {
        self.th.x87_sw &= !(7 << 11);
        if ic == "EMMS" {
            self.th.x87_tags = 0;
        }
    }

    /// `mmx.execute` (83-121)
    fn mmx(&mut self, ic: &str, dst: MmxArg, src: MmxArg, imm: u64) -> Result<(), Fault> {
        if self.th.x87_sw & ES != 0 {
            return Err(Fault::Refused(format!(
                "{ic} with an x87 exception pending (status word 0x{:x})",
                self.th.x87_sw
            )));
        }
        if ic == "EMMS" {
            self.mmx_transition(ic);
            return Ok(());
        }
        match ic {
            "MOVD" | "MOVQ" | "MOVDQ2Q" => {
                let v = self.mmx_get(src)?;
                self.mmx_transition(ic);
                self.mmx_put(dst, if ic == "MOVD" { v & 0xFFFF_FFFF } else { v })?;
            }
            "PEXTRW" => {
                let v = self.mmx_get(src)? >> (16 * (imm & 3)) & 0xFFFF;
                self.mmx_transition(ic);
                self.mmx_put(dst, v)?;
            }
            _ => {
                let shift = matches!(ic, "PSRAW" | "PSRAD" | "PSRLQ");
                let count = if shift {
                    if src == MmxArg::Imm { imm } else { self.mmx_get(src)? }
                } else {
                    imm
                };
                let s = if shift { 0 } else { self.mmx_get(src)? };
                let v = mmx_compute(ic, self.mmx_get(dst)?, s, count)?;
                self.mmx_transition(ic);
                self.mmx_put(dst, v)?;
            }
        }
        // the tags valid once the write is done (a faulting store keeps them)
        self.th.x87_tags = 0xFF;
        Ok(())
    }
}

/// The memory operand of an x87 step: `_x87`'s `read` and `write` (runtime.py 149-153).
struct X87Operand<'a, M: Memory> {
    mem: &'a mut M,
    address: u64,
}

impl<M: Memory> crate::x87::X87Memory for X87Operand<'_, M> {
    fn read(&mut self, n: usize) -> Result<u128, Fault> {
        let b = self.mem.read(self.address, n)?;
        let mut w = [0u8; 16];
        w[..n].copy_from_slice(&b[..n]);
        Ok(u128::from_le_bytes(w))
    }

    fn write(&mut self, n: usize, v: u128, offset: u64) -> Result<(), Fault> {
        self.mem.write(self.address.wrapping_add(offset), &v.to_le_bytes()[..n])
    }
}

impl<M: Memory> Step<'_, M> {
    /// `_x87` (runtime.py 140-163): the register stack from the thread, one instruction, and the stack back on success
    /// only, so a guest fault leaves the x87 state as it was. An exception the control word does not mask is refused by
    /// name.
    fn x87(&mut self, ic: &str, opcode: u8, reg: u8, st: Option<u8>, mem: Option<Loc>, at: u64) -> Result<(), Fault> {
        let address = match mem {
            Some(Loc::M { addr, .. }) => self.temp(addr)?,
            _ => 0,
        };
        let th = &mut *self.th;
        let mut fpu = crate::x87::Fpu::new(th.fcw, th.x87_sw, th.x87_regs, th.x87_tags);
        let mut ax = th.regs[0] as u16;
        let mut operand = X87Operand {
            mem: &mut *self.mem,
            address,
        };
        crate::x87::execute(&mut fpu, ic, opcode, reg, st, &mut operand, &mut ax).map_err(|e| match e {
            Fault::Refused(why) => Fault::Refused(format!("{ic} at 0x{at:x}: {why}")),
            e => e,
        })?;
        (th.fcw, th.x87_sw, th.x87_regs, th.x87_tags) = (fpu.cw, fpu.sw, fpu.regs, fpu.abridged_tags());
        if ic == "FNSTSW" && mem.is_none() {
            th.regs[0] = th.regs[0] & !0xFFFF | u64::from(ax);
        }
        Ok(())
    }
}

/// `SSE_049B` (runtime.py 32-33): the set-049b legacy forms.
/// `SHIFT_BY_COUNT` (runtime.py 34): at VEX.256 one count shifts both halves.
const SHIFT_BY_COUNT: [&str; 7] = ["PSRLQ", "PSLLQ", "PSRLD", "PSLLD", "PSRAD", "PSRLW", "PSRAW"];

/// `VEX_FP` (runtime.py 35): FP bases beyond ADD..MAX, whose two VEX.256 halves are checked once.
const VEX_FP: [&str; 8] = [
    "HADDPD",
    "CVTTPD2DQ",
    "CVTPD2DQ",
    "CMPPS_VEX",
    "CVTDQ2PS",
    "CVTTPS2DQ",
    "CVTPS2PD",
    "ROUNDSD",
];

const SSE_049B: [&str; 14] = [
    "PSHUFB",
    "PALIGNR",
    "PINSRD",
    "PINSRQ",
    "PEXTRQ",
    "PMOVSXDQ",
    "ANDNPD",
    "CVTDQ2PS",
    "CVTTPS2DQ",
    "CVTPS2PD",
    "CMPPS_VEX",
    "ROUNDSD",
    "PERMILPS_VAR",
    "PBLENDD",
];

/// `SSE_050_NEW` (runtime.py 95-96): the set-050 legacy classes.
const SSE_050_NEW: [&str; 19] = [
    "CVTPS2DQ",
    "CVTPD2PS",
    "CMPPS",
    "CMPPD",
    "CMPSS",
    "SHUFPD",
    "MOVLPS",
    "MOVHPD",
    "MOVNTDQ",
    "PSADBW",
    "PACKSSWB",
    "PACKSSDW",
    "PMADDUBSW",
    "PMADDWD",
    "PEXTRW",
    "PINSRB",
    "PMOVZXWD",
    "PSLLW",
    "MPSADBW",
];

/// `SSE2_INT` (lift.py 108-125): the classes `_sse` sends to `_sse2_int`, as the Python's set holds them (62).
const SSE2_INT: [&str; 62] = [
    "ANDNPS",
    "CVTDQ2PD",
    "MOVDQA",
    "MOVDQU",
    "MOVHLPS",
    "MOVHPS",
    "MOVLHPS",
    "MOVMSKPS",
    "MOVNTPS",
    "ORPS",
    "PACKUSWB",
    "PADDD",
    "PADDQ",
    "PADDSB",
    "PADDSW",
    "PADDW",
    "PAND",
    "PANDN",
    "PCMPEQB",
    "PCMPEQD",
    "PCMPGTD",
    "PCMPISTRI",
    "PCMPISTRI64",
    "PINSRW",
    "PMAXUB",
    "PMOVMSKB",
    "PMULLW",
    "POR",
    "PSHUFD",
    "PSHUFHW",
    "PSHUFLW",
    "PSLLD",
    "PSLLDQ",
    "PSLLQ",
    "PSRAD",
    "PSRAW",
    "PSRLD",
    "PSRLDQ",
    "PSRLQ",
    "PSRLW",
    "PSUBD",
    "PSUBQ",
    "PSUBSB",
    "PSUBUSB",
    "PSUBW",
    "PUNPCKHBW",
    "PUNPCKHDQ",
    "PUNPCKHWD",
    "PUNPCKLBW",
    "PUNPCKLDQ",
    "PUNPCKLQDQ",
    "PUNPCKLWD",
    "PXOR",
    "RCPPS",
    "RCPSS",
    "RSQRTPS",
    "RSQRTSS",
    "SHUFPS",
    "UNPCKHPD",
    "UNPCKHPS",
    "UNPCKLPD",
    "UNPCKLPS",
];

/// A lane of dest p and source q at width w (`LANE_ARITH`'s lambdas).
type LaneFn = fn(u64, u64, u32) -> i128;

/// `LANE_ARITH` (runtime.py 69-94): lane by lane, (lane bits, the lane of dest p and source q).
fn lane_arith(ic: &str) -> Option<(u32, LaneFn)> {
    Some(match ic {
        "PADDW" => (16, |p, q, _| i128::from(p) + i128::from(q)),
        "PADDQ" => (64, |p, q, _| i128::from(p) + i128::from(q)),
        "PSUBW" => (16, |p, q, _| i128::from(p) - i128::from(q)),
        "PCMPEQD" => (32, |p, q, w| if p == q { i128::from(mask(w)) } else { 0 }),
        "PCMPGTD" | "PCMPGTB" | "PCMPGTW" => (
            match ic {
                "PCMPGTD" => 32,
                "PCMPGTB" => 8,
                _ => 16,
            },
            |p, q, w| {
                if signed(p, w) > signed(q, w) {
                    i128::from(mask(w))
                } else {
                    0
                }
            },
        ),
        "PSUBUSB" => (8, |p, q, _| (i128::from(p) - i128::from(q)).max(0)),
        "PMAXUB" => (8, |p, q, _| i128::from(p.max(q))),
        "PADDSB" => (8, |p, q, w| {
            (signed(p, w) + signed(q, w)).clamp(-(1i128 << (w - 1)), (1i128 << (w - 1)) - 1)
        }),
        "PADDSW" => (16, |p, q, w| {
            (signed(p, w) + signed(q, w)).clamp(-(1i128 << (w - 1)), (1i128 << (w - 1)) - 1)
        }),
        "PSUBSB" => (8, |p, q, w| {
            (signed(p, w) - signed(q, w)).clamp(-(1i128 << (w - 1)), (1i128 << (w - 1)) - 1)
        }),
        "PMULLW" => (16, |p, q, _| i128::from(p) * i128::from(q)),
        "PADDB" => (8, |p, q, _| i128::from(p) + i128::from(q)),
        "PCMPEQQ" => (64, |p, q, w| if p == q { i128::from(mask(w)) } else { 0 }),
        "PMULUDQ" => (64, |p, q, _| i128::from(p & 0xFFFF_FFFF) * i128::from(q & 0xFFFF_FFFF)),
        "PADDUSB" => (8, |p, q, _| (i128::from(p) + i128::from(q)).min(0xFF)),
        "PAVGB" => (8, |p, q, _| (i128::from(p) + i128::from(q) + 1) >> 1),
        "PMAXUD" => (32, |p, q, _| i128::from(p.max(q))),
        "PMAXUW" => (16, |p, q, _| i128::from(p.max(q))),
        "PMAXSW" => (16, |p, q, w| signed(p, w).max(signed(q, w))),
        "PMINSD" => (32, |p, q, w| signed(p, w).min(signed(q, w))),
        "PMULHW" => (16, |p, q, w| (signed(p, w) * signed(q, w)) >> 16),
        "PMULLD" => (32, |p, q, _| i128::from(p) * i128::from(q)),
        "PABSW" => (16, |_, q, w| signed(q, w).abs()),
        _ => return None,
    })
}

/// `INTERLEAVE` (runtime.py 97-98): (lane bits, from the high half).
fn interleave(ic: &str) -> Option<(u32, bool)> {
    Some(match ic {
        "PUNPCKHQDQ" | "UNPCKHPD" => (64, true),
        "PUNPCKLQDQ" | "UNPCKLPD" => (64, false),
        "PUNPCKHBW" => (8, true),
        "PUNPCKHDQ" => (32, true),
        "PUNPCKHWD" => (16, true),
        "PUNPCKLWD" => (16, false),
        _ => return None,
    })
}

/// lanes of a 128-bit value, `w` bits each (runtime.py's local `lanes`)
fn lanes128(v: u128, w: u32) -> Vec<u64> {
    crate::x86fp::lanes(v, w, 128 / w)
}

/// `pack`: lanes of `w` bits, each masked to `w`, from a list of possibly negative or wide values
fn pack(ls: &[i128], w: u32) -> u128 {
    ls.iter()
        .enumerate()
        .fold(0u128, |acc, (i, n)| acc | ((*n as u128) & mask128(w)) << (w * i as u32))
}

impl<M: Memory> Step<'_, M> {
    /// `_xread` (runtime.py 391-396)
    fn xread(&self, loc: SseLoc, nbytes: u32) -> Result<u128, Fault> {
        match loc {
            SseLoc::X(n) => Ok(self.th.xmm[usize::from(n)] & mask128(8 * nbytes)),
            SseLoc::Loc(Loc::R { idx, bits, hi }) => Ok(u128::from(self.rd(idx, bits, hi))),
            SseLoc::Loc(Loc::M { addr, .. }) => {
                // a VEX.256 operand's 32 bytes are read whole (a fault in bytes 16-31 stands); a 128-bit step uses
                // the low 16
                let b = self.mem.read(self.temp(addr)?, nbytes as usize)?;
                let n = (nbytes as usize).min(16);
                let mut w = [0u8; 16];
                w[..n].copy_from_slice(&b[..n]);
                Ok(u128::from_le_bytes(w))
            }
        }
    }

    /// `_xwrite` (runtime.py 965-971): an XMM write zeroes what `nbytes` does not cover
    fn xwrite(&mut self, loc: SseLoc, v: u128, nbytes: u32) -> Result<(), Fault> {
        match loc {
            SseLoc::X(n) => {
                self.th.xmm[usize::from(n)] = v & mask128(8 * nbytes);
                Ok(())
            }
            SseLoc::Loc(Loc::R { idx, bits, hi }) => {
                self.wr(idx, bits, hi, v as u64);
                Ok(())
            }
            SseLoc::Loc(Loc::M { addr, .. }) => {
                let va = self.temp(addr)?;
                self.mem.write(va, &v.to_le_bytes()[..nbytes as usize])
            }
        }
    }

    /// `_fp_check` (runtime.py 983-992) with `_slow_sse`'s fault code (vm_backend 4301-4302): an unmasked exception
    /// merges its flags into MXCSR and is the guest's #XM with the code `xm_status` picks; otherwise every flag merges
    fn fp_check(&mut self, env: &crate::x86fp::Env) -> Result<(), Fault> {
        if self.defer_fp {
            self.deferred_flags |= env.flags;
            return Ok(());
        }
        match env.check() {
            Ok(()) => {
                self.th.mxcsr |= env.flags;
                Ok(())
            }
            Err(e) => {
                self.th.mxcsr |= e.flags;
                Err(self.xm_fault())
            }
        }
    }

    fn xm_fault(&self) -> Fault {
        match crate::x86fp::xm_status(self.th.mxcsr) {
            Some(code) => Fault::Guest {
                code,
                why: format!("#XM: MXCSR 0x{:x}", self.th.mxcsr),
            },
            None => Fault::Refused(format!("#XM with no unmasked flag in MXCSR 0x{:x}", self.th.mxcsr)),
        }
    }

    fn xmm(&self, loc: SseLoc) -> Result<usize, Fault> {
        match loc {
            SseLoc::X(n) => Ok(usize::from(n)),
            _ => Err(Fault::Refused("an XMM register operand expected".into())),
        }
    }

    /// `_sse` (runtime.py 398-496): one legacy-SSE instruction, every read and every fault check before any write.
    fn sse(&mut self, ic: &str, dst: SseLoc, src: SseLoc, align: u32, imm: Option<u64>) -> Result<(), Fault> {
        use crate::x86fp::{self as fp, Env, F32, F64};
        for loc in [dst, src] {
            if let SseLoc::Loc(Loc::M { addr, .. }) = loc {
                let va = self.temp(addr)?;
                if align != 0 && va % u64::from(align) != 0 {
                    return Err(guest(format!(
                        "#GP: {ic} memory operand at 0x{va:x} not {align}-byte aligned"
                    )));
                }
            }
        }
        let msize = [dst, src]
            .iter()
            .find_map(|l| match l {
                SseLoc::Loc(Loc::M { bytes, .. }) => Some(*bytes),
                _ => None,
            })
            .unwrap_or(16);
        let imm_of = || imm.ok_or_else(|| Fault::Refused(format!("{ic} without its immediate")));
        if matches!(
            ic,
            "MOVAPS" | "MOVAPD" | "MOVUPS" | "MOVUPD" | "MOVDQA" | "MOVDQU" | "MOVNTPS" | "LDDQU"
        ) {
            let v = self.xread(src, 16)?;
            return self.xwrite(dst, v, 16);
        }
        if SSE_049B.contains(&ic) {
            return self.sse_049b(ic, dst, src, imm);
        }
        if SSE_050_NEW.contains(&ic) {
            return self.sse_050(ic, dst, src, imm);
        }
        if SSE2_INT.contains(&ic) || lane_arith(ic).is_some() || interleave(ic).is_some() {
            return self.sse2_int(ic, dst, src, imm);
        }
        match ic {
            "MOVSS" | "MOVSD_XMM" => {
                let n = if ic == "MOVSS" { 4 } else { 8 };
                let v = self.xread(src, n)?;
                if let (SseLoc::X(d), SseLoc::X(_)) = (dst, src) {
                    let d = usize::from(d);
                    self.th.xmm[d] = self.th.xmm[d] & !mask128(8 * n) | v; // register form merges
                } else {
                    self.xwrite(dst, v, n)?; // load zeroes the rest
                }
            }
            "MOVD" | "MOVQ" => {
                let mut n = if ic == "MOVD" { 4 } else { 8 };
                if let SseLoc::Loc(Loc::R { bits, .. }) = src {
                    n = bits / 8;
                } else if let SseLoc::Loc(Loc::R { bits, .. }) = dst {
                    n = bits / 8;
                }
                let v = self.xread(src, n)?;
                self.xwrite(dst, v, n)?; // to XMM: zero-extended
            }
            "XORPS" | "XORPD" | "ANDPS" | "ANDPD" | "ORPD" => {
                let d = self.xmm(dst)?;
                let (a, c) = (self.th.xmm[d], self.xread(src, 16)?);
                self.th.xmm[d] = if ic.starts_with("XOR") {
                    a ^ c
                } else if ic == "ORPD" {
                    a | c
                } else {
                    a & c
                };
            }
            "PCMPEQW" => {
                let d = self.xmm(dst)?;
                let (a, c) = (self.th.xmm[d], self.xread(src, 16)?);
                let ls: Vec<i128> = lanes128(a, 16)
                    .iter()
                    .zip(lanes128(c, 16))
                    .map(|(p, q)| if *p == q { 0xFFFF } else { 0 })
                    .collect();
                self.th.xmm[d] = pack(&ls, 16);
            }
            "CVTTPD2DQ" | "CVTPD2DQ" => {
                // two doubles to two int32; bits 127:64 zeroed
                let d = self.xmm(dst)?;
                let v = self.xread(src, 16)?;
                let mut env = Env::new(self.th.mxcsr);
                let words: Vec<u64> = fp::lanes(v, 64, 2)
                    .iter()
                    .map(|x| fp::to_int(F64, *x, 32, ic == "CVTTPD2DQ", &mut env))
                    .collect();
                self.fp_check(&env)?;
                self.th.xmm[d] = fp::join(&words, 32);
            }
            "CMPSD_XMM" | "CMPSD_XMM_VEX" => {
                // predicate on the low doubles; all ones or 0; legacy: imm8[2:0]; VEX: imm8[4:0] (SDM CMPSD)
                let d = self.xmm(dst)?;
                let mut env = Env::new(self.th.mxcsr);
                let b = self.xread(src, 8)? as u64;
                let pred = imm_of()? & if ic.ends_with("VEX") { 31 } else { 7 };
                let hit = fp::cmp_predicate(F64, self.th.xmm[d] as u64, b, pred, &mut env);
                self.fp_check(&env)?;
                self.th.xmm[d] = self.th.xmm[d] & !mask128(64) | if hit { mask128(64) } else { 0 };
            }
            "HADDPD" => {
                let d = self.xmm(dst)?;
                let (a, c) = (self.th.xmm[d], self.xread(src, 16)?);
                let mut env = Env::new(self.th.mxcsr);
                let mut flags = 0;
                let mut sums = Vec::new();
                for (p, q) in [(a as u64, (a >> 64) as u64), (c as u64, (c >> 64) as u64)] {
                    env.flags = 0; // each lane's flags, then all of them (as execute)
                    sums.push(fp::arith("add", F64, p, q, &mut env));
                    flags |= env.flags;
                }
                env.flags = flags;
                self.fp_check(&env)?;
                self.th.xmm[d] = u128::from(sums[0]) | u128::from(sums[1]) << 64;
            }
            _ if ic.len() > 2
                && matches!(
                    &ic[..ic.len() - 2],
                    "ADD" | "SUB" | "MUL" | "DIV" | "SQRT" | "MIN" | "MAX"
                ) =>
            {
                let d = self.xmm(dst)?;
                let v = self.xread(
                    src,
                    if matches!(src, SseLoc::Loc(Loc::M { .. })) {
                        msize
                    } else {
                        16
                    },
                )?;
                if self.defer_fp {
                    // VEX.256: flags of both halves, one check
                    let (out, flags) = fp::execute_unchecked(ic, self.th.xmm[d], v, self.th.mxcsr);
                    self.deferred_flags |= flags;
                    self.th.xmm[d] = out;
                    return Ok(());
                }
                match fp::execute(ic, self.th.xmm[d], v, self.th.mxcsr) {
                    Ok((out, mxcsr)) => {
                        self.th.mxcsr = mxcsr;
                        self.th.xmm[d] = out;
                    }
                    Err(e) => {
                        // `_fp`: the flags at an unmasked fault (oracle)
                        self.th.mxcsr |= e.flags;
                        return Err(self.xm_fault());
                    }
                }
            }
            "CVTTSS2SI" | "CVTTSD2SI" | "CVTSS2SI" | "CVTSD2SI" => {
                let f = if ic.contains("SS") { F32 } else { F64 };
                let v = self.xread(src, f.bits / 8)? as u64;
                let SseLoc::Loc(Loc::R { idx, bits, .. }) = dst else {
                    return Err(Fault::Refused(format!("{ic}: a general register destination expected")));
                };
                let mut env = Env::new(self.th.mxcsr);
                let r = fp::to_int(f, v, bits, ic.starts_with("CVTT"), &mut env);
                self.fp_check(&env)?;
                self.wr(idx, bits, false, r);
            }
            "CVTSI2SS" | "CVTSI2SD" => {
                let f = if ic.ends_with("SS") { F32 } else { F64 };
                let d = self.xmm(dst)?;
                let bits = match src {
                    SseLoc::Loc(Loc::R { bits, .. }) => bits,
                    SseLoc::Loc(Loc::M { bytes, .. }) => 8 * bytes,
                    SseLoc::X(_) => return Err(Fault::Refused(format!("{ic}: an integer source expected"))),
                };
                let v = self.xread(src, bits / 8)? as u64;
                let mut env = Env::new(self.th.mxcsr);
                let r = fp::from_int(f, v, bits, &mut env);
                self.fp_check(&env)?;
                self.th.xmm[d] = self.th.xmm[d] & !mask128(f.bits) | u128::from(r);
            }
            "CVTSS2SD" | "CVTSD2SS" => {
                let (s_f, d_f) = if ic == "CVTSS2SD" { (F32, F64) } else { (F64, F32) };
                let d = self.xmm(dst)?;
                let v = self.xread(src, s_f.bits / 8)? as u64;
                let mut env = Env::new(self.th.mxcsr);
                let r = fp::convert(s_f, d_f, v, &mut env);
                self.fp_check(&env)?;
                self.th.xmm[d] = self.th.xmm[d] & !mask128(d_f.bits) | u128::from(r);
            }
            "UCOMISS" | "UCOMISD" | "COMISS" | "COMISD" => {
                let f = if ic.ends_with("SS") { F32 } else { F64 };
                let d = self.xmm(dst)?;
                let a = self.th.xmm[d] as u64 & f.mask();
                let c = self.xread(src, f.bits / 8)? as u64;
                let mut env = Env::new(self.th.mxcsr);
                let (z, p, cf) = fp::compare(f, a, c, &mut env, ic.starts_with("COMI"));
                self.fp_check(&env)?;
                self.th.flags = Flags {
                    z: Some(z),
                    p: Some(p),
                    c: Some(cf),
                    o: Some(false),
                    s: Some(false),
                    a: Some(false),
                };
            }
            _ => return Err(Fault::Refused(format!("sse {ic}"))),
        }
        Ok(())
    }

    /// `n` bytes at a memory operand's address plus `offset`, little-endian, up to 32
    fn read_bytes(&self, loc: SseLoc, offset: u64, n: usize) -> Result<[u128; 2], Fault> {
        let Some(addr) = loc.addr() else {
            return Err(Fault::Refused("a memory operand expected".into()));
        };
        let b = self.mem.read(self.temp(addr)? + offset, n)?;
        let mut w = [0u8; 32];
        w[..n].copy_from_slice(&b[..n]);
        Ok([
            u128::from_le_bytes(w[..16].try_into().unwrap()),
            u128::from_le_bytes(w[16..].try_into().unwrap()),
        ])
    }

    /// `_vex_cross` (runtime.py 668-730): VEX instructions that move data between the 128-bit halves, on whole
    /// 256-bit values (low, high): VPERMQ, VPERMD, the broadcasts, VINSERT/VEXTRACT F/I128, VCVTPS2PD and VPMOVSXDQ
    /// at 256 bits, VZEROALL.
    fn vex_cross(
        &mut self,
        ic: &str,
        d: SseLoc,
        s1: Option<SseLoc>,
        s2: SseLoc,
        vl: bool,
        imm: Option<u64>,
    ) -> Result<(), Fault> {
        use crate::x86fp::{self as fp, Env, F32, F64};
        if ic == "VZEROALL" {
            self.th.xmm[..16].fill(0);
            self.th.ymm_hi = [0; 16];
            return Ok(());
        }
        let read = |s: &Self, loc: SseLoc, nbytes: u32| -> Result<[u128; 2], Fault> {
            match loc {
                SseLoc::X(n) => {
                    let (lo, hi) = (s.th.xmm[usize::from(n)], s.th.ymm_hi[usize::from(n)]);
                    Ok(if nbytes > 16 {
                        [lo, hi & mask128(8 * (nbytes - 16))]
                    } else {
                        [lo & mask128(8 * nbytes), 0]
                    })
                }
                _ => s.read_bytes(loc, 0, nbytes as usize),
            }
        };
        let lanes256 = |v: [u128; 2], w: u32| -> Vec<u64> { [lanes128(v[0], w), lanes128(v[1], w)].concat() };
        let pack256 = |ls: &[u64], w: u32| -> [u128; 2] {
            let per = (128 / w) as usize;
            let half = |part: &[u64]| fp::join(part, w);
            [half(&ls[..per.min(ls.len())]), half(&ls[per.min(ls.len())..])]
        };
        let imm_of = || imm.ok_or_else(|| Fault::Refused(format!("{ic} without its immediate")));
        let mut width = if vl { 32 } else { 16 };
        let out = match ic {
            "VPERMQ" => {
                let (q, imm) = (lanes256(read(self, s2, 32)?, 64), imm_of()?);
                pack256(
                    &(0..4).map(|i| q[(imm >> (2 * i) & 3) as usize]).collect::<Vec<_>>(),
                    64,
                )
            }
            "VPERMD" => {
                let s1 = s1.ok_or_else(|| Fault::Refused("VPERMD without its index operand".into()))?;
                let (idx, src) = (lanes256(read(self, s1, 32)?, 32), lanes256(read(self, s2, 32)?, 32));
                pack256(&idx.iter().map(|i| src[(i & 7) as usize]).collect::<Vec<_>>(), 32)
            }
            "VPBROADCASTD" | "VBROADCASTSS" => {
                let v = read(self, s2, 4)?[0] as u64;
                pack256(&vec![v; width / 4], 32)
            }
            "VPBROADCASTQ" => {
                let v = read(self, s2, 8)?[0] as u64;
                pack256(&vec![v; width / 8], 64)
            }
            "VBROADCASTF128" => {
                let v = read(self, s2, 16)?[0];
                [v, v]
            }
            "VINSERTF128" | "VINSERTI128" => {
                let s1 = s1.ok_or_else(|| Fault::Refused(format!("{ic} without its base operand")))?;
                let (mut base, part) = (read(self, s1, 32)?, read(self, s2, 16)?[0]);
                base[(imm_of()? & 1) as usize] = part;
                base
            }
            "VEXTRACTF128" | "VEXTRACTI128" => {
                width = 16;
                [read(self, s2, 32)?[(imm_of()? & 1) as usize], 0]
            }
            "VCVTPS2PD" => {
                let v = read(self, s2, 16)?[0];
                let mut env = Env::new(self.th.mxcsr);
                let mut flags = 0;
                let mut res = Vec::new();
                for f32 in lanes128(v, 32) {
                    env.flags = 0;
                    res.push(fp::convert(F32, F64, f32, &mut env));
                    flags |= env.flags;
                }
                env.flags = flags;
                self.fp_check(&env)?;
                pack256(&res, 64)
            }
            "VPMOVSXDQ" => {
                let ls: Vec<u64> = lanes128(read(self, s2, 16)?[0], 32)
                    .iter()
                    .map(|w| i64::from(*w as u32 as i32) as u64)
                    .collect();
                pack256(&ls, 64)
            }
            _ => return Err(Fault::Refused(format!("vex {ic}"))),
        };
        match d {
            SseLoc::X(n) => {
                let n = usize::from(n);
                self.th.xmm[n] = out[0];
                self.th.ymm_hi[n] = if width == 32 { out[1] } else { 0 };
                Ok(())
            }
            _ => {
                let Some(addr) = d.addr() else {
                    return Err(Fault::Refused(format!(
                        "{ic}: a register or memory destination expected"
                    )));
                };
                let va = self.temp(addr)?;
                let bytes = [out[0].to_le_bytes(), out[1].to_le_bytes()].concat();
                self.mem.write(va, &bytes[..width])
            }
        }
    }

    /// `_vex` (runtime.py 732-786): one VEX instruction as its legacy-SSE base: d = BASE(s1, s2) on the low 128 bits,
    /// the same on bits 255:128 for VEX.256; a VEX.128 write to a register zeroes its bits 255:128 (SDM Vol 1 14.1.1).
    /// XMM 16 and 17 are the scratch (an operand that aliases the destination, the high halves). A memory operand of
    /// 32 bytes is read whole before any write.
    #[allow(clippy::too_many_arguments)]
    fn vex(
        &mut self,
        base: &str,
        d: SseLoc,
        s1: Option<SseLoc>,
        s2: SseLoc,
        vl: bool,
        align: u32,
        imm: Option<u64>,
    ) -> Result<(), Fault> {
        for loc in [d, s2] {
            if let Some(addr) = loc.addr() {
                let va = self.temp(addr)?;
                if align != 0 && va % u64::from(align) != 0 {
                    return Err(guest(format!(
                        "#GP: V{base} memory operand at 0x{va:x} not {align}-byte aligned"
                    )));
                }
            }
        }
        let mut src_hi = 0u128;
        if vl {
            src_hi = match s2 {
                SseLoc::X(n) => self.th.ymm_hi[usize::from(n)],
                SseLoc::Loc(Loc::M { .. }) => self.read_bytes(s2, 16, 16)?[0],
                SseLoc::Loc(_) => 0,
            };
        }
        if vl && SHIFT_BY_COUNT.contains(&base) && imm.is_none() {
            // the count is s2's low quadword, both halves
            src_hi = match s2 {
                SseLoc::X(n) => self.th.xmm[usize::from(n)],
                _ => self.read_bytes(s2, 0, 16)?[0],
            };
        }
        let first_hi = match (vl, s1, d) {
            (true, Some(SseLoc::X(n)), _) | (true, _, SseLoc::X(n)) => self.th.ymm_hi[usize::from(n)],
            _ => 0,
        };
        if vl && let Some(addr) = d.addr() {
            // a 256-bit store: all 32 bytes or none
            let SseLoc::X(s) = s2 else {
                return Err(Fault::Refused(format!(
                    "V{base}: a 256-bit store from a register expected"
                )));
            };
            let s = usize::from(s);
            let va = self.temp(addr)?;
            let bytes = [self.th.xmm[s].to_le_bytes(), self.th.ymm_hi[s].to_le_bytes()].concat();
            return self.mem.write(va, &bytes);
        }
        let fp_both = vl
            && (base.len() > 2
                && matches!(
                    &base[..base.len() - 2],
                    "ADD" | "SUB" | "MUL" | "DIV" | "SQRT" | "MIN" | "MAX"
                )
                || VEX_FP.contains(&base));
        let mxcsr_in = self.th.mxcsr;
        if fp_both {
            // both halves with the real MXCSR, their flags collected, then one check (an unmasked exception faults
            // with no lane committed)
            self.defer_fp = true;
            self.deferred_flags = 0;
        }
        let mut low_src = s2;
        if let (Some(SseLoc::X(a)), SseLoc::X(dd)) = (s1, d)
            && a != dd
        {
            if s2 == SseLoc::X(dd) {
                self.th.xmm[16] = self.th.xmm[usize::from(dd)];
                low_src = SseLoc::X(16);
            }
            self.th.xmm[usize::from(dd)] = self.th.xmm[usize::from(a)];
        }
        self.sse(base, d, low_src, 0, imm)?;
        match d {
            SseLoc::Loc(Loc::R { idx, bits, .. }) if base == "PMOVMSKB" && vl => {
                // 32 sign bits: the upper half's 16 above
                let upper = (0..16).fold(0u64, |acc, i| acc | ((src_hi >> (8 * i + 7)) as u64 & 1) << (16 + i));
                let v = self.rd(idx, bits, false) | upper;
                self.wr(idx, bits, false, v);
            }
            SseLoc::X(_) if matches!(base, "COMISD" | "UCOMISD" | "COMISS" | "UCOMISS") => {} // flags only: d is a source
            SseLoc::X(dd) => {
                let dd = usize::from(dd);
                if vl {
                    self.th.xmm[16] = first_hi;
                    self.th.xmm[17] = src_hi;
                    let imm = if base == "PBLENDD" { imm.map(|i| i >> 4) } else { imm };
                    self.sse(base, SseLoc::X(16), SseLoc::X(17), 0, imm)?;
                    self.th.ymm_hi[dd] = self.th.xmm[16];
                } else {
                    self.th.ymm_hi[dd] = 0;
                }
            }
            _ => {}
        }
        if fp_both {
            // this instruction's flags, both halves
            let env = crate::x86fp::Env {
                mxcsr: mxcsr_in,
                flags: self.deferred_flags,
            };
            self.defer_fp = false;
            self.th.mxcsr = mxcsr_in;
            self.fp_check(&env)?;
        }
        Ok(())
    }

    /// `_fma` (runtime.py 788-827): VFMADD/VFMSUB/VFNMADD/VFNMSUB 132/213/231 in SS/SD/PS/PD, one rounding. Operand 1
    /// is the destination, 2 VEX.vvvv, 3 r/m; 132 is o1*o3 + o2, 213 o2*o1 + o3, 231 o2*o3 + o1.
    fn fma(&mut self, ic: &str, d: SseLoc, s2: SseLoc, s3: SseLoc, vl: bool) -> Result<(), Fault> {
        use crate::x86fp::{self as fp, Env, F32, F64};
        let refused = || Fault::Refused(format!("fma {ic}"));
        let rest = ic.strip_prefix('V').ok_or_else(refused)?;
        let (kind, rest) = ["FNMADD", "FNMSUB", "FMADD", "FMSUB"]
            .iter()
            .find_map(|k| rest.strip_prefix(k).map(|r| (*k, r)))
            .ok_or_else(refused)?;
        if rest.len() != 5 {
            return Err(refused());
        }
        let (order, form) = rest.split_at(3);
        let (neg_p, neg_a) = match kind {
            "FMADD" => (false, false),
            "FMSUB" => (false, true),
            "FNMADD" => (true, false),
            _ => (true, true),
        };
        let (mul, add) = match order {
            "132" => ((0, 2), 1),
            "213" => ((1, 0), 2),
            "231" => ((1, 2), 0),
            _ => return Err(refused()),
        };
        let f = if form.as_bytes()[1] == b'S' { F32 } else { F64 };
        let scalar = form.as_bytes()[0] == b'S';
        let nbytes = if scalar { f.bits / 8 } else { 16 };
        let (d, s2) = (self.xmm(d)?, self.xmm(s2)?);
        let (o1, o2, o3) = (self.th.xmm[d], self.th.xmm[s2], self.xread(s3, nbytes)?);
        let mut env = Env::new(self.th.mxcsr);
        let count = if scalar { 1 } else { (128 / f.bits) as usize };
        let n = 128 / f.bits;
        let (a1, a2, a3) = (
            fp::lanes(o1, f.bits, n),
            fp::lanes(o2, f.bits, n),
            fp::lanes(o3, f.bits, n),
        );
        let mut out = a1.clone();
        let mut flags = 0;
        // each lane's flags, then all of them (as execute)
        let mut lane = |ops: [u64; 3], env: &mut Env| {
            env.flags = 0;
            let r = fp::fma_lane(f, ops, mul, add, neg_p, neg_a, env);
            flags |= env.flags;
            r
        };
        for i in 0..count {
            out[i] = lane([a1[i], a2[i], a3[i]], &mut env);
        }
        let mut new_hi = 0;
        if vl && !scalar {
            let h3 = match s3 {
                SseLoc::X(k) => self.th.ymm_hi[usize::from(k)],
                _ => self.read_bytes(s3, 16, 16)?[0],
            };
            let (b1, b2, b3) = (
                fp::lanes(self.th.ymm_hi[d], f.bits, n),
                fp::lanes(self.th.ymm_hi[s2], f.bits, n),
                fp::lanes(h3, f.bits, n),
            );
            let hi: Vec<u64> = (0..count).map(|i| lane([b1[i], b2[i], b3[i]], &mut env)).collect();
            new_hi = fp::join(&hi, f.bits);
        }
        env.flags = flags;
        self.fp_check(&env)?;
        self.th.xmm[d] = fp::join(&out, f.bits);
        self.th.ymm_hi[d] = new_hi;
        Ok(())
    }

    /// `_sse_049b` (runtime.py 498-572): the set-049b legacy forms, on 128 bits
    fn sse_049b(&mut self, ic: &str, dst: SseLoc, src: SseLoc, imm: Option<u64>) -> Result<(), Fault> {
        use crate::x86fp::{self as fp, Env, F32, F64};
        let imm_of = || imm.ok_or_else(|| Fault::Refused(format!("{ic} without its immediate")));
        if ic == "PEXTRQ" {
            let s = self.xmm(src)?;
            let v = self.th.xmm[s] >> (64 * (imm_of()? & 1)) & mask128(64);
            return self.xwrite(dst, v, 8);
        }
        let d = self.xmm(dst)?;
        let a = self.th.xmm[d];
        match ic {
            "PINSRD" | "PINSRQ" => {
                let w = if ic == "PINSRD" { 32 } else { 64 };
                let lane = (imm_of()? & if w == 32 { 3 } else { 1 }) as u32;
                let v = self.xread(src, w / 8)? & mask128(w);
                self.th.xmm[d] = a & !(mask128(w) << (w * lane)) | v << (w * lane);
                return Ok(());
            }
            "PMOVSXDQ" => {
                let v = self.xread(src, 8)? as u64;
                let ls = [signed(v & 0xFFFF_FFFF, 32), signed(v >> 32, 32)];
                self.th.xmm[d] = pack(&ls, 64);
                return Ok(());
            }
            "CVTPS2PD" => {
                let v = self.xread(src, 8)? as u64;
                let mut env = Env::new(self.th.mxcsr);
                let (mut flags, mut out) = (0, Vec::new());
                for f32 in [v & 0xFFFF_FFFF, v >> 32] {
                    env.flags = 0;
                    out.push(fp::convert(F32, F64, f32, &mut env));
                    flags |= env.flags;
                }
                env.flags = flags;
                self.fp_check(&env)?;
                self.th.xmm[d] = fp::join(&out, 64);
                return Ok(());
            }
            "ROUNDSD" => {
                let v = self.xread(src, 8)? as u64;
                let mut env = Env::new(self.th.mxcsr);
                self.th.xmm[d] = a & !mask128(64) | u128::from(fp::round_to_integer(F64, v, imm_of()?, &mut env));
                return self.fp_check(&env);
            }
            _ => {}
        }
        let v = self.xread(src, 16)?;
        self.th.xmm[d] = match ic {
            "PSHUFB" => {
                // byte i: 0 if index bit 7, else a[index & 15]
                let ab = lanes128(a, 8);
                let ls: Vec<i128> = lanes128(v, 8)
                    .iter()
                    .map(|i| {
                        if i & 0x80 != 0 {
                            0
                        } else {
                            i128::from(ab[(i & 15) as usize])
                        }
                    })
                    .collect();
                pack(&ls, 8)
            }
            "PALIGNR" => {
                // (dst:src) >> 8*imm, the low 16 bytes
                let n = imm_of()?;
                if n < 32 {
                    let wide = (a, v); // a << 128 | v, as two halves
                    let shift = 8 * n as u32;
                    if shift == 0 {
                        wide.1
                    } else if shift < 128 {
                        wide.1 >> shift | wide.0 << (128 - shift)
                    } else {
                        wide.0 >> (shift - 128)
                    }
                } else {
                    0
                }
            }
            "ANDNPD" => !a & v,
            "PERMILPS_VAR" => {
                let aw = lanes128(a, 32);
                let ls: Vec<i128> = lanes128(v, 32)
                    .iter()
                    .map(|c| i128::from(aw[(c & 3) as usize]))
                    .collect();
                pack(&ls, 32)
            }
            "PBLENDD" => {
                let n = imm_of()?;
                let (aw, vw) = (lanes128(a, 32), lanes128(v, 32));
                let ls: Vec<i128> = (0..4)
                    .map(|i| i128::from(if n >> i & 1 != 0 { vw[i] } else { aw[i] }))
                    .collect();
                pack(&ls, 32)
            }
            "CVTDQ2PS" | "CVTTPS2DQ" | "CMPPS_VEX" => {
                let mut env = Env::new(self.th.mxcsr);
                let (mut flags, mut out) = (0, Vec::new());
                for (p, q) in lanes128(a, 32).into_iter().zip(lanes128(v, 32)) {
                    env.flags = 0;
                    out.push(match ic {
                        "CVTDQ2PS" => fp::from_int(F32, q, 32, &mut env),
                        "CVTTPS2DQ" => fp::to_int(F32, q, 32, true, &mut env),
                        _ => {
                            if fp::cmp_predicate(F32, p, q, imm_of()? & 31, &mut env) {
                                0xFFFF_FFFF
                            } else {
                                0
                            }
                        }
                    });
                    flags |= env.flags;
                }
                env.flags = flags;
                self.fp_check(&env)?;
                fp::join(&out, 32)
            }
            _ => return Err(Fault::Refused(format!("sse {ic}"))),
        };
        Ok(())
    }

    /// `_sse_050` (runtime.py 574-666): the set-050 legacy classes, on 128 bits
    fn sse_050(&mut self, ic: &str, dst: SseLoc, src: SseLoc, imm: Option<u64>) -> Result<(), Fault> {
        use crate::x86fp::{self as fp, Env, F32, F64};
        let imm_of = || imm.ok_or_else(|| Fault::Refused(format!("{ic} without its immediate")));
        let sat = |v: i128, w: u32| v.clamp(-(1i128 << (w - 1)), (1i128 << (w - 1)) - 1);
        if matches!(ic, "MOVLPS" | "MOVHPD") {
            // one quadword, memory to or from a register's half
            let shift = if ic == "MOVLPS" { 0 } else { 64 };
            if let SseLoc::X(d) = dst {
                let q = self.xread(src, 8)?;
                let d = usize::from(d);
                self.th.xmm[d] = self.th.xmm[d] & !(mask128(64) << shift) | q << shift;
                return Ok(());
            }
            let s = self.xmm(src)?;
            let v = self.th.xmm[s] >> shift & mask128(64);
            return self.xwrite(dst, v, 8);
        }
        if ic == "MOVNTDQ" {
            let s = self.xmm(src)?;
            let v = self.th.xmm[s];
            return self.xwrite(dst, v, 16);
        }
        if ic == "PEXTRW" {
            // word imm & 7, zero-extended to the register
            let s = self.xmm(src)?;
            let SseLoc::Loc(Loc::R { idx, .. }) = dst else {
                return Err(Fault::Refused("PEXTRW: a general register destination expected".into()));
            };
            let v = (self.th.xmm[s] >> (16 * (imm_of()? & 7)) & 0xFFFF) as u64;
            self.wr(idx, 32, false, v);
            return Ok(());
        }
        let d = self.xmm(dst)?;
        let a = self.th.xmm[d];
        match ic {
            "PINSRB" => {
                let byte = self.xread(src, 1)? & 0xFF;
                let lane = (imm_of()? & 15) as u32;
                self.th.xmm[d] = a & !(0xFFu128 << (8 * lane)) | byte << (8 * lane);
                return Ok(());
            }
            "PMOVZXWD" => {
                let ls: Vec<i128> = lanes128(self.xread(src, 8)?, 16)[..4]
                    .iter()
                    .map(|w| i128::from(*w))
                    .collect();
                self.th.xmm[d] = pack(&ls, 32);
                return Ok(());
            }
            "CMPSS" => {
                let mut env = Env::new(self.th.mxcsr);
                let b = self.xread(src, 4)? as u64 & 0xFFFF_FFFF;
                let hit = fp::cmp_predicate(F32, a as u64 & 0xFFFF_FFFF, b, imm_of()? & 7, &mut env);
                self.fp_check(&env)?;
                self.th.xmm[d] = a & !mask128(32) | if hit { mask128(32) } else { 0 };
                return Ok(());
            }
            _ => {}
        }
        let v = self.xread(src, 16)?;
        let (al8, vl8) = (lanes128(a, 8), lanes128(v, 8));
        self.th.xmm[d] = match ic {
            "PSLLW" => {
                // by imm8 or the source's low quadword
                let count = match imm {
                    Some(n) => n,
                    None => v as u64,
                };
                if count > 15 {
                    0
                } else {
                    let ls: Vec<i128> = lanes128(a, 16).iter().map(|w| i128::from(*w) << count).collect();
                    pack(&ls, 16)
                }
            }
            "SHUFPD" => {
                let n = imm_of()?;
                (a >> (64 * (n & 1)) & mask128(64)) | (v >> (64 * (n >> 1 & 1)) & mask128(64)) << 64
            }
            "MPSADBW" => {
                // eight sums of |a - v| over 4 bytes, the destination's block sliding a byte each (SDM MPSADBW)
                let n = imm_of()?;
                let (d0, s0) = (4 * (n >> 2 & 1) as usize, 4 * (n & 3) as usize);
                let ls: Vec<i128> = (0..8)
                    .map(|i| {
                        (0..4)
                            .map(|j| (i128::from(al8[d0 + i + j]) - i128::from(vl8[s0 + j])).abs())
                            .sum()
                    })
                    .collect();
                pack(&ls, 16)
            }
            "PSADBW" => {
                // per quadword: the sum of |a - v| over its bytes
                let ls: Vec<i128> = [0, 8]
                    .iter()
                    .map(|h| {
                        (*h..h + 8)
                            .map(|i| (i128::from(al8[i]) - i128::from(vl8[i])).abs())
                            .sum()
                    })
                    .collect();
                pack(&ls, 64)
            }
            "PACKSSWB" | "PACKSSDW" => {
                // signed, saturated: the destination's half first
                let w = if ic == "PACKSSWB" { 16 } else { 32 };
                let ls: Vec<i128> = lanes128(a, w)
                    .into_iter()
                    .chain(lanes128(v, w))
                    .map(|n| sat(signed(n, w), w / 2))
                    .collect();
                pack(&ls, w / 2)
            }
            "PMADDWD" => {
                // signed word products, adjacent pairs added
                let aw: Vec<i128> = lanes128(a, 16).iter().map(|n| signed(*n, 16)).collect();
                let vw: Vec<i128> = lanes128(v, 16).iter().map(|n| signed(*n, 16)).collect();
                let ls: Vec<i128> = (0..4)
                    .map(|i| aw[2 * i] * vw[2 * i] + aw[2 * i + 1] * vw[2 * i + 1])
                    .collect();
                pack(&ls, 32)
            }
            "PMADDUBSW" => {
                // unsigned dst bytes by signed src bytes, saturated
                let vb: Vec<i128> = vl8.iter().map(|n| signed(*n, 8)).collect();
                let ls: Vec<i128> = (0..8)
                    .map(|i| {
                        sat(
                            i128::from(al8[2 * i]) * vb[2 * i] + i128::from(al8[2 * i + 1]) * vb[2 * i + 1],
                            16,
                        )
                    })
                    .collect();
                pack(&ls, 16)
            }
            "CVTPS2DQ" | "CVTPD2PS" | "CMPPS" | "CMPPD" => {
                let mut env = Env::new(self.th.mxcsr);
                let (mut flags, mut out) = (0, Vec::new());
                let w;
                if ic == "CVTPD2PS" {
                    // two doubles to two floats; bits 127:64 zeroed
                    for x in lanes128(v, 64) {
                        env.flags = 0;
                        out.push(fp::convert(F64, F32, x, &mut env));
                        flags |= env.flags;
                    }
                    w = 32;
                } else {
                    let f = if ic == "CMPPD" { F64 } else { F32 };
                    w = f.bits;
                    for (p, q) in lanes128(a, w).into_iter().zip(lanes128(v, w)) {
                        env.flags = 0;
                        out.push(if ic == "CVTPS2DQ" {
                            fp::to_int(F32, q, 32, false, &mut env)
                        } else if fp::cmp_predicate(f, p, q, imm_of()? & 7, &mut env) {
                            mask(w)
                        } else {
                            0
                        });
                        flags |= env.flags;
                    }
                }
                env.flags = flags;
                self.fp_check(&env)?;
                fp::join(&out, w)
            }
            _ => return Err(Fault::Refused(format!("sse {ic}"))),
        };
        Ok(())
    }

    /// `_sse2_int` (runtime.py 829-963): the SSE2 integer block, no MXCSR flags
    fn sse2_int(&mut self, ic: &str, dst: SseLoc, src: SseLoc, imm: Option<u64>) -> Result<(), Fault> {
        let imm_of = || imm.ok_or_else(|| Fault::Refused(format!("{ic} without its immediate")));
        match ic {
            "PINSRW" => {
                let d = self.xmm(dst)?;
                let word = self.xread(src, 2)? & 0xFFFF;
                let lane = (imm_of()? & 7) as u32;
                self.th.xmm[d] = self.th.xmm[d] & !(0xFFFFu128 << (16 * lane)) | word << (16 * lane);
                return Ok(());
            }
            "MOVHPS" => {
                if let SseLoc::X(d) = dst {
                    let d = usize::from(d);
                    self.th.xmm[d] = self.th.xmm[d] & mask128(64) | self.xread(src, 8)? << 64;
                    return Ok(());
                }
                let s = self.xmm(src)?;
                let v = self.th.xmm[s] >> 64;
                return self.xwrite(dst, v, 8);
            }
            // string_compare.py and approximate.py: a later slice, refused by name
            "PCMPISTRI" | "PCMPISTRI64" | "RCPPS" | "RSQRTPS" | "RCPSS" | "RSQRTSS" => {
                return Err(Fault::Refused(format!("sse {ic}: its model comes with a later slice")));
            }
            "MOVMSKPS" | "PMOVMSKB" => {
                let s = self.xmm(src)?;
                let v = self.th.xmm[s];
                let SseLoc::Loc(Loc::R { idx, bits, .. }) = dst else {
                    return Err(Fault::Refused(format!("{ic}: a general register destination expected")));
                };
                let r: u64 = if ic == "MOVMSKPS" {
                    (0..4).map(|i| ((v >> (32 * i + 31)) as u64 & 1) << i).sum()
                } else {
                    (0..16).map(|i| ((v >> (8 * i + 7)) as u64 & 1) << i).sum()
                };
                self.wr(idx, bits, false, r);
                return Ok(());
            }
            "CVTDQ2PD" => {
                // exact: every int32 is a double
                let d = self.xmm(dst)?;
                let v = self.xread(src, 8)? as u64;
                let ls: Vec<i128> = [v as u32, (v >> 32) as u32]
                    .iter()
                    .map(|w| i128::from(f64::from(*w as i32).to_bits()))
                    .collect();
                self.th.xmm[d] = pack(&ls, 64);
                return Ok(());
            }
            _ => {}
        }
        let d = self.xmm(dst)?;
        let a = self.th.xmm[d];
        let count_of = |s: &Self| -> Result<u64, Fault> {
            match imm {
                Some(n) => Ok(n),
                None => Ok(s.xread(src, 16)? as u64),
            }
        };
        match ic {
            "PSRLDQ" | "PSLLDQ" => {
                let n = imm_of()?.min(16) as u32;
                self.th.xmm[d] = if n == 16 {
                    0
                } else if ic == "PSRLDQ" {
                    a >> (8 * n)
                } else {
                    a << (8 * n)
                };
                return Ok(());
            }
            "PSRLW" | "PSRAW" | "PSLLQ" => {
                let count = count_of(self)?;
                let w = if ic == "PSLLQ" { 64 } else { 16 };
                self.th.xmm[d] = if ic == "PSRAW" {
                    let ls: Vec<i128> = lanes128(a, 16)
                        .iter()
                        .map(|v| signed(*v, 16) >> count.min(15))
                        .collect();
                    pack(&ls, 16)
                } else if count >= w {
                    0
                } else {
                    let ls: Vec<i128> = lanes128(a, w as u32)
                        .iter()
                        .map(|v| {
                            if ic == "PSRLW" {
                                i128::from(v >> count)
                            } else {
                                i128::from(*v) << count
                            }
                        })
                        .collect();
                    pack(&ls, w as u32)
                };
                return Ok(());
            }
            "PSRLD" | "PSLLD" | "PSRAD" => {
                let count = count_of(self)?;
                self.th.xmm[d] = if ic == "PSRAD" {
                    let ls: Vec<i128> = lanes128(a, 32)
                        .iter()
                        .map(|v| signed(*v, 32) >> count.min(31))
                        .collect();
                    pack(&ls, 32)
                } else if count > 31 {
                    0
                } else {
                    let ls: Vec<i128> = lanes128(a, 32)
                        .iter()
                        .map(|v| {
                            if ic == "PSRLD" {
                                i128::from(v >> count)
                            } else {
                                i128::from(*v) << count
                            }
                        })
                        .collect();
                    pack(&ls, 32)
                };
                return Ok(());
            }
            "MOVHLPS" | "MOVLHPS" => {
                let s = self.xmm(src)?;
                let v = self.th.xmm[s];
                self.th.xmm[d] = if ic == "MOVHLPS" {
                    a & !mask128(64) | v >> 64
                } else {
                    a & mask128(64) | (v & mask128(64)) << 64
                };
                return Ok(());
            }
            "PSRLQ" => {
                let count = count_of(self)?;
                self.th.xmm[d] = if count > 63 {
                    0
                } else {
                    let ls: Vec<i128> = lanes128(a, 64).iter().map(|l| i128::from(l >> count)).collect();
                    pack(&ls, 64)
                };
                return Ok(());
            }
            _ => {}
        }
        let v = self.xread(src, 16)?;
        let words = |x: u128| lanes128(x, 32);
        self.th.xmm[d] = match ic {
            "PSHUFD" => {
                let n = imm_of()?;
                let ws = words(v);
                let ls: Vec<i128> = (0..4).map(|i| i128::from(ws[(n >> (2 * i) & 3) as usize])).collect();
                pack(&ls, 32)
            }
            "PAND" => a & v,
            "POR" | "ORPS" => a | v,
            "PSUBD" | "PSUBQ" => {
                let w = if ic == "PSUBD" { 32 } else { 64 };
                let ls: Vec<i128> = lanes128(a, w)
                    .iter()
                    .zip(lanes128(v, w))
                    .map(|(p, q)| i128::from(*p) - i128::from(q))
                    .collect();
                pack(&ls, w)
            }
            "PUNPCKLBW" => {
                let (ab, vb) = (lanes128(a, 8), lanes128(v, 8));
                let ls: Vec<i128> = (0..8).flat_map(|i| [i128::from(ab[i]), i128::from(vb[i])]).collect();
                pack(&ls, 8)
            }
            "PCMPEQB" => {
                let ls: Vec<i128> = lanes128(a, 8)
                    .iter()
                    .zip(lanes128(v, 8))
                    .map(|(p, q)| if *p == q { 0xFF } else { 0 })
                    .collect();
                pack(&ls, 8)
            }
            "SHUFPS" => {
                let n = imm_of()?;
                let (aw, vw) = (words(a), words(v));
                let ls = [
                    aw[(n & 3) as usize],
                    aw[(n >> 2 & 3) as usize],
                    vw[(n >> 4 & 3) as usize],
                    vw[(n >> 6 & 3) as usize],
                ]
                .map(i128::from);
                pack(&ls, 32)
            }
            "PXOR" => a ^ v,
            "ANDNPS" | "PANDN" => !a & v,
            "UNPCKLPS" | "PUNPCKLDQ" | "UNPCKHPS" => {
                let (aw, vw) = (words(a), words(v));
                let base = if ic == "UNPCKHPS" { 2 } else { 0 };
                let ls = [aw[base], vw[base], aw[base + 1], vw[base + 1]].map(i128::from);
                pack(&ls, 32)
            }
            "PADDD" => {
                let ls: Vec<i128> = words(a)
                    .iter()
                    .zip(words(v))
                    .map(|(p, q)| i128::from(*p) + i128::from(q))
                    .collect();
                pack(&ls, 32)
            }
            "PACKUSWB" => {
                // signed words to unsigned bytes, saturated
                let ls: Vec<i128> = lanes128(a, 16)
                    .into_iter()
                    .chain(lanes128(v, 16))
                    .map(|word| signed(word, 16).clamp(0, 255))
                    .collect();
                pack(&ls, 8)
            }
            "PSHUFLW" | "PSHUFHW" => {
                let n = imm_of()?;
                let ws = lanes128(v, 16);
                let base = if ic == "PSHUFLW" { 0 } else { 4 };
                let moved: Vec<u64> = (0..4).map(|i| ws[base + (n >> (2 * i) & 3) as usize]).collect();
                let all: Vec<u64> = if ic == "PSHUFLW" {
                    moved.iter().chain(&ws[4..]).copied().collect()
                } else {
                    ws[..4].iter().chain(&moved).copied().collect()
                };
                let ls: Vec<i128> = all.iter().map(|w| i128::from(*w)).collect();
                pack(&ls, 16)
            }
            _ => {
                if let Some((w, f)) = lane_arith(ic) {
                    let ls: Vec<i128> = lanes128(a, w)
                        .iter()
                        .zip(lanes128(v, w))
                        .map(|(p, q)| f(*p, q, w))
                        .collect();
                    pack(&ls, w)
                } else if let Some((w, high)) = interleave(ic) {
                    let (al, vl) = (lanes128(a, w), lanes128(v, w));
                    let half = (64 / w) as usize;
                    let base = if high { half } else { 0 };
                    let ls: Vec<i128> = (0..half)
                        .flat_map(|i| [i128::from(al[base + i]), i128::from(vl[base + i])])
                        .collect();
                    pack(&ls, w)
                } else {
                    return Err(Fault::Refused(format!("sse {ic}")));
                }
            }
        };
        Ok(())
    }
}

/// `_exec` (995-1013): one instruction. On a guest fault every register, flag, MXCSR and DF returns to its value
/// before it, but REP's completed iterations stay (SDM Vol 1 7.3.9), MXCSR's flags a fault set survive, and RIP is the
/// instruction's. An MMX form keeps the x87 state a fault leaves, which `_mmx`'s `finally` writes back (a faulting
/// store has set TOP 0, set 065b); an x87 one writes it only on success. `code` is the instruction's bytes, which
/// `_windows_divide_status` decodes again.
pub fn step<M: Memory>(th: &mut Thread, mem: &mut M, ins: &Lifted, code: &[u8]) -> Result<(), Fault> {
    let saved = th.clone();
    let mut s = Step {
        th,
        mem,
        t: Default::default(),
        code,
        defer_fp: false,
        deferred_flags: 0,
    };
    let r = s.exec_ops(ins);
    s.th.xmm[16..].fill(0); // `_vex`'s `del x[16:]`: the scratch registers go
    if let Err(Fault::Guest { .. }) = &r {
        let interrupted = s.th.regs;
        let mxcsr = s.th.mxcsr;
        let x87 = (s.th.x87_sw, s.th.x87_tags, s.th.x87_regs);
        *s.th = saved;
        if ins.ops.iter().any(|op| matches!(op, Op::Rep { .. })) {
            s.th.regs = interrupted;
        }
        if ins.ops.iter().any(|op| matches!(op, Op::Mmx { .. })) {
            (s.th.x87_sw, s.th.x87_tags, s.th.x87_regs) = x87;
        }
        s.th.mxcsr = mxcsr;
        s.th.rip = ins.address;
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Memory of one region at 0x1000, 64 bytes, for the model's unit tests.
    struct Region(BTreeMap<u64, u8>);

    impl Memory for Region {
        fn read(&self, va: u64, n: usize) -> Result<Vec<u8>, Fault> {
            (va..va + n as u64)
                .map(|a| self.0.get(&a).copied().ok_or_else(|| guest(format!("0x{a:x}"))))
                .collect()
        }
        fn write(&mut self, va: u64, data: &[u8]) -> Result<(), Fault> {
            self.read(va, data.len())?;
            for (i, b) in data.iter().enumerate() {
                self.0.insert(va + i as u64, *b);
            }
            Ok(())
        }
    }

    fn region() -> Region {
        Region((0x1000..0x1040).map(|a| (a, 0)).collect())
    }

    fn thread() -> Thread {
        Thread {
            regs: [0; 16],
            flags: Flags::default(),
            df: false,
            mxcsr: 0x1F80,
            fcw: 0x27F,
            rip: 0x1000,
            xmm: [0; 18],
            ymm_hi: [0; 16],
            x87_sw: 0,
            x87_tags: 0,
            x87_regs: [0; 8],
        }
    }

    fn run(code: &[u8], th: &mut Thread, mem: &mut Region) -> Result<(), Fault> {
        let ins = translator::lift::lift(code, th.rip).unwrap();
        step(th, mem, &ins, code)
    }

    /// `tests/test_runtime.py` 59-61: `mul_hi` drives the skeleton's decimal formatting, multiplying by
    /// 0xcccccccccccccccd; the high half of 1234567890 times it is 0x3ade68a8 (123456789 after the shift by 3).
    #[test]
    fn mul_hi_is_the_high_half_of_the_product() {
        let s = Step {
            th: &mut thread(),
            mem: &mut region(),
            t: Default::default(),
            code: &[],
            defer_fp: false,
            deferred_flags: 0,
        };
        assert_eq!(
            s.alu("mul_hi", 1234567890, 0xcccc_cccc_cccc_cccd, 64).unwrap(),
            0x3ade_68a8
        );
    }

    /// `_div` and `_exec` (runtime.py 343-374, 995-1013): DIV ECX with ECX 0 faults with
    /// STATUS_INTEGER_DIVIDE_BY_ZERO, every register as before it and RIP at the instruction; with 7 / 2 it writes
    /// quotient 3 and remainder 1 and leaves every flag undefined.
    #[test]
    fn div_faults_before_any_write_and_leaves_the_flags_undefined() {
        let (mut th, mut mem) = (thread(), region());
        th.regs[0] = 7;
        th.flags.c = Some(true);
        let r = run(&[0xF7, 0xF1], &mut th, &mut mem); // div ecx
        assert!(
            matches!(
                r,
                Err(Fault::Guest {
                    code: STATUS_INTEGER_DIVIDE_BY_ZERO,
                    ..
                })
            ),
            "{r:?}"
        );
        assert_eq!((th.regs[0], th.rip, th.flags.c), (7, 0x1000, Some(true)));
        th.regs[1] = 2;
        run(&[0xF7, 0xF1], &mut th, &mut mem).unwrap();
        assert_eq!((th.regs[0], th.regs[2], th.rip), (3, 1, 0x1002));
        assert_eq!(th.flags, Flags::default());
    }

    /// `integer_model` RCR (77-90): an 8-bit RCR takes its count mod 9; RCR AL by 9 with CF 0 leaves AL and CF as
    /// they were.
    #[test]
    fn an_eight_bit_rcr_takes_its_count_mod_nine() {
        let (mut th, mut mem) = (thread(), region());
        th.regs[0] = 0x81;
        th.regs[1] = 9;
        th.flags.c = Some(false);
        run(&[0xD2, 0xD8], &mut th, &mut mem).unwrap(); // rcr al, cl
        assert_eq!((th.regs[0] & 0xFF, th.flags.c), (0x81, Some(false)));
    }
}
