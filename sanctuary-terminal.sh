#!/usr/bin/env bash
# Plays SanctuaryRPG inside the terminal this is run from, instead of in its own window
# (Linux / Steam Deck, through the Proton the game normally runs with).
# Needs sanctuary-pad to be installed (install.sh) and the game to have been started
# once from Steam, so that its Proton prefix exists.
#
# Usage: ./sanctuary-terminal.sh [--game-dir DIR] [--proton DIR]
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
game_name="SanctuaryRPG - Black Edition"
app_id=328760
game_dir=""
proton_dir=""

while [ $# -gt 0 ]; do
    case "$1" in
        --game-dir) game_dir="${2:?--game-dir needs a folder}"; shift 2 ;;
        --proton) proton_dir="${2:?--proton needs a folder}"; shift 2 ;;
        -h|--help) sed -n '2,7p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
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
    local library libraries
    mapfile -t libraries < <(steam_libraries)
    for library in "${libraries[@]}"; do
        if [ -f "$library/steamapps/common/$game_name/SanctuaryRPG.exe" ]; then
            echo "$library/steamapps/common/$game_name"
            return 0
        fi
    done
    return 1
}

# The Proton prefix Steam made for the game.
find_compat_data() {
    local library libraries
    mapfile -t libraries < <(steam_libraries)
    for library in "${libraries[@]}"; do
        if [ -d "$library/steamapps/compatdata/$app_id/pfx" ]; then
            echo "$library/steamapps/compatdata/$app_id"
            return 0
        fi
    done
    return 1
}

# Every installed Proton: Valve's in the Steam libraries, others (GE-Proton) in
# compatibilitytools.d.
proton_dirs() {
    local library libraries dir
    mapfile -t libraries < <(steam_libraries)
    for library in "${libraries[@]}"; do
        for dir in "$library/steamapps/common"/Proton*/ "$library/compatibilitytools.d"/*/; do
            [ -f "$dir/proton" ] && echo "${dir%/}"
        done
    done
    for dir in "$HOME/.steam/root/compatibilitytools.d"/*/; do
        [ -f "$dir/proton" ] && echo "${dir%/}"
    done
    return 0
}

# Picks the Proton that made the prefix (its prefix version matches), otherwise
# Proton Experimental, otherwise whichever is found first.
find_proton() {
    local compat_data="$1" wanted="" dir dirs version fallback=""
    [ -f "$compat_data/version" ] && wanted="$(tr -d '[:space:]' < "$compat_data/version")"
    mapfile -t dirs < <(proton_dirs | sort -u)
    for dir in "${dirs[@]}"; do
        [ -n "$dir" ] || continue
        version="$(sed -n 's/^CURRENT_PREFIX_VERSION="\(.*\)"$/\1/p' "$dir/proton" | head -n 1)"
        if [ -n "$wanted" ] && [ "$version" = "$wanted" ]; then
            echo "$dir"
            return 0
        fi
        case "$(basename "$dir")" in
            "Proton - Experimental") fallback="$dir" ;;
            *) [ -n "$fallback" ] || fallback="$dir" ;;
        esac
    done
    [ -n "$fallback" ] && echo "$fallback"
}

if [ -z "$game_dir" ]; then
    if [ -f "$script_dir/SanctuaryRPG.exe" ]; then
        game_dir="$script_dir"
    else
        game_dir="$(find_game_dir)" || {
            echo "Could not find \"$game_name\" in any Steam library." >&2
            echo "Pass its folder with --game-dir." >&2
            exit 1
        }
    fi
fi
if [ ! -f "$game_dir/SanctuaryRPG.exe" ]; then
    echo "No SanctuaryRPG.exe in: $game_dir" >&2
    exit 1
fi
if [ ! -f "$game_dir/SDL_orig.dll" ]; then
    echo "sanctuary-pad is not installed in: $game_dir" >&2
    echo "Run install.sh first." >&2
    exit 1
fi

compat_data="$(find_compat_data)" || {
    echo "The game has no Proton prefix yet." >&2
    echo "Start it once from Steam (and quit), then run this script again." >&2
    exit 1
}

if [ -z "$proton_dir" ]; then
    proton_dir="$(find_proton "$compat_data")"
    if [ -z "$proton_dir" ]; then
        echo "Could not find a Proton installation. Pass its folder with --proton." >&2
        exit 1
    fi
fi
wine=""
for candidate in "$proton_dir/files/bin/wine" "$proton_dir/dist/bin/wine"; do
    if [ -x "$candidate" ]; then
        wine="$candidate"
        break
    fi
done
if [ -z "$wine" ]; then
    echo "No wine binary under: $proton_dir" >&2
    exit 1
fi
if [ ! -t 0 ] || [ ! -t 1 ]; then
    echo "Run this from a terminal: the game is drawn into it." >&2
    exit 1
fi

# Wine hands the game this terminal as its standard input and output, and the proxy draws
# into it directly. The terminal has to be in raw mode for that, and since the game cannot
# ask the terminal for its size from inside Wine, it is kept in a file that is rewritten
# whenever the window is resized. Wine's own chatter goes to a log instead of the screen.
size_file="$(mktemp "${TMPDIR:-/tmp}/sanctuary-terminal.XXXXXX")"
saved_tty="$(stty -g)"
restore() {
    stty "$saved_tty"
    rm -f "$size_file"
}
trap restore EXIT
write_size() {
    stty size > "$size_file"
}
trap write_size WINCH
write_size
stty raw -echo

log="$game_dir/sanctuary-terminal.log"
cd "$game_dir"
SANCTUARY_PAD_TERMINAL="Z:$(printf '%s' "$size_file" | tr / '\\')" \
    WINEPREFIX="$compat_data/pfx" WINEDEBUG=-all \
    "$wine" SanctuaryRPG.exe <&0 2> "$log" &   # <&0: a background job would get /dev/null
game=$!
# `wait` returns early when the resize trap runs, so wait until the game is really gone.
status=0
while :; do
    if wait "$game"; then status=0; else status=$?; fi
    kill -0 "$game" 2> /dev/null || break
done
stty "$saved_tty"
if [ "$status" -ne 0 ]; then
    echo "The game exited with status $status. Last lines of $log:" >&2
    tail -n 20 "$log" >&2
fi
exit "$status"
