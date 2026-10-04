# Debug helper: runs the game with scripted controller presses and prints the screens it saw.
param([string]$Buttons = "a", [int]$Seconds = 15, [int]$Tail = 200, [string]$Screenshot,
      [string]$GameDir = "E:\SteamBlitz\steamapps\common\SanctuaryRPG - Black Edition")
Get-Process SanctuaryRPG -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500
powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot\install.ps1" -GameDir $GameDir | Select-Object -Last 1
Remove-Item "$GameDir\sanctuary-pad-screen.txt" -ErrorAction SilentlyContinue
$env:SANCTUARY_PAD_SIMULATE = $Buttons
$p = Start-Process "$GameDir\SanctuaryRPG.exe" -WorkingDirectory $GameDir -PassThru
Start-Sleep -Seconds $Seconds
$p.Refresh(); "alive: $(-not $p.HasExited)"
if ($Screenshot -and -not $p.HasExited) {
    # PrintWindow captures the game window even when other windows cover it.
    Add-Type -AssemblyName System.Drawing
    Add-Type 'using System; using System.Runtime.InteropServices;
        public class ProbeWin { [StructLayout(LayoutKind.Sequential)] public struct Rect { public int L, T, R, B; }
        [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out Rect r);
        [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags); }'
    $r = New-Object ProbeWin+Rect
    [void][ProbeWin]::GetWindowRect($p.MainWindowHandle, [ref]$r)
    $bmp = New-Object System.Drawing.Bitmap ($r.R - $r.L), ($r.B - $r.T)
    $gfx = [System.Drawing.Graphics]::FromImage($bmp)
    $dc = $gfx.GetHdc()
    [void][ProbeWin]::PrintWindow($p.MainWindowHandle, $dc, 2)
    $gfx.ReleaseHdc($dc)
    $bmp.Save($Screenshot)
}
if (-not $p.HasExited) { Stop-Process $p -Force }
"--- log ---"; Get-Content "$GameDir\sanctuary-pad.log" -ErrorAction SilentlyContinue
"--- screen ---"; Get-Content "$GameDir\sanctuary-pad-screen.txt" -ErrorAction SilentlyContinue | Select-Object -Last $Tail
