#!/usr/bin/env bash
# Tags the current commit as v<VERSION> and pushes the tag. GitHub Actions
# (.github/workflows/release.yml) then builds the zip and publishes the release.
#
# With --bump, the version is first raised in VERSION, Cargo.toml and Cargo.lock, and
# that change is committed and pushed before tagging.
#
# Usage: scripts/release.sh [--bump patch|minor|major|<version>]
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

bump=""
while [ $# -gt 0 ]; do
    case "$1" in
        --bump) bump="${2:?--bump needs patch, minor, major or a version}"; shift 2 ;;
        -h|--help) sed -n '2,8p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "Unknown option: $1" >&2; exit 2 ;;
    esac
done

version="$(tr -d '[:space:]' < VERSION)"
cargo_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
if [ "$version" != "$cargo_version" ]; then
    echo "VERSION ($version) and Cargo.toml ($cargo_version) disagree" >&2
    exit 1
fi
if [ -n "$(git status --porcelain)" ]; then
    echo "Working tree is not clean; commit first" >&2
    exit 1
fi

if [ -n "$bump" ]; then
    IFS=. read -r major minor patch <<< "$version"
    case "$bump" in
        major) version="$((major + 1)).0.0" ;;
        minor) version="$major.$((minor + 1)).0" ;;
        patch) version="$major.$minor.$((patch + 1))" ;;
        *)
            if ! [[ "$bump" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
                echo "--bump needs patch, minor, major or a version like 1.2.3, not: $bump" >&2
                exit 1
            fi
            version="$bump" ;;
    esac
fi
tag="v$version"

if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
    echo "Tag $tag already exists; use --bump" >&2
    exit 1
fi

if [ -n "$bump" ]; then
    echo "$version" > VERSION
    sed -i "0,/^version = \".*\"/s//version = \"$version\"/" Cargo.toml
    # Cargo.lock has the package's own version too; keep it in sync so the next
    # build does not dirty the tree.
    sed -i "/^name = \"sanctuary-pad\"$/{n;s/^version = \".*\"/version = \"$version\"/}" Cargo.lock
    git add VERSION Cargo.toml Cargo.lock
    git commit -q -m "release $tag"
    git push
elif [ "$(git rev-parse HEAD)" != "$(git rev-parse '@{upstream}')" ]; then
    echo "HEAD is not pushed; run git push first" >&2
    exit 1
fi

git tag -a "$tag" -m "$tag"
git push origin "$tag"
echo "Pushed $tag. Watch the release build with: gh run watch"
