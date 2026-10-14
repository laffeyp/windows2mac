// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
// Gameplay input for play-mode runs (docs/windows-reference-machine.md R5): a Recorder that takes the keyboard and
// mouse as raw input while the game has the focus, and a Player that sends recorded events with SendInput. An event
// is (seconds since the game started, kind, values): key (scan code, extended, up), move (dx, dy, relative counts),
// button (left, right, middle; up), wheel (delta). run-job.ps1 loads this with Add-Type.
//
// Layouts, x64 (winuser.h):
//   RAWINPUTHEADER: DWORD dwType @0, DWORD dwSize @4, HANDLE hDevice @8, WPARAM wParam @16; 24 bytes.
//   RAWMOUSE @24: USHORT usFlags @24, (pad), USHORT usButtonFlags @28, USHORT usButtonData @30,
//     ULONG ulRawButtons @32, LONG lLastX @36, LONG lLastY @40.
//   RAWKEYBOARD @24: USHORT MakeCode @24, USHORT Flags @26 (RI_KEY_BREAK 1, RI_KEY_E0 2), USHORT Reserved @28,
//     USHORT VKey @30.
//   INPUT: DWORD type @0, union @8; sizeof 40. MOUSEINPUT: dx @8, dy @12, mouseData @16, dwFlags @20, time @24,
//     dwExtraInfo @32. KEYBDINPUT: wVk @8, wScan @10, dwFlags @12, time @16, dwExtraInfo @24.
// The round trip (a script played while recorded, the recording equal to it) is the check on these offsets.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Windows.Forms;

public sealed class W2mInputEvent {
    public double T; public string Kind; public int Scan; public bool E0; public bool Up; public int Dx; public int Dy;
    public string Button; public int Delta;

    public string Json() {
        var c = CultureInfo.InvariantCulture;
        switch (Kind) {
            case "key": return string.Format(c, "{{\"t\":{0:F3},\"kind\":\"key\",\"scan\":{1},\"e0\":{2},\"up\":{3}}}", T, Scan, E0 ? "true" : "false", Up ? "true" : "false");
            case "move": return string.Format(c, "{{\"t\":{0:F3},\"kind\":\"move\",\"dx\":{1},\"dy\":{2}}}", T, Dx, Dy);
            case "button": return string.Format(c, "{{\"t\":{0:F3},\"kind\":\"button\",\"button\":\"{1}\",\"up\":{2}}}", T, Button, Up ? "true" : "false");
            default: return string.Format(c, "{{\"t\":{0:F3},\"kind\":\"wheel\",\"delta\":{1}}}", T, Delta);
        }
    }
}

public sealed class W2mRecorder : Form {
    [StructLayout(LayoutKind.Sequential)]
    struct RAWINPUTDEVICE { public ushort UsagePage; public ushort Usage; public uint Flags; public IntPtr Target; }
    [DllImport("user32.dll", SetLastError = true)]
    static extern bool RegisterRawInputDevices(RAWINPUTDEVICE[] devices, uint count, uint size);
    [DllImport("user32.dll")]
    static extern uint GetRawInputData(IntPtr input, uint command, IntPtr data, ref uint size, uint headerSize);
    const int WM_INPUT = 0x00FF; const uint RID_INPUT = 0x10000003; const uint RIDEV_INPUTSINK = 0x100;

    readonly Stopwatch clock;
    readonly double offset;
    public readonly List<W2mInputEvent> Recorded = new List<W2mInputEvent>();

    W2mRecorder(Stopwatch clock, double offset) {
        this.clock = clock; this.offset = offset;
        ShowInTaskbar = false; FormBorderStyle = FormBorderStyle.None; Opacity = 0; Size = new System.Drawing.Size(1, 1);
    }

    protected override void OnHandleCreated(EventArgs e) {
        base.OnHandleCreated(e);
        var devices = new[] {
            new RAWINPUTDEVICE { UsagePage = 1, Usage = 2, Flags = RIDEV_INPUTSINK, Target = Handle },    // mouse
            new RAWINPUTDEVICE { UsagePage = 1, Usage = 6, Flags = RIDEV_INPUTSINK, Target = Handle } };  // keyboard
        if (!RegisterRawInputDevices(devices, 2, (uint)Marshal.SizeOf(typeof(RAWINPUTDEVICE))))
            throw new System.ComponentModel.Win32Exception();
    }

