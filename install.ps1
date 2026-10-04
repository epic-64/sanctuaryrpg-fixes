# Builds the proxy and installs it into the game directory.
# The original SDL.dll is kept as SDL_orig.dll; run with -Uninstall to put it back.
param(
    # Defaults to wherever Steam has the game installed.
    [string]$GameDir,
    [switch]$Uninstall
)
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "game-dir.ps1")
if (-not $GameDir) {
    $GameDir = Find-GameDir
}
if (-not (Test-Path (Join-Path $GameDir "SanctuaryRPG.exe"))) {
    throw "No SanctuaryRPG.exe in: $GameDir"
}

$sdl = Join-Path $GameDir "SDL.dll"
$orig = Join-Path $GameDir "SDL_orig.dll"

if ($Uninstall) {
    if (Test-Path $orig) {
        Remove-Item $sdl -Confirm:$false
        Rename-Item $orig "SDL.dll"
        Write-Host "Restored the original SDL.dll"
    } else {
        Write-Host "Nothing to uninstall"
    }
    return
}

cargo build --release
if ($LASTEXITCODE -ne 0) { throw "build failed" }

# Keep the prebuilt copy that install.sh uses on Linux up to date.
$dll = Join-Path $PSScriptRoot "target\i686-pc-windows-msvc\release\sanctuary_pad.dll"
New-Item -ItemType Directory -Force (Join-Path $PSScriptRoot "dist") | Out-Null
Copy-Item $dll (Join-Path $PSScriptRoot "dist\sanctuary_pad.dll") -Force

# Only the very first install sees the real SDL.dll under its own name.
if (-not (Test-Path $orig)) {
    Rename-Item $sdl "SDL_orig.dll"
}
Copy-Item $dll $sdl -Force

$ini = Join-Path $GameDir "sanctuary-pad.ini"
if (-not (Test-Path $ini)) {
    Copy-Item (Join-Path $PSScriptRoot "sanctuary-pad.ini") $ini
}
Write-Host "Installed to $GameDir"
