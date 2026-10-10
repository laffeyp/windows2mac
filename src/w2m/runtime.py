"""runtime.py — execute lifted x86-64 operations on the host, with kernel32 serviced in the host
(Sprint 029; profile_kind `model`). One guest thread. The executor runs `lift.py` operations; a
branch to a loader thunk calls the named kernel32 function (src/w2m/kernel32.py), which returns to the
caller as RET would. Since Sprint 030 it is also the reference the managed-VM back end is compared
against, one instruction at a time (`step`).
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field

from w2m import cpuid
from w2m import x86fp as fp
from w2m.kernel32 import M64, HandlerFailed, Kernel32, Thread, Unimplemented, monotonic_count
from w2m.lift import RAX, RDX, Lifted, LiftError, Op, lift
from w2m.loader import GuestFault, Loaded

STATUS_INTEGER_DIVIDE_BY_ZERO = 0xC000_0094   # ntstatus.h; DIV #DE with a zero divisor
STATUS_INTEGER_OVERFLOW = 0xC000_0095         # ntstatus.h; DIV #DE with a quotient too wide

__all__ = ["Result", "Runtime", "UndefinedFlag", "Unimplemented"]


class UndefinedFlag(RuntimeError):
    """An instruction read a flag the SDM leaves undefined after the instruction that set it."""


@dataclass
class Result:
    exit_code: int | None
    stdout: bytes
    instructions_executed: int
    distinct_addresses: int
    calls: list[str]
    stop: str
    calls_total: dict[str, int] = field(default_factory=dict)   # every call by service (VM: past the ring too)
    calls_dropped: int = 0                                       # calls the ring could not record in order


def _mask(bits: int) -> int:
    return (1 << bits) - 1


def _parity(v: int) -> bool:
    return bin(v & 0xFF).count("1") % 2 == 0


class Runtime:
    def __init__(self, loaded: Loaded, tid: int = 4,
                 clock: Callable[[], int] = monotonic_count) -> None:
        self.ld = loaded
        self.mem = loaded.memory
        self.th = Thread(tid)
        self.th.rip = loaded.entry
        self.th.regs[4] = loaded.stack_top - 0x100 - 8           # entry as if called: RSP % 16 == 8
        self.cache: dict[int, Lifted] = {}
        self.kernel = Kernel32(self.mem, clock)
        self.kernel32 = self.kernel.handlers
        self.steps = 0
        self.seen: set[int] = set()
        self.cpuid_reads: list[tuple[int, int]] = []           # (EAX, ECX) per CPUID executed

    @property
    def exit_code(self) -> int | None:
        return self.kernel.exit_code

    @property
    def calls(self) -> list[str]:
        return self.kernel.calls

    # registers -------------------------------------------------------------------------------
    def _rd(self, idx: int, bits: int, hi: bool) -> int:
        v = self.th.regs[idx]
        return (v >> 8) & 0xFF if hi else v & _mask(bits)

    def _wr(self, idx: int, bits: int, hi: bool, v: int) -> None:
        old = self.th.regs[idx]
        if hi:
            self.th.regs[idx] = (old & ~0xFF00) | ((v & 0xFF) << 8)
        elif bits == 32:
            self.th.regs[idx] = v & 0xFFFFFFFF                    # zero-extends (SDM 1 3.4.1.1)
        elif bits == 64:
            self.th.regs[idx] = v & M64
        else:
            self.th.regs[idx] = (old & ~_mask(bits) & M64) | (v & _mask(bits))

    def _load(self, loc: Op, t: dict[int, int]) -> int:
        if loc[0] == "r":
            return self._rd(loc[1], loc[2], loc[3])
        return int.from_bytes(self.mem.read(t[loc[1]], loc[2]), "little")

    def _store(self, loc: Op, v: int) -> None:
        if loc[0] == "r":
            self._wr(loc[1], loc[2], loc[3], v)
        else:
            n = loc[2]
            self.mem.write(self._t[loc[1]], (v & _mask(8 * n)).to_bytes(n, "little"))

    # flags -----------------------------------------------------------------------------------
    def _flag(self, f: str) -> bool:
        v = self.th.flags[f]
        if v is None:
            raise UndefinedFlag(f"{f} read while undefined at 0x{self.th.rip:x}")
        return v

    def _cond(self, cc: str) -> bool:
        if cc == "always":
            return True
        neg = cc.startswith("n")                                  # nb, nz, nbe, ...: negations
        base = {"o": lambda: self._flag("O"), "b": lambda: self._flag("C"),
                "z": lambda: self._flag("Z"), "be": lambda: self._flag("C") or self._flag("Z"),
                "s": lambda: self._flag("S"), "p": lambda: self._flag("P"),
                "l": lambda: self._flag("S") != self._flag("O"),
                "le": lambda: self._flag("Z") or self._flag("S") != self._flag("O")}
        key = cc[1:] if neg else cc
        r = base[key]()
        return not r if neg else r

    def _set_flags(self, kind: str, a: int, b: int, r: int, bits: int) -> None:
        m, sign = _mask(bits), 1 << (bits - 1)
        res = r & m
        f = self.th.flags
        if kind == "mul":                                         # SDM MUL: SF ZF AF PF undefined
            f.update(C=res != 0, O=res != 0, Z=None, S=None, P=None, A=None)
            return
        cnt = b & (0x3F if bits == 64 else 0x1F)                  # shift and rotate counts are masked
        if kind in ("shr", "shl", "sar", "rol", "ror") and cnt == 0:
            return                                                # SDM: a zero count affects no flag
        if kind == "shr":
            # SDM SHR: CF undefined when the count >= the operand size (Sprint 041; the model had
            # defined it); OF only for a 1-bit shift; AF undefined
            f.update(C=bool((a >> (cnt - 1)) & 1) if cnt < bits else None, Z=res == 0, S=bool(res & sign),
                     P=_parity(res), O=bool(a & sign) if cnt == 1 else None, A=None)
            return
        if kind == "shl":                                         # SDM SAL/SHL
            c = bool((a >> (bits - cnt)) & 1) if cnt < bits else None
            f.update(C=c, Z=res == 0, S=bool(res & sign), P=_parity(res), A=None,
                     O=(bool(res & sign) != c) if cnt == 1 and c is not None else None)
            return
        if kind == "sar":                                         # SDM SAR: CF defined for every count
            sa = (a & m) - (1 << bits) if a & sign else a & m
            f.update(C=bool((sa >> (cnt - 1)) & 1), Z=res == 0, S=bool(res & sign), P=_parity(res),
                     O=False if cnt == 1 else None, A=None)
            return
        if kind == "rol":                                         # SDM ROL: only CF and OF
            f.update(C=bool(res & 1), O=(bool(res & sign) != bool(res & 1)) if cnt == 1 else None)
            return
        if kind == "ror":                                         # SDM ROR: only CF and OF
            f.update(C=bool(res & sign), O=(bool(res & sign) != bool(res & (sign >> 1))) if cnt == 1 else None)
            return
        if kind == "bt":                                          # SDM BT/BTS/BTR: CF the bit; ZF unchanged
            f.update(C=bool(r & 1), O=None, S=None, A=None, P=None)
            return
        if kind == "sbb":                                         # SDM SBB: a - b - CF, with borrow
            f.update(Z=res == 0, S=bool(res & sign), P=_parity(res), A=bool((a ^ b ^ res) & 0x10),
                     O=bool((a ^ b) & (a ^ res) & sign), C=bool(((~a & b) | (~(a ^ b) & res)) & sign))
            return
        if kind == "imul":                                        # SDM IMUL: CF=OF on truncation; SF ZF AF PF undefined
            sx = (a & m) - (1 << bits) if a & sign else a & m
            sy = (b & m) - (1 << bits) if b & sign else b & m
            sr = res - (1 << bits) if res & sign else res
            f.update(C=sx * sy != sr, O=sx * sy != sr, Z=None, S=None, P=None, A=None)
            return
        f.update(Z=res == 0, S=bool(res & sign), P=_parity(res))
        if kind == "logic":
            f.update(C=False, O=False, A=None)                    # SDM AND/OR/XOR/TEST: AF undefined
            return
        f["A"] = bool((a ^ b ^ res) & 0x10)                       # carry out of bit 3 (SDM Vol 1 3.4.3.1)
        if kind in ("add", "inc"):
            o = bool(~(a ^ b) & (a ^ res) & sign)
            f["O"] = o
            if kind == "add":
                f["C"] = (a & m) + (b & m) > m
        elif kind in ("sub", "dec"):
            f["O"] = bool((a ^ b) & (a ^ res) & sign)
            if kind == "sub":
                f["C"] = (a & m) < (b & m)

    def _div(self, bits: int, divisor: int) -> None:
        """DIV (SDM DIV): faults before any write; every flag undefined afterwards."""
        m = _mask(bits)
        divisor &= m
        if bits == 8:
            dividend = self._rd(RAX, 16, False)
        else:
            dividend = self._rd(RDX, bits, False) << bits | self._rd(RAX, bits, False)
        if divisor == 0:
            raise GuestFault(f"#DE: DIV by zero at 0x{self.th.rip:x}", STATUS_INTEGER_DIVIDE_BY_ZERO)
        q, rem = divmod(dividend, divisor)
        if q > m:
            raise GuestFault(f"#DE: DIV quotient 0x{q:x} wider than {bits} bits at 0x{self.th.rip:x}",
                             STATUS_INTEGER_OVERFLOW)
        if bits == 8:
            self._wr(RAX, 8, False, q)
            self._wr(RAX, 8, True, rem)
        else:
            self._wr(RAX, bits, False, q)
            self._wr(RDX, bits, False, rem)
        self.th.flags.update(dict.fromkeys("CZSOPA"))

    # SSE ---------------------------------------------------------------------------------------
    def _xread(self, loc: Op, nbytes: int) -> int:
        if loc[0] == "x":
            return self.th.xmm[loc[1]] & _mask(8 * nbytes)
        if loc[0] == "r":
            return self._rd(loc[1], loc[2], loc[3])
        return int.from_bytes(self.mem.read(self._t[loc[1]], nbytes), "little")

    def _sse(self, ic: str, dst: Op, src: Op, align: int) -> None:
        """One legacy-SSE instruction: every read and every fault check before any write."""
        th = self.th
        for loc in (dst, src):
            if loc[0] == "m" and align and self._t[loc[1]] % align:
                raise GuestFault(f"#GP: {ic} memory operand at 0x{self._t[loc[1]]:x} not {align}-byte aligned")
        msize = next((loc[2] for loc in (dst, src) if loc[0] == "m"), 16)
        x = th.xmm
        if ic in ("MOVAPS", "MOVAPD", "MOVUPS", "MOVUPD"):
            v = self._xread(src, 16)
            self._xwrite(dst, v, 16)
        elif ic in ("MOVSS", "MOVSD_XMM"):
            n = 4 if ic == "MOVSS" else 8
            v = self._xread(src, n)
            if dst[0] == "x" and src[0] == "x":
                x[dst[1]] = x[dst[1]] & ~_mask(8 * n) & _mask(128) | v        # register form merges
            else:
                self._xwrite(dst, v, n)                                       # load zeroes the rest
        elif ic in ("MOVD", "MOVQ"):
            n = 4 if ic == "MOVD" else 8
            if src[0] == "r":
                n = src[2] // 8
            elif dst[0] == "r":
                n = dst[2] // 8
            v = self._xread(src, n)
            self._xwrite(dst, v, n)                                           # to XMM: zero-extended
        elif ic in ("XORPS", "XORPD", "ANDPS", "ANDPD"):
            a, c = x[dst[1]], self._xread(src, 16)
            x[dst[1]] = a ^ c if ic.startswith("XOR") else a & c
        elif ic[:-2] in ("ADD", "SUB", "MUL", "DIV", "SQRT", "MIN", "MAX"):
            v = self._xread(src, msize if src[0] == "m" else 16)
            out, th.mxcsr = self._fp(lambda: fp.execute(ic, x[dst[1]], v, th.mxcsr))
            x[dst[1]] = out
        elif ic in ("CVTTSS2SI", "CVTTSD2SI", "CVTSS2SI", "CVTSD2SI"):
            f = fp.F32 if "SS" in ic else fp.F64
            v = self._xread(src, f.bits // 8)
            env = fp.Env(th.mxcsr)
            r = fp.to_int(f, v, dst[2], ic.startswith("CVTT"), env)
            self._fp_check(env)
            self._wr(dst[1], dst[2], False, r)
        elif ic in ("CVTSI2SS", "CVTSI2SD"):
            f = fp.F32 if ic.endswith("SS") else fp.F64
            bits = src[2] if src[0] == "r" else 8 * src[2]
            v = self._xread(src, bits // 8)
            env = fp.Env(th.mxcsr)
            r = fp.from_int(f, v, bits, env)
            self._fp_check(env)
            x[dst[1]] = x[dst[1]] & ~_mask(f.bits) & _mask(128) | r
        elif ic in ("CVTSS2SD", "CVTSD2SS"):
            s_f, d_f = (fp.F32, fp.F64) if ic == "CVTSS2SD" else (fp.F64, fp.F32)
            v = self._xread(src, s_f.bits // 8)
            env = fp.Env(th.mxcsr)
            r = fp.convert(s_f, d_f, v, env)
            self._fp_check(env)
            x[dst[1]] = x[dst[1]] & ~_mask(d_f.bits) & _mask(128) | r
        elif ic in ("UCOMISS", "UCOMISD", "COMISS", "COMISD"):
            f = fp.F32 if ic.endswith("SS") else fp.F64
            a, c = x[dst[1]] & _mask(f.bits), self._xread(src, f.bits // 8)
            env = fp.Env(th.mxcsr)
            z, p, cf = fp.compare(f, a, c, env, signal_qnan=ic.startswith("COMI"))
            self._fp_check(env)
            th.flags.update(Z=z, P=p, C=cf, O=False, S=False, A=False)
        else:
            raise LiftError(f"sse {ic}")

    def _xwrite(self, loc: Op, v: int, nbytes: int) -> None:
        if loc[0] == "x":
            self.th.xmm[loc[1]] = v & _mask(8 * nbytes)
        elif loc[0] == "r":
            self._wr(loc[1], loc[2], loc[3], v)
        else:
            self.mem.write(self._t[loc[1]], (v & _mask(8 * nbytes)).to_bytes(nbytes, "little"))

    def _fp(self, run: Callable[[], tuple[int, int]]) -> tuple[int, int]:
        try:
            return run()
        except fp.SimdFault as e:
            self.th.mxcsr |= e.flags                          # flags at an unmasked fault (oracle)
            raise

    def _fp_check(self, env: fp.Env) -> None:
        try:
            env.check()
        except fp.SimdFault as e:
            self.th.mxcsr |= e.flags                          # the flags the fault reports
            raise
        self.th.mxcsr |= env.flags

    # execution -------------------------------------------------------------------------------
    def _exec(self, ins: Lifted) -> None:
        """Run one instruction; on a fault, every register returns to its value before it (the
        instruction's single memory store is all-or-nothing in Memory.write)."""
        th = self.th
        saved = (list(th.regs), dict(th.flags), list(th.xmm), th.mxcsr, th.df)
        try:
            self._exec_ops(ins)
        except (GuestFault, fp.SimdFault):
            mxcsr = th.mxcsr
            th.regs, th.flags, th.xmm, th.mxcsr, th.df = saved
            th.mxcsr = mxcsr                                  # MXCSR flags a fault sets survive it
            th.rip = ins.address
            raise

    def _exec_ops(self, ins: Lifted) -> None:
        t: dict[int, int] = {}
        self._t = t
        nxt = ins.address + ins.length
        for op in ins.ops:
            k = op[0]
            if k == "imm":
                t[op[1]] = op[2] & M64
            elif k == "addr":
                _, d, base, index, scale, disp, abits, seg = op
                if seg is not None:            # the model has no TEB; the VM serves gs: (Sprint 044)
                    raise HandlerFailed(f"{seg}-relative access at 0x{ins.address:x}: the model has no TEB")
                v = disp + (self.th.regs[base] if base is not None else 0)
                v += self.th.regs[index] * scale if index is not None else 0
                t[d] = v & _mask(abits)
            elif k == "div":
                self._div(op[1], t[op[2]])
            elif k == "ld":
                t[op[1]] = self._load(op[2], t)
            elif k == "st":
                self._store(op[1], t[op[2]])
            elif k == "alu":
                _, d, o, a, b, bits = op
                x, y, m = t[a], t[b], _mask(bits)
                if o == "add":
                    v = x + y
                elif o == "sub":
                    v = x - y
                elif o == "and":
                    v = x & y
                elif o == "or":
                    v = x | y
                elif o == "xor":
                    v = x ^ y
                elif o == "shr":
                    v = (x & m) >> (y & (0x3F if bits == 64 else 0x1F))
                elif o == "mul_lo":
                    v = x * y
                elif o == "mul_hi":
                    v = (x * y) >> 64
                elif o == "sext32":
                    v = x - (1 << 32) if x & 0x80000000 else x
                elif o in ("sext8", "sext16"):
                    w = 8 if o == "sext8" else 16
                    xw = x & _mask(w)
                    v = xw - (1 << w) if xw >> (w - 1) else xw
                elif o == "bswap":
                    v = int.from_bytes((x & m).to_bytes(bits // 8, "little"), "big")
                elif o == "sbb":
                    v = x - y - int(self._flag("C"))
                elif o in ("shl", "sar", "rol", "ror"):
                    c = y & (0x3F if bits == 64 else 0x1F)
                    xm = x & m
                    if o == "shl":
                        v = xm << c
                    elif o == "sar":
                        v = ((xm - (1 << bits)) if xm >> (bits - 1) else xm) >> c
                    else:
                        k = c % bits
                        v = ((xm << k) | (xm >> (bits - k))) if o == "rol" else ((xm >> k) | (xm << (bits - k)))
                elif o in ("smul_lo", "smul_hi"):
                    sx = (x & m) - (1 << bits) if x >> (bits - 1) & 1 else x & m
                    sy = (y & m) - (1 << bits) if y >> (bits - 1) & 1 else y & m
                    v = sx * sy if o == "smul_lo" else (sx * sy) >> bits

                else:
                    raise LiftError(f"alu {o}")
                t[d] = v & m
            elif k == "flags":
                _, kind, a, b, r, bits = op
                self._set_flags(kind, t[a], t[b], t[r], bits)
            elif k == "set":
                self._store(op[2], int(self._cond(op[1])))
            elif k == "df":
                self.th.df = bool(op[1])
            elif k == "sel":                                      # ("sel", t, cond_t, x, y): cond ? x : y
                t[op[1]] = t[op[3]] if t[op[2]] else t[op[4]]
            elif k in ("cst", "cstz"):                            # store when cond is nonzero (cst) or zero (cstz)
                if bool(t[op[1]]) == (k == "cst"):
                    self._store(op[2], t[op[3]])
            elif k == "cmov":                                     # ("cmov", cc, loc, t): SDM CMOVcc
                if self._cond(op[1]):
                    self._store(op[2], t[op[3]])
                elif op[2][0] == "r" and op[2][2] == 32:
                    self.th.regs[op[2][1]] &= 0xFFFF_FFFF         # 64-bit mode: a 32-bit CMOV zero-extends
            elif k == "cpuid":                                # 32-bit results zero-extend (SDM CPUID)
                eax, ecx = self.th.regs[0] & 0xFFFF_FFFF, self.th.regs[1] & 0xFFFF_FFFF
                self.cpuid_reads.append((eax, ecx))
                a, b, c, d = cpuid.lookup(eax, ecx)
                self.th.regs[0], self.th.regs[3], self.th.regs[1], self.th.regs[2] = a, b, c, d
            elif k == "sse":
                self._sse(op[1], op[2], op[3], op[4])
            elif k == "br":
                if self._cond(op[1]):
                    self.th.rip = t[op[2]]
                    return
            else:
                raise LiftError(f"operation {k}")
        self.th.rip = nxt

    def step(self) -> bool:
        """Execute one x86 instruction, or service one kernel32 call when RIP is a thunk.
        Returns False once the process has exited."""
        th = self.th
        if th.rip in self.ld.thunks:
            ret = self.kernel.call(self.ld.thunks[th.rip], th)
            if ret is None:
                return False
            th.rip = ret
            return True
        ins = self.cache.get(th.rip)
        if ins is None:
            ins = self.cache[th.rip] = lift(self.mem.code_at(th.rip), th.rip)
        self.seen.add(th.rip)
        self._exec(ins)
        self.steps += 1
        return True

    def run(self, max_steps: int = 1_000_000) -> Result:
        stop = "max_steps"
        try:
            while self.steps < max_steps:
                if not self.step():
                    stop = "ExitProcess"
                    break
        except (LiftError, GuestFault, UndefinedFlag, Unimplemented, HandlerFailed, fp.SimdFault) as e:
            stop = f"{type(e).__name__}: {e}"
        return Result(self.exit_code, bytes(self.kernel.out), self.steps, len(self.seen),
                      self.calls, stop)