    protected override void WndProc(ref Message m) {
        if (m.Msg == WM_INPUT) {
            uint size = 0;
            GetRawInputData(m.LParam, RID_INPUT, IntPtr.Zero, ref size, 24);
            IntPtr b = Marshal.AllocHGlobal((int)size);
            try {
                if (GetRawInputData(m.LParam, RID_INPUT, b, ref size, 24) == size) {
                    double t = offset + clock.Elapsed.TotalSeconds;
                    int type = Marshal.ReadInt32(b, 0);
                    lock (Recorded) {
                        if (type == 0) Mouse(b, t); else if (type == 1) Keyboard(b, t);
                    }
                }
            } finally { Marshal.FreeHGlobal(b); }
        }
        base.WndProc(ref m);
    }

    void Mouse(IntPtr b, double t) {
        ushort flags = (ushort)Marshal.ReadInt16(b, 24);
        ushort buttons = (ushort)Marshal.ReadInt16(b, 28);
        short data = Marshal.ReadInt16(b, 30);
        int dx = Marshal.ReadInt32(b, 36), dy = Marshal.ReadInt32(b, 40);
        if ((flags & 1) == 0 && (dx != 0 || dy != 0))             // MOUSE_MOVE_RELATIVE is 0
            Recorded.Add(new W2mInputEvent { T = t, Kind = "move", Dx = dx, Dy = dy });
        string[] names = { "left", "right", "middle" };
        for (int i = 0; i < 3; i++) {
            if ((buttons & (1 << (2 * i))) != 0) Recorded.Add(new W2mInputEvent { T = t, Kind = "button", Button = names[i], Up = false });
            if ((buttons & (2 << (2 * i))) != 0) Recorded.Add(new W2mInputEvent { T = t, Kind = "button", Button = names[i], Up = true });
        }
        if ((buttons & 0x400) != 0) Recorded.Add(new W2mInputEvent { T = t, Kind = "wheel", Delta = data });
    }

    void Keyboard(IntPtr b, double t) {
        ushort make = (ushort)Marshal.ReadInt16(b, 24), flags = (ushort)Marshal.ReadInt16(b, 26);
        Recorded.Add(new W2mInputEvent { T = t, Kind = "key", Scan = make, E0 = (flags & 2) != 0, Up = (flags & 1) != 0 });
    }

    // A recorder on its own thread with its own message loop; Stop closes it and returns the events as JSON lines.
    public static W2mRecorder Start(Stopwatch clock, double offset) {
        W2mRecorder r = null; var ready = new ManualResetEvent(false);
        var thread = new Thread(() => { r = new W2mRecorder(clock, offset); r.Load += (s, e) => ready.Set(); Application.Run(r); });
        thread.SetApartmentState(ApartmentState.STA); thread.IsBackground = true; thread.Start();
        ready.WaitOne(10000);
        return r;
    }

    public string Stop() {
        Invoke(new Action(Close));
        var sb = new StringBuilder();
        lock (Recorded) { foreach (var e in Recorded) sb.Append(e.Json()).Append('\n'); }
        return sb.ToString();
    }
}

public static class W2mPlayer {
    [StructLayout(LayoutKind.Explicit, Size = 40)]
    struct INPUT {
        [FieldOffset(0)] public uint Type;
        [FieldOffset(8)] public int Dx; [FieldOffset(12)] public int Dy; [FieldOffset(16)] public uint MouseData;
        [FieldOffset(20)] public uint MouseFlags;
        [FieldOffset(8)] public ushort Vk; [FieldOffset(10)] public ushort Scan; [FieldOffset(12)] public uint KeyFlags;
    }
    [DllImport("user32.dll", SetLastError = true)]
    static extern uint SendInput(uint count, INPUT[] inputs, int size);

    public static void Send(string kind, int scan, bool e0, bool up, int dx, int dy, string button, int delta) {
        var i = new INPUT();
        switch (kind) {
            case "key":
                i.Type = 1; i.Scan = (ushort)scan;
                i.KeyFlags = 0x8 | (e0 ? 0x1u : 0) | (up ? 0x2u : 0);       // SCANCODE, EXTENDEDKEY, KEYUP
                break;
            case "move":
                i.Type = 0; i.Dx = dx; i.Dy = dy; i.MouseFlags = 0x1;          // MOVE, relative
                break;
            case "button":
                i.Type = 0;
                uint down = button == "left" ? 0x2u : button == "right" ? 0x8u : 0x20u;
                i.MouseFlags = up ? down << 1 : down;                           // LEFTUP 0x4, RIGHTUP 0x10, MIDDLEUP 0x40
                break;
            default:
                i.Type = 0; i.MouseFlags = 0x800; i.MouseData = (uint)delta;      // WHEEL
                break;
        }
        if (SendInput(1, new[] { i }, 40) != 1) throw new System.ComponentModel.Win32Exception();
    }
}
