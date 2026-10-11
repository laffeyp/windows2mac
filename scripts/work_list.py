# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""work_list.py — the functions the game calls on Windows that w2m does not serve, as families (Sprint 049).

The game's run on a Windows runner (`.github/workflows/windows-game.yml`, tools/jit_capture) records every
statically imported function UnityPlayer.dll and mono-2.0-bdwgc.dll call, with counts (<capture>.imports.tsv),
every name they resolve with GetProcAddress (<capture>.dynamic.tsv), and, from Sprint 049, the first 32 calls of
each function in order (<capture>.trace.tsv). This script reads those, subtracts what w2m serves (vm_backend's
SERVICES and DLL_SERVICES), groups the rest by FAMILIES, and orders the families by the first call any member
receives in the trace: the order the game needs them. Names the game's own DLLs export (mono_*, the Unity plug-in
and Steam entry points) are not Windows functions; they resolve once those DLLs load and are listed apart.

A name no family claims stops the script: every function the game calls is placed by hand, once. An ordinal
import is named with its DLL ("OLEAUT32.dll!#2"): the same number means another function in another DLL.

Usage: uv run python scripts/work_list.py <capture-run-dir> [<capture-run-dir> ...] [--out specs/work_list.json]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
from w2m.vm_backend import DLL_SERVICES, SERVICES  # noqa: E402

VERSION = "0.1.0"

