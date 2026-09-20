$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Diagnostics;
using System.Runtime.InteropServices;
public static class CompositorProbe {
    [DllImport("dwmapi.dll")] static extern int DwmFlush();
    [DllImport("dcomp.dll")] static extern uint DCompositionWaitForCompositorClock(uint count, IntPtr handles, uint timeout);
    public static void Run(bool clock) {
        var deltas = new double[1000];
        int errors = 0;
        long last = Stopwatch.GetTimestamp();
        for (int i = -100; i < deltas.Length; i++) {
            uint result = clock ? DCompositionWaitForCompositorClock(0, IntPtr.Zero, 1000) : unchecked((uint)DwmFlush());
            if (result != 0) errors++;
            long now = Stopwatch.GetTimestamp();
            if (i >= 0) deltas[i] = (now - last) * 1000.0 / Stopwatch.Frequency;
            last = now;
        }
        Array.Sort(deltas);
        double sum = 0;
        foreach (var d in deltas) sum += d;
        Console.WriteLine("{0}: mean={1:F3}ms p99={2:F3}ms max={3:F3}ms errors={4}", clock ? "DCompClock" : "DwmFlush", sum/deltas.Length, deltas[989], deltas[999], errors);
    }
}
'@
[CompositorProbe]::Run($false)
[CompositorProbe]::Run($true)
