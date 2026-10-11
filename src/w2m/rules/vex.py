# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The VEX classes mono-2.0-bdwgc.dll reaches (Intel SDM Vol 2, each instruction's VEX form; Sprint 049): one
module, so one candidate for the set's item run, until a class needs its own."""

from w2m.rules import _vex

ICLASSES = tuple(sorted(set(_vex.BASE) | {f for f in _vex.FMA if f in {
    "VFMADD213SD", "VFMADD231SD", "VFMADD132SD", "VFNMADD231SD", "VFNMADD132SD", "VFNMADD213SD", "VFMSUB213SD",
    "VFMSUB132SD", "VFMADD132PD", "VFMADD213SS", "VFMADD231SS", "VFMSUB213SS", "VFMADD231PS", "VFNMADD231PS"}}
    | {"VZEROUPPER", "VPERMILPS"} | _vex.CROSS))
SHARED = ("_vex", "_sse")
rule = _vex.rule
lower = _vex.lower
