#!/usr/bin/env bash
# Packs everything a player needs into dist/sanctuary-fixes-<VERSION>.zip for upload.
# The zip keeps the repository layout, so the install scripts work unchanged from the
# extracted folder. Nothing is built: the DLL comes from prebuilt/.
#
# Usage: scripts/distribute.sh
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

version="$(tr -d '[:space:]' < VERSION)"
cargo_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
if [ "$version" != "$cargo_version" ]; then
    echo "VERSION ($version) and Cargo.toml ($cargo_version) disagree" >&2
    exit 1
fi

files=(
    VERSION
    LICENSE
    README.md
    docs/install.md
    docs/controls.md
    docs/terminal-mode.md
    install.ps1
    install.sh
    game-dir.ps1
    prebuilt/sanctuary_pad.dll
    sanctuary-pad.ini
    sanctuary-terminal.cmd
    sanctuary-terminal.sh
)

name="sanctuary-fixes-$version"
zip="dist/$name.zip"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

mkdir -p "$stage/$name" dist
for file in "${files[@]}"; do
    mkdir -p "$stage/$name/$(dirname "$file")"
    cp "$file" "$stage/$name/$file"
done
# Players get the zip, not the repository.
sed -i '/docs\/contributing.md/d' "$stage/$name/README.md"

rm -f "$zip"
(cd "$stage" && zip -qr "$root/$zip" "$name")
echo "Wrote $zip"
unzip -l "$zip"
