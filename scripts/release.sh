#!/usr/bin/env bash
# Tags the current commit as v<VERSION> and pushes the tag. GitHub Actions
# (.github/workflows/release.yml) then builds the zip and publishes the release.
#
# Usage: scripts/release.sh
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

version="$(tr -d '[:space:]' < VERSION)"
cargo_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
tag="v$version"

if [ "$version" != "$cargo_version" ]; then
    echo "VERSION ($version) and Cargo.toml ($cargo_version) disagree" >&2
    exit 1
fi
if [ -n "$(git status --porcelain)" ]; then
    echo "Working tree is not clean; commit first" >&2
    exit 1
fi
if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
    echo "Tag $tag already exists; bump VERSION and Cargo.toml first" >&2
    exit 1
fi
if [ "$(git rev-parse HEAD)" != "$(git rev-parse '@{upstream}')" ]; then
    echo "HEAD is not pushed; run git push first" >&2
    exit 1
fi

git tag -a "$tag" -m "$tag"
git push origin "$tag"
echo "Pushed $tag. Watch the release build with: gh run watch"