FAMILIES: dict[str, tuple[str, ...]] = {
    "run_time_loading": ("GetModuleFileNameA", "GetModuleHandleA", "SetDllDirectoryW", "AddDllDirectory",
                         "EnumProcessModules", "LoadLibraryA", "LoadLibraryExA", "GetModuleHandleExA"),
    "registry": ("RegCloseKey", "RegCreateKeyW", "RegDeleteValueA", "RegOpenKeyExW", "RegQueryValueExA",
                 "RegSetValueExA", "RegQueryValueExW", "RegSetValueExW", "RegCreateKeyExW", "RegOpenKeyExA"),
    "locale": ("GetLocaleInfoA", "GetThreadLocale", "GetUserDefaultLocaleName", "CompareStringEx",
               "EnumSystemLocalesEx", "GetDateFormatEx", "GetLocaleInfoEx", "GetTimeFormatEx", "IsValidLocaleName",
               "LCIDToLocaleName", "LocaleNameToLCID", "SystemTimeToTzSpecificLocalTime", "GetUserDefaultLCID",
               "GetUserDefaultUILanguage", "GetSystemDefaultLCID"),
    "messages": ("FormatMessageW", "FormatMessageA"),
    "file_search": ("FindFirstFileExW", "FindNextFileW", "FindClose", "FindFirstFileW"),
    "files_more": ("GetTempFileNameW", "GetFileSize", "GetFileInformationByHandleEx", "GetWindowsDirectoryW",
                   "SetHandleInformation", "GetFileType", "GetFinalPathNameByHandleW"),
    "file_mapping": ("CreateFileMappingW", "MapViewOfFile", "UnmapViewOfFile", "CreateFileMappingA"),
    "threads": ("CreateThread", "SuspendThread", "ResumeThread", "GetThreadContext", "SetThreadContext",
                "GetExitCodeThread", "OpenThread", "SetThreadPriority", "SwitchToThread", "SleepEx",
                "DuplicateHandle", "CreateSemaphoreW", "SetThreadDescription", "GetThreadPriority",
                "AppPolicyGetThreadInitializationType"),
    "exceptions": ("AddVectoredExceptionHandler", "RemoveVectoredExceptionHandler", "RaiseException", "RtlUnwindEx",
                   "RtlAddGrowableFunctionTable", "RtlGrowFunctionTable", "RtlDeleteGrowableFunctionTable",
                   "RtlAddFunctionTable", "RtlDeleteFunctionTable"),
    "interlocked_lists": ("InterlockedFlushSList", "InterlockedPopEntrySList", "InterlockedPushEntrySList",
                          "InitializeSListHead", "QueryDepthSList"),
    "time": ("timeGetTime", "timeBeginPeriod", "timeEndPeriod", "GetTickCount64", "GetTickCount",
             "GetSystemTimePreciseAsFileTime"),
    "com": ("CoCreateGuid", "CoCreateInstance", "CoInitialize", "CoInitializeEx", "CoUninitialize", "PropVariantClear",
            "PropVariantCopy", "CoTaskMemAlloc", "StringFromGUID2", "CoSetProxyBlanket",
            "CoCreateFreeThreadedMarshaler", "OLEAUT32.dll!#2", "OLEAUT32.dll!#6", "OLEAUT32.dll!#9"),
    "winrt": ("RoInitialize", "RoUninitialize", "RoGetActivationFactory", "RoActivateInstance", "RoGetAgileReference",
              "WindowsCompareStringOrdinal", "WindowsCreateString", "WindowsCreateStringReference",
              "WindowsDeleteString", "WindowsDuplicateString", "WindowsGetStringRawBuffer"),
    "windows": ("CreateWindowExW", "DefWindowProcW", "DispatchMessageA", "EnumDisplayDevicesA", "EnumDisplayMonitors",
                "EnumDisplaySettingsW", "GetClientRect", "GetFocus", "GetMessageExtraInfo", "GetMonitorInfoA",
                "GetMonitorInfoW", "GetSystemMetrics", "IsWindowVisible", "LoadCursorA", "LoadIconA", "LoadImageW",
                "MonitorFromWindow", "PeekMessageA", "RegisterClassExW", "RegisterClassW",
                "RegisterDeviceNotificationW", "SetFocus", "SetWindowLongPtrA", "SetWindowLongPtrW",
                "TranslateMessage", "GetDpiForMonitor", "MessageBoxA", "MessageBoxW", "ShowWindow", "UpdateWindow",
                "PeekMessageW", "DispatchMessageW", "GetWindowLongPtrW", "SetWindowPos", "GetWindowRect",
                "AdjustWindowRectEx", "ClientToScreen", "ScreenToClient", "GetCursorPos", "SetCursor",
                "ShowCursor", "ClipCursor", "GetForegroundWindow", "GetActiveWindow", "DestroyWindow",
                "UnregisterClassW", "PostQuitMessage", "GetKeyState", "GetAsyncKeyState", "MapVirtualKeyW",
                "ToUnicode", "GetKeyboardLayout", "SetWindowTextW", "GetDC", "ReleaseDC", "GetParent", "GetWindowLongA",
                "SetWindowLongA", "GetWindowPlacement", "IsIconic", "LoadIconW", "MonitorFromRect", "SendMessageW",
                "SystemParametersInfoW", "TrackMouseEvent", "UnregisterDeviceNotification", "ValidateRect"),
    "input": ("GetRawInputData", "GetRawInputDeviceList", "RegisterRawInputDevices", "GetPointerDevices",
              "GetPointerPenInfo", "RegisterTouchWindow", "GetCurrentInputMessageSource", "MapVirtualKeyExA",
              "ImmAssociateContext", "ImmGetContext", "ImmReleaseContext", "HidD_GetHidGuid",
              "SetupDiDestroyDeviceInfoList", "SetupDiEnumDeviceInfo", "SetupDiEnumDeviceInterfaces",
              "SetupDiGetClassDevsA", "SetupDiGetDeviceInterfaceDetailW", "XInputGetCapabilities", "XInputGetState",
              "XInputSetState"),
    "version": ("VerSetConditionMask", "RtlVerifyVersionInfo", "DllGetVersion", "VerifyVersionInfoW",
                "GetFileVersionInfoSizeW", "GetFileVersionInfoW", "VerQueryValueW"),
    "processes": ("CreatePipe", "CreateProcessW", "SetConsoleCtrlHandler", "GetConsoleOutputCP"),
    "network": ("WS2_32.dll!#115", "WSAStartup", "if_nametoindex"),
    "graphics": ("CreateDXGIFactory", "CreateDXGIFactory1", "CreateDXGIFactory2", "D3D11CreateDevice"),
}
GAME_EXPORT_PREFIXES = ("mono_", "unity_mono_", "Unity", "SteamAPI_")


