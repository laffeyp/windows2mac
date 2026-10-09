"""runtime.py — execute lifted x86-64 operations on the host, with kernel32 serviced in the host
(Sprint 029; profile_kind `model`). One guest thread. The executor runs `lift.py` operations; a
branch to a loader thunk calls the named kernel32 function (src/w2m/kernel32.py), which returns to the
caller as RET would. Since Sprint 030 it is also the reference the managed-VM back end is compared
against, one instruction at a time (`step`).
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

from w2m.kernel32 import M64, HandlerFailed, Kernel32, Thread, Unimplemented, monotonic_count
from w2m.lift import Lifted, LiftError, Op, lift
from w2m.loader import GuestFault, Loaded

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
            f.update(C=res != 0, O=res != 0, Z=None, S=None, P=None)
            return
        if kind == "shr":
            cnt = b & (0x3F if bits == 64 else 0x1F)
            if cnt == 0:
                return                                            # flags unaffected
            f.update(C=bool((a >> (cnt - 1)) & 1), Z=res == 0, S=bool(res & sign), P=_parity(res),
                     O=bool(a & sign) if cnt == 1 else None)
            return
        f.update(Z=res == 0, S=bool(res & sign), P=_parity(res))
        if kind == "logic":
            f.update(C=False, O=False)
        elif kind in ("add", "inc"):
            o = bool(~(a ^ b) & (a ^ res) & sign)
            f["O"] = o
            if kind == "add":
                f["C"] = (a & m) + (b & m) > m
        elif kind in ("sub", "dec"):
            f["O"] = bool((a ^ b) & (a ^ res) & sign)
            if kind == "sub":
                f["C"] = (a & m) < (b & m)

    # execution -------------------------------------------------------------------------------
    def _exec(self, ins: Lifted) -> None:
        t: dict[int, int] = {}
        self._t = t
        nxt = ins.address + ins.length
        for op in ins.ops:
            k = op[0]
            if k == "imm":
                t[op[1]] = op[2] & M64
            elif k == "addr":
                _, d, base, index, scale, disp, abits = op
                v = disp + (self.th.regs[base] if base is not None else 0)
                v += self.th.regs[index] * scale if index is not None else 0
                t[d] = v & _mask(abits)
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
                else:
                    raise LiftError(f"alu {o}")
                t[d] = v & m
            elif k == "flags":
                _, kind, a, b, r, bits = op
                self._set_flags(kind, t[a], t[b], t[r], bits)
            elif k == "set":
                self._store(op[2], int(self._cond(op[1])))
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
        except (LiftError, GuestFault, UndefinedFlag, Unimplemented, HandlerFailed) as e:
            stop = f"{type(e).__name__}: {e}"
        return Result(self.exit_code, bytes(self.kernel.out), self.steps, len(self.seen),
                      self.calls, stop)
