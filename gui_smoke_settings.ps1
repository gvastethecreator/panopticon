$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
if (-not ('Win32Smoke2' -as [type])) {
Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public static class Win32Smoke2 {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr hWnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder lpString, int nMaxCount);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

    public static string[] GetVisibleTopLevelWindowTitlesForPid(int pid) {
        var titles = new List<string>();
        EnumWindows(delegate (IntPtr hWnd, IntPtr lParam) {
            uint windowPid;
            GetWindowThreadProcessId(hWnd, out windowPid);
            if (windowPid == pid && IsWindowVisible(hWnd)) {
                int len = GetWindowTextLength(hWnd);
                var sb = new StringBuilder(len + 1);
                GetWindowText(hWnd, sb, sb.Capacity);
                titles.Add(sb.ToString());
            }
            return true;
        }, IntPtr.Zero);
        return titles.ToArray();
    }

    public static IntPtr GetFirstVisibleTopLevelWindowForPid(int pid) {
        IntPtr found = IntPtr.Zero;
        EnumWindows(delegate (IntPtr hWnd, IntPtr lParam) {
            uint windowPid;
            GetWindowThreadProcessId(hWnd, out windowPid);
            if (windowPid == pid && IsWindowVisible(hWnd)) {
                found = hWnd;
                return false;
            }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
"@
}

$step = '1'
$titles = @()
$proc = $null
try {
    Get-Process -Name 'panopticon' -ErrorAction SilentlyContinue | Stop-Process -Force

    $step = '2'
    $proc = Start-Process -FilePath ".\target\debug\panopticon.exe" -WorkingDirectory "$PWD" -PassThru
    if (-not $proc) { throw 'Start-Process returned no process object.' }

    $step = '3'
    try {
        $null = $proc.WaitForInputIdle(15000)
    } catch {
        throw "WaitForInputIdle failed: $($_.Exception.Message)"
    }
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $hwnd = [IntPtr]::Zero
    while ($sw.ElapsedMilliseconds -lt 15000) {
        $proc.Refresh()
        if ($proc.HasExited) { throw 'Process exited before a visible top-level window was found.' }
        $hwnd = [Win32Smoke2]::GetFirstVisibleTopLevelWindowForPid($proc.Id)
        if ($hwnd -ne [IntPtr]::Zero) { break }
    }
    if ($hwnd -eq [IntPtr]::Zero) { throw 'No visible top-level window found after input-idle.' }

    $step = '4'
    [Win32Smoke2]::ShowWindow($hwnd, 5) | Out-Null
    if (-not [Win32Smoke2]::SetForegroundWindow($hwnd)) { throw 'SetForegroundWindow returned false.' }
    [System.Windows.Forms.SendKeys]::SendWait('O')

    $step = '5'
    $deadline = [System.Diagnostics.Stopwatch]::StartNew()
    do {
        $proc.Refresh()
        $titles = [Win32Smoke2]::GetVisibleTopLevelWindowTitlesForPid($proc.Id)
    } while (-not $proc.HasExited -and $deadline.ElapsedMilliseconds -lt 5000 -and -not ($titles | Where-Object { $_ -match 'Settings' }))

    $step = '6'
    $proc.Refresh()
    $alive = -not $proc.HasExited
    $hasSettings = [bool]($titles | Where-Object { $_ -match 'Settings' })
    "RESULT StepFailed=None"
    "RESULT ProcessId=$($proc.Id)"
    "RESULT StayedAliveAfterO=$alive"
    "RESULT AnyTitleContainsSettings=$hasSettings"
    if ($titles.Count -eq 0) {
        'RESULT WindowTitles=<none>'
    } else {
        $titles | ForEach-Object { "TITLE $_" }
    }
}
catch {
    if ($proc) {
        try {
            if (-not $proc.HasExited) { $proc.Refresh(); $titles = [Win32Smoke2]::GetVisibleTopLevelWindowTitlesForPid($proc.Id) }
        } catch {}
    }
    "RESULT StepFailed=$step"
    "RESULT Error=$($_.Exception.Message)"
    if ($titles.Count -eq 0) {
        'RESULT WindowTitles=<none>'
    } else {
        $titles | ForEach-Object { "TITLE $_" }
    }
}
finally {
    if ($proc) {
        try { if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force } } catch {}
    }
}