def served() -> set[str]:
    return set(SERVICES) | {fn for table in DLL_SERVICES.values() for fn in table}


def read_run(run: Path) -> tuple[list[list[str]], list[str], list[list[str]]]:
    imports = [line.split("\t") for line in (run / "jit_capture.bin.imports.tsv").read_text().splitlines() if line]
    dynamic = [line for line in (run / "jit_capture.bin.dynamic.tsv").read_text().splitlines() if line]
    trace_file = run / "jit_capture.bin.trace.tsv"
    trace = [line.split("\t") for line in trace_file.read_text().splitlines() if line] if trace_file.exists() else []
    return imports, dynamic, trace


def build(runs: list[Path]) -> dict[str, Any]:
    have = served()
    owner = {fn: fam for fam, fns in FAMILIES.items() for fn in fns}
    functions: dict[str, dict[str, Any]] = {}
    game_exports: set[str] = set()

    def entry(fn: str, dll: str) -> dict[str, Any]:
        return functions.setdefault(fn, {"name": fn, "dll": dll, "callers": set(), "calls": 0, "first_seq": None,
                                         "dynamic": False})

    for run in runs:
        imports, dynamic, trace = read_run(run)
        for image, dll, fn, count in imports:
            if fn.startswith("#"):
                fn = f"{dll}!{fn}"
            if int(count) > 0 and fn not in have:
                e = entry(fn, dll)
                e["callers"].add(image)
                e["calls"] = max(e["calls"], int(count))
        for fn in dynamic:
            if fn.startswith(GAME_EXPORT_PREFIXES):
                game_exports.add(fn)
            elif fn not in have:
                entry(fn, "")["dynamic"] = True
        for row in trace:
            seq, _tid, image, dll, fn = row[:5]
            if fn.startswith("#"):
                fn = f"{dll}!{fn}"
            if image == "GetProcAddress" and fn in functions:
                functions[fn]["dll"] = functions[fn]["dll"] or dll
            if fn in functions:
                s = functions[fn]["first_seq"]
                functions[fn]["first_seq"] = int(seq) if s is None else min(s, int(seq))
    unplaced = sorted(fn for fn in functions if fn not in owner)
    if unplaced:
        raise SystemExit(f"no family claims: {', '.join(unplaced)}")
    families: dict[str, list[dict[str, Any]]] = {}
    for fn, e in functions.items():
        families.setdefault(owner[fn], []).append({**e, "callers": sorted(e["callers"])})

    def first(members: list[dict[str, Any]]) -> float:
        seqs = [m["first_seq"] for m in members if m["first_seq"] is not None]
        return min(seqs) if seqs else float("inf")

    ordered = sorted(families.items(), key=lambda kv: (first(kv[1]), kv[0]))
    return {
        "version": VERSION,
        "sources": [str(r.relative_to(ROOT)) if r.is_relative_to(ROOT) else str(r) for r in runs],
        "families": [{"name": name, "first_seq": None if first(m) == float("inf") else int(first(m)),
                      "functions": sorted(m, key=lambda x: (x["first_seq"] is None, x["first_seq"] or 0, x["name"]))}
                     for name, m in ordered],
        "game_exports": sorted(game_exports),
        "unserved": len(functions),
    }


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("runs", type=Path, nargs="+")
    ap.add_argument("--out", type=Path, default=ROOT / "specs" / "work_list.json")
    a = ap.parse_args(argv)
    spec = build([r.resolve() for r in a.runs])
    a.out.write_text(json.dumps(spec, indent=1) + "\n")
    for f in spec["families"]:
        print(f"{f['first_seq'] if f['first_seq'] is not None else '-':>8}  {f['name']:<18} "
              + " ".join(m["name"] for m in f["functions"]))
    print(f"{spec['unserved']} unserved Windows functions in {len(spec['families'])} families; "
          f"{len(spec['game_exports'])} names the game's own DLLs export")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
