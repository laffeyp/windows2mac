// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The model (Sprint 173; step 4's first component, `docs/step-4-census-2026-10-28.md` row 4): one lifted instruction
//! run on a thread's registers and flags over guest memory, carried from `src/w2m/runtime.py` `_exec` and `_exec_ops`
//! (995-1161) with the helpers they call (193-388) and `src/w2m/integer_model.py` (whole). The VM reaches it through
//! hvc 10 and 11 (`vm_backend.py` `_slow_sse`, 4268-4323), when a block's instruction is one the lowering leaves to it.
//!
//! This slice holds the core's integer and control operations and the integer classes. An `sse` operation is refused
//! by name: the SSE, MMX, VEX and x87 slices follow.

use translator::lift::{Lifted, Loc, Op, RAX, RBX, RCX, RDI, RDX, RSI, RSP, Temp};

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
                Op::Sse { ic, .. } => {
                    return Err(Fault::Refused(format!("sse {ic}: the model's SSE slice comes later")));
                }
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

/// `_exec` (995-1013): one instruction. On a guest fault every register, flag, MXCSR and DF returns to its value
/// before it, but REP's completed iterations stay (SDM Vol 1 7.3.9), MXCSR's flags a fault set survive, and RIP is the
/// instruction's. `code` is the instruction's bytes, which `_windows_divide_status` decodes again.
pub fn step<M: Memory>(th: &mut Thread, mem: &mut M, ins: &Lifted, code: &[u8]) -> Result<(), Fault> {
    let saved = th.clone();
    let mut s = Step {
        th,
        mem,
        t: Default::default(),
        code,
    };
    let r = s.exec_ops(ins);
    if let Err(Fault::Guest { .. }) = &r {
        let interrupted = s.th.regs;
        let mxcsr = s.th.mxcsr;
        *s.th = saved;
        if ins.ops.iter().any(|op| matches!(op, Op::Rep { .. })) {
            s.th.regs = interrupted;
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
