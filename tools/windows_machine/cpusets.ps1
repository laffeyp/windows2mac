# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# The logical processors and their core types (docs/windows-reference-machine-2026-10-12.md G1), for profile.ps1 and for
# run-job.ps1's pinning. GetSystemCpuSetInformation fills SYSTEM_CPU_SET_INFORMATION records: Size (DWORD, offset 0),
# Type (4), then CpuSet: Id (8), Group (12, WORD), LogicalProcessorIndex (14), CoreIndex (15),
# LastLevelCacheIndex (16), NumaNodeIndex (17), EfficiencyClass (18), AllFlags (19), SchedulingClass (20).
if (-not ('W2mCpuSets' -as [type])) {
  Add-Type -TypeDefinition @'
using System; using System.Runtime.InteropServices; using System.Collections.Generic;
public static class W2mCpuSets {
  [DllImport("kernel32.dll", SetLastError=true)]
  static extern bool GetSystemCpuSetInformation(IntPtr info, uint len, out uint ret, IntPtr proc, uint flags);
  // one entry per logical processor: { logical, core, efficiency_class, scheduling_class }
  public static int[][] Sets() {
    uint n; GetSystemCpuSetInformation(IntPtr.Zero, 0, out n, IntPtr.Zero, 0);
    IntPtr b = Marshal.AllocHGlobal((int)n);
    try {
      if (!GetSystemCpuSetInformation(b, n, out n, IntPtr.Zero, 0)) throw new System.ComponentModel.Win32Exception();
      var r = new List<int[]>(); int off = 0;
      while (off < n) {
        r.Add(new int[] { Marshal.ReadByte(b, off + 14), Marshal.ReadByte(b, off + 15), Marshal.ReadByte(b, off + 18),
                          Marshal.ReadByte(b, off + 20) });
        off += Marshal.ReadInt32(b, off);
      }
      return r.ToArray();
    } finally { Marshal.FreeHGlobal(b); }
  }
}
'@
}

# The affinity mask of the logical processors of one core type: 'performance' is the highest efficiency class,
# 'efficiency' the lowest (on a machine with one class, both are every processor).
function Get-W2mAffinityMask([string] $class) {
  $sets = [W2mCpuSets]::Sets()
  $classes = $sets | ForEach-Object { $_[2] } | Sort-Object -Unique
  $want = if ($class -eq 'performance') { $classes[-1] } else { $classes[0] }
  [uint64] $mask = 0
  foreach ($s in $sets) { if ($s[2] -eq $want) { $mask = $mask -bor ([uint64]1 -shl $s[0]) } }
  return $mask
}
