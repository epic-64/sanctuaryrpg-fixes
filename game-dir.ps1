# Shared by install.ps1 and probe.ps1: locates the game in the Steam libraries.

function Find-GameDir {
    $gameName = "SanctuaryRPG - Black Edition"

    # Steam's own folder, from the registry.
    $roots = @()
    foreach ($key in "HKCU:\Software\Valve\Steam", "HKLM:\SOFTWARE\WOW6432Node\Valve\Steam", "HKLM:\SOFTWARE\Valve\Steam") {
        $properties = Get-ItemProperty -Path $key -ErrorAction SilentlyContinue
        if ($properties) {
            $roots += $properties.SteamPath, $properties.InstallPath
        }
    }

    # Every library that Steam knows about, including ones on other drives.
    $libraries = @()
    foreach ($root in $roots) {
        if (-not $root) { continue }
        $libraries += $root
        $vdf = Join-Path $root "steamapps\libraryfolders.vdf"
        if (Test-Path $vdf) {
            foreach ($line in Get-Content $vdf) {
                if ($line -match '^\s*"path"\s+"(.*)"\s*$') {
                    $libraries += $Matches[1].Replace('\\', '\')
                }
            }
        }
    }

    foreach ($library in $libraries) {
        $candidate = Join-Path $library "steamapps\common\$gameName"
        if (Test-Path (Join-Path $candidate "SanctuaryRPG.exe")) {
            return (Get-Item $candidate).FullName
        }
    }
    throw "Could not find `"$gameName`" in any Steam library. Pass its folder with -GameDir."
}
