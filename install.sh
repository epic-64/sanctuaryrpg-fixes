#!/usr/bin/env bash
# Installs the proxy into the game's folder on Linux / Steam Deck (the game runs
# through Proton). The game's own SDL.dll is kept as SDL_orig.dll.
#
# This script does not build anything. It installs the prebuilt dist/sanctuary_pad.dll
# that is checked into the repository (install.ps1 refreshes it on Windows), or a
# sanctuary_pad.dll placed next to this script.
#
# Usage: ./install.sh [--game-dir DIR] [--dll FILE] [--uninstall]
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
game_name="SanctuaryRPG - Black Edition"
game_dir=""
dll=""
uninstall=false

while [ $# -gt 0 ]; do
    case "$1" in
        --game-dir) game_dir="${2:?--game-dir needs a folder}"; shift 2 ;;
        --dll) dll="${2:?--dll needs a file}"; shift 2 ;;
        --uninstall) uninstall=true; shift ;;
        -h|--help) sed -n '2,9p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "Unknown option: $1" >&2; exit 2 ;;
    esac
done

# Prints every Steam library folder: the usual install locations, plus the libraries
# they list (SD cards, extra drives).
steam_libraries() {
    local root
    for root in \
        "$HOME/.local/share/Steam" \
        "$HOME/.steam/steam" \
        "$HOME/.steam/root" \
        "$HOME/.var/app/com.valvesoftware.Steam/.local/share/Steam"; do
        [ -d "$root/steamapps" ] || continue
        echo "$root"
        if [ -f "$root/steamapps/libraryfolders.vdf" ]; then
            sed -n 's/^[[:space:]]*"path"[[:space:]]*"\(.*\)"[[:space:]]*$/\1/p' \
                "$root/steamapps/libraryfolders.vdf"
        fi
    done
}

find_game_dir() {
    local library
    while IFS= read -r library; do
        if [ -f "$library/steamapps/common/$game_name/SanctuaryRPG.exe" ]; then
            echo "$library/steamapps/common/$game_name"
            return 0
        fi
    done < <(steam_libraries)
    return 1
}

if [ -z "$game_dir" ]; then
    game_dir="$(find_game_dir)" || {
        echo "Could not find \"$game_name\" in any Steam library." >&2
        echo "Pass its folder with --game-dir." >&2
        exit 1
    }
fi
if [ ! -f "$game_dir/SanctuaryRPG.exe" ]; then
    echo "No SanctuaryRPG.exe in: $game_dir" >&2
    exit 1
fi
if pgrep -f "SanctuaryRPG.exe" > /dev/null 2>&1; then
    echo "The game is running. Close it first." >&2
    exit 1
fi

sdl="$game_dir/SDL.dll"
orig="$game_dir/SDL_orig.dll"

if $uninstall; then
    if [ -f "$orig" ]; then
        mv -f "$orig" "$sdl"
        echo "Restored the original SDL.dll in: $game_dir"
    else
        echo "Nothing to uninstall in: $game_dir"
    fi
    exit 0
fi

if [ -z "$dll" ]; then
    for candidate in \
        "$script_dir/sanctuary_pad.dll" \
        "$script_dir/dist/sanctuary_pad.dll" \
        "$script_dir/target/i686-pc-windows-msvc/release/sanctuary_pad.dll"; do
        if [ -f "$candidate" ]; then
            dll="$candidate"
            break
        fi
    done
fi
if [ -z "$dll" ] || [ ! -f "$dll" ]; then
    echo "Could not find sanctuary_pad.dll." >&2
    echo "It should be in dist/ of the repository. Otherwise build it on Windows with" >&2
    echo "install.ps1 and copy dist/sanctuary_pad.dll next to this script, or pass its" >&2
    echo "location with --dll." >&2
    exit 1
fi

# Only the very first install sees the real SDL.dll under its own name. After Steam
# has verified the game files, SDL.dll is the original again while SDL_orig.dll is
# still there; the copy below then simply replaces it.
if [ ! -f "$orig" ]; then
    if [ ! -f "$sdl" ]; then
        echo "The game folder has no SDL.dll. Verify the game files in Steam first." >&2
        exit 1
    fi
    if cmp -s "$dll" "$sdl"; then
        echo "SDL.dll is already the proxy but SDL_orig.dll is missing." >&2
        echo "Verify the game files in Steam, then run this script again." >&2
        exit 1
    fi
    mv "$sdl" "$orig"
fi
cp -f "$dll" "$sdl"

if [ ! -f "$game_dir/sanctuary-pad.ini" ]; then
    cp "$script_dir/sanctuary-pad.ini" "$game_dir/sanctuary-pad.ini"
fi

echo "Installed to: $game_dir"
echo "In Steam, set the game's controller layout to the Gamepad template."
