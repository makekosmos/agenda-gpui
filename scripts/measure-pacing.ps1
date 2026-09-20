param(
    [int]$Seconds = 25,
    [string]$Route = 'inbox',
    [int]$Tasks = 10000,
    [switch]$Scroll,
    [switch]$HighRateInput,
    [switch]$Background,
    [switch]$NoVsync,
    [switch]$CheckControls,
    [switch]$ToggleVsync,
    [switch]$CheckWake,
    [switch]$RestoreWake,
    [ValidateSet('auto', 'dwm')]
    [string]$VsyncSource = 'auto',
    [switch]$NoFrameLog,
    [switch]$Onscreen,
    [string]$Executable = (Join-Path $PSScriptRoot '..\target\debug\agenda-gpui.exe'),
    [double]$MinAverage = 164.5,
    [double]$MaxAverage = 166,
    [double]$MinP99 = 160
)
$ErrorActionPreference = 'Stop'
$env:AGENDA_OFFSCREEN = '1'
if ($Onscreen) { Remove-Item Env:AGENDA_OFFSCREEN }
$env:AGENDA_FPS = '1'
$env:AGENDA_FRAME_LOG = '1'
if ($NoFrameLog) { Remove-Item Env:AGENDA_FRAME_LOG }
$env:AGENDA_UNTHROTTLED = '1'
if ($Background) { Remove-Item Env:AGENDA_UNTHROTTLED }
$env:AGENDA_VSYNC = if ($NoVsync) { '0' } else { '1' }
$env:AGENDA_DEVTASKS = "$Tasks"
$env:AGENDA_ROUTE = $Route
$env:AGENDA_VSYNC_SOURCE = $VsyncSource
$env:AGENDA_PRESENT_TRACE = if ($CheckWake) { '1' } else { $null }
$wakeSamples = @()
$log = Join-Path $env:TEMP ('agenda-pacing-' + [guid]::NewGuid() + '.log')
if ($Scroll -or $CheckControls -or $ToggleVsync -or $CheckWake) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PacingInput {
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode)] static extern IntPtr CreateWaitableTimerExW(IntPtr attributes, string name, uint flags, uint access);
    [DllImport("kernel32.dll")] static extern bool SetWaitableTimer(IntPtr timer, ref long due, int period, IntPtr callback, IntPtr arg, bool resume);
    [DllImport("kernel32.dll")] static extern uint WaitForSingleObject(IntPtr handle, uint timeout);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
    static IntPtr timer;
    public static void WaitHighResolution() {
        if (timer == IntPtr.Zero) timer = CreateWaitableTimerExW(IntPtr.Zero, null, 2, 0x1F0003);
        long due = -80000; // 8 ms, in relative 100-ns units.
        if (timer == IntPtr.Zero || !SetWaitableTimer(timer, ref due, 0, IntPtr.Zero, IntPtr.Zero, false)
            || WaitForSingleObject(timer, 1000) != 0) throw new InvalidOperationException("High-resolution timer failed");
    }
    public static void CloseTimer() {
        if (timer != IntPtr.Zero) { CloseHandle(timer); timer = IntPtr.Zero; }
    }
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr hwnd, uint msg, UIntPtr w, IntPtr l);
    delegate bool EnumCallback(IntPtr hwnd, IntPtr data);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumCallback callback, IntPtr data);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr hwnd, System.Text.StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr hwnd, int command);
    [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y, int width, int height, uint flags);
    public static void Resize(IntPtr hwnd, int change) {
        Rect rect;
        if (!GetWindowRect(hwnd, out rect) || !SetWindowPos(hwnd, IntPtr.Zero, 0, 0,
            rect.Right - rect.Left + change, rect.Bottom - rect.Top, 0x16))
            throw new InvalidOperationException("Resize failed");
    }
    [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] static extern bool ClientToScreen(IntPtr hwnd, ref Point point);
    [DllImport("user32.dll")] static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll")] static extern IntPtr SendMessageTimeoutW(IntPtr hwnd, uint msg, UIntPtr w, IntPtr l, uint flags, uint timeout, out UIntPtr result);
    struct Point { public int X, Y; }
    public static void ClickEnergyToggle(IntPtr hwnd) {
        double scale = GetDpiForWindow(hwnd) / 96.0;
        IntPtr point = new IntPtr(((int)(110 * scale) << 16) | (int)(500 * scale));
        if (!PostMessageW(hwnd, 0x0200, UIntPtr.Zero, point)
            || !PostMessageW(hwnd, 0x0201, new UIntPtr(1), point)
            || !PostMessageW(hwnd, 0x0202, UIntPtr.Zero, point))
            throw new InvalidOperationException("Could not click energy toggle");
    }
    public static void CheckControls(IntPtr hwnd) {
        Rect client; if (!GetClientRect(hwnd, out client)) throw new InvalidOperationException("No client rect");
        double scale = GetDpiForWindow(hwnd) / 96.0;
        int[] offsets = { 0, 115, 69, 23 };
        int[] expected = { 2, 8, 9, 20 }; // HTCAPTION, HTMINBUTTON, HTMAXBUTTON, HTCLOSE.
        for (int i = 0; i < offsets.Length; i++) {
            Point point = new Point { X = i == 0 ? client.Right / 2 : client.Right - (int)(offsets[i] * scale), Y = (int)(20 * scale) };
            if (!ClientToScreen(hwnd, ref point)) throw new InvalidOperationException("No screen point");
            IntPtr lp = new IntPtr((point.Y << 16) | (point.X & 0xffff));
            UIntPtr result;
            if (SendMessageTimeoutW(hwnd, 0x0084, UIntPtr.Zero, lp, 2, 1000, out result) == IntPtr.Zero)
                throw new InvalidOperationException("WM_NCHITTEST timed out");
            if (result.ToUInt64() != (ulong)expected[i])
                throw new InvalidOperationException("Control " + i + ": expected hit " + expected[i] + ", got " + result.ToUInt64());
        }
    }
    public struct Rect { public int Left, Top, Right, Bottom; }
    public static IntPtr FindWindow(uint processId) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((hwnd, data) => {
            uint pid; GetWindowThreadProcessId(hwnd, out pid);
            var title = new System.Text.StringBuilder(256);
            GetWindowText(hwnd, title, title.Capacity);
            if (pid == processId && title.ToString() == "Agenda") { found = hwnd; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
'@
}
$windowStyle = if ($Onscreen) { 'Normal' } else { 'Hidden' }
$proc = Start-Process $Executable -WindowStyle $windowStyle -RedirectStandardError $log -PassThru
Write-Output "PID=$($proc.Id) LOG=$log route=$Route tasks=$Tasks scroll=$Scroll"
try {
    Start-Sleep -Seconds 5
    $proc.Refresh()
    if ($proc.HasExited) { throw "Application exited: $($proc.ExitCode)" }
    if ($Scroll -or $CheckControls -or $ToggleVsync -or $CheckWake) {
        $hwnd = [PacingInput]::FindWindow($proc.Id)
        if ($hwnd -eq [IntPtr]::Zero) { throw 'No window for scrolling' }
        $rect = New-Object PacingInput+Rect
        if (![PacingInput]::GetWindowRect($hwnd, [ref]$rect)) { throw 'No window bounds' }
        if ($Onscreen -and ![PacingInput]::IsWindowVisible($hwnd)) { throw 'Onscreen test window is hidden' }
        if ($CheckWake -and $Onscreen -and !$Background) {
            [void][PacingInput]::SetForegroundWindow($hwnd)
            Start-Sleep -Milliseconds 100
            if ([PacingInput]::GetForegroundWindow() -ne $hwnd) { throw 'Wake test needs foreground window' }
        }
        $x = [int](($rect.Left + $rect.Right) / 2) + 150
        $y = [int](($rect.Top + $rect.Bottom) / 2)
        $lparam = [IntPtr](($y -shl 16) -bor ($x -band 0xffff))
        if ($ToggleVsync) { [PacingInput]::ClickEnergyToggle($hwnd) }
    }
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $tick = 0
    if ($CheckWake) {
        # Vary the phase relative to the idle heartbeat, including its last 50 ms.
        foreach ($delay in @(1050, 1400, 1800, 1930, 1950, 1970)) {
            foreach ($resize in @($false, $true)) {
                if ($RestoreWake) { [void][PacingInput]::ShowWindowAsync($hwnd, 6) }
                Start-Sleep -Milliseconds $delay
                $start = [Diagnostics.Stopwatch]::GetTimestamp()
                if ($RestoreWake) { [void][PacingInput]::ShowWindowAsync($hwnd, 9) }
                if ($resize) { [PacingInput]::Resize($hwnd, $(if ($tick % 2) { -2 } else { 2 })); $tick++ }
                $burst = [Diagnostics.Stopwatch]::StartNew()
                while ($burst.ElapsedMilliseconds -lt 250) {
                    [void][PacingInput]::PostMessageW($hwnd, 0x020A, [UIntPtr]([uint32]120 -shl 16), $lparam)
                    [PacingInput]::WaitHighResolution()
                }
                $wakeSamples += [pscustomobject]@{ Start = $start; Resize = $resize; Idle = $delay }
                if ($Onscreen -and !$Background -and [PacingInput]::GetForegroundWindow() -ne $hwnd) { throw 'Wake test lost foreground window' }
                if ($Background -and [PacingInput]::GetForegroundWindow() -eq $hwnd) { throw 'Background wake test gained focus' }
            }
        }
        Start-Sleep -Milliseconds 1500
        $idleStart = [Diagnostics.Stopwatch]::GetTimestamp()
        Start-Sleep -Milliseconds 2200
        $idleEnd = [Diagnostics.Stopwatch]::GetTimestamp()
    }
    while (!$CheckWake -and $watch.Elapsed.TotalSeconds -lt $Seconds) {
        if ($CheckControls) { [PacingInput]::CheckControls($hwnd) }
        if ($Scroll) {
            $delta = if (($tick % 240) -lt 120) { -120 } else { 120 }
            $wparam = [UIntPtr]([uint32]($delta -band 0xffff) -shl 16)
            if (![PacingInput]::PostMessageW($hwnd, 0x020A, $wparam, $lparam)) {
                throw 'Could not post wheel input'
            }
            $tick++
        }
        if ($Scroll -and $HighRateInput) { [PacingInput]::WaitHighResolution() }
        else { Start-Sleep -Milliseconds 8 }
    }
    if ($Scroll) { Write-Output ("Scroll events={0} rate={1:F1}/s" -f $tick, ($tick / $watch.Elapsed.TotalSeconds)) }
} finally {
    if ($Scroll -or $CheckWake) { [PacingInput]::CloseTimer() }
    if (!$proc.HasExited) { Stop-Process -Id $proc.Id }
}
if ($CheckWake) {
    $presents = @(Select-String -Path $log -Pattern '\[present\] qpc=(\d+)' | ForEach-Object { [long]$_.Matches[0].Groups[1].Value })
    $frequency = [Diagnostics.Stopwatch]::Frequency
    $failed = $false
    $minWakeFrames = if ($NoVsync) { 12 } else { 30 }
    $maxWakeMs = if ($NoVsync) { 50 } else { 30 }
    foreach ($sample in $wakeSamples) {
        $deltas = @($presents | Where-Object { $_ -ge $sample.Start -and $_ -lt $sample.Start + $frequency / 4 } | ForEach-Object { ($_ - $sample.Start) * 1000.0 / $frequency })
        $first = if ($deltas.Count) { $deltas[0] } else { 1000 }
        Write-Output ('wake idle={0}ms resize={1} first={2:F1}ms frames/250ms={3}' -f $sample.Idle, $sample.Resize, $first, $deltas.Count)
        if ($first -gt $maxWakeMs -or $deltas.Count -lt $minWakeFrames) { $failed = $true }
    }
    $idleFrames = @($presents | Where-Object { $_ -ge $idleStart -and $_ -lt $idleEnd }).Count
    Write-Output "idle frames/2200ms=$idleFrames (expected 2..3)"
    if ($idleFrames -lt 2 -or $idleFrames -gt 3) { $failed = $true }
    if ($failed) { throw 'Slow wake from idle' }
    Write-Output 'PASS: idle wake'
    exit 0
}
$lines = @(Select-String -Path $log -Pattern '\[fps\]')
if (!$lines.Count) { Get-Content $log -Tail 20; throw 'No FPS samples' }
$lines | Select-Object -Last 3 | ForEach-Object { $_.Line }
$last = $lines[-1].Line
if ($last -notmatch 'avg=([\d.]+) 1%=([\d.]+)') { throw 'Unrecognized FPS sample' }
$culture = [Globalization.CultureInfo]::InvariantCulture
$avg = [double]::Parse($Matches[1], $culture)
$p99 = [double]::Parse($Matches[2], $culture)
if ($avg -lt $MinAverage -or $avg -gt $MaxAverage -or $p99 -lt $MinP99) {
    Write-Output "FAIL: avg=$avg (range $MinAverage..$MaxAverage), p99 FPS=$p99 (min $MinP99)"
    exit 1
}
Write-Output 'PASS'
