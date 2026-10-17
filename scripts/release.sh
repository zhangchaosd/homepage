#!/usr/bin/env bash
# Local CI and release; replaces the GitHub Actions workflows.
#
#   scripts/release.sh             checks, builds all platforms, smoke tests, packages
#   scripts/release.sh --publish   ...then tags v<version>, pushes and creates the GitHub Release
#
# Environment:
#   ZIG_HOME    directory with the zig 0.14.1 binary (default ~/.local/zig-0.14.1)
#   SMOKE_HOST  user@host of an x86_64 Linux machine for the Linux smoke test (optional)
#
# Requires an Apple Silicon Mac with Rust, Node, cargo-zigbuild, Zig 0.14.1 and gh.
# macOS builds are native (arm64) and cross (x86_64); Linux builds use cargo-zigbuild
# against glibc 2.28. Asset names carry no version; the in-app updater relies on them.
set -euo pipefail
cd "$(dirname "$0")/.."

publish=0
for arg in "$@"; do
  case "$arg" in
    --publish) publish=1 ;;
    *) echo "usage: scripts/release.sh [--publish]" >&2; exit 2 ;;
  esac
done

step() { printf '\n==> %s\n' "$*"; }
die() { printf 'release: %s\n' "$*" >&2; exit 1; }

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
tag="v$version"
out="target/release-$version"
repo=zhangchaosd/homepage
# label:rust target pairs, matching update::platform()
platforms=(
  "linux-x86_64:x86_64-unknown-linux-gnu.2.28"
  "linux-aarch64:aarch64-unknown-linux-gnu.2.28"
  "macos-aarch64:aarch64-apple-darwin"
  "macos-x86_64:x86_64-apple-darwin"
)

step "Preconditions for $tag"
[ -z "$(git status --porcelain)" ] || die "working tree is not clean"
grep -q "\"version\": \"$version\"" web/package.json || die "web/package.json is not $version"
if [ "$publish" = 1 ]; then
  [ "$(git branch --show-current)" = main ] || die "publish from main"
  git fetch -q origin
  git merge-base --is-ancestor origin/main HEAD || die "main is behind origin/main; pull first"
  ! git rev-parse -q --verify "refs/tags/$tag" >/dev/null || die "tag $tag already exists"
  ! git ls-remote --exit-code --tags origin "$tag" >/dev/null 2>&1 || die "tag $tag already exists on origin"
fi
export PATH="${ZIG_HOME:-$HOME/.local/zig-0.14.1}:$PATH"
[ "$(zig version 2>/dev/null)" = 0.14.1 ] || die "zig 0.14.1 not found (set ZIG_HOME)"
command -v cargo-zigbuild >/dev/null || die "cargo-zigbuild not installed"
rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu aarch64-apple-darwin x86_64-apple-darwin >/dev/null

step "Checks (what CI ran)"
npm ci --prefix web
npm run check --prefix web
npm run build --prefix web
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked

step "Build"
for entry in "${platforms[@]}"; do
  target=${entry#*:}
  case "$target" in
    *linux*) cargo zigbuild --release --locked --target "$target" ;;
    *) cargo build --release --locked --target "$target" ;;
  esac
done

step "Package $out"
rm -rf "$out"
mkdir -p "$out"
assets=()
for entry in "${platforms[@]}"; do
  label=${entry%%:*}
  target=${entry#*:}
  scripts/package.sh "target/${target%.2.28}/release/homepage" "$label" "$out" >/dev/null
  assets+=("homepage-$label.tar.gz")
done
(cd "$out" && shasum -a 256 "${assets[@]}" > SHA256SUMS.txt && shasum -a 256 -c SHA256SUMS.txt >/dev/null)
ls -l "$out"

step "Smoke tests"
scripts/smoke.sh "$out/homepage-macos-aarch64.tar.gz" "$version"
for label in linux-x86_64 linux-aarch64; do
  binary=$(tar -tzf "$out/homepage-$label.tar.gz" | grep '/homepage$')
  glibc=$(tar -xzOf "$out/homepage-$label.tar.gz" "$binary" | strings | grep -oE '^GLIBC_2\.[0-9]+' | sort -t. -k2 -n -u | tail -1)
  [ "$(printf '%s\n' "$glibc" GLIBC_2.28 | sort -V | tail -1)" = GLIBC_2.28 ] ||
    die "$label needs $glibc, expected at most GLIBC_2.28"
  echo "$label: needs at most $glibc"
done
if [ -n "${SMOKE_HOST:-}" ]; then
  remote=$(ssh "$SMOKE_HOST" 'mktemp -d')
  scp -q "$out/homepage-linux-x86_64.tar.gz" scripts/smoke.sh "$SMOKE_HOST:$remote/"
  ssh "$SMOKE_HOST" "sh $remote/smoke.sh $remote/homepage-linux-x86_64.tar.gz $version; status=\$?; rm -rf $remote; exit \$status"
else
  echo "SMOKE_HOST not set: Linux binaries were not run (glibc checked above)"
fi

if [ "$publish" = 0 ]; then
  printf '\nBuilt %s in %s. Run with --publish to tag and release.\n' "$tag" "$out"
  exit 0
fi

step "Publish $tag"
prerelease=()
[[ "$version" == *-* ]] && prerelease=(--prerelease)
git tag -a "$tag" -m "Homepage $tag"
git push origin main "$tag"
# ${arr[@]+...} keeps an empty array working under set -u in macOS bash 3.2.
gh release create "$tag" --repo "$repo" --verify-tag --generate-notes ${prerelease[@]+"${prerelease[@]}"} --title "Homepage $tag" \
  "${assets[@]/#/$out/}" "$out/SHA256SUMS.txt"
check=$(mktemp -d)
gh release download "$tag" --repo "$repo" --dir "$check"
(cd "$check" && shasum -a 256 -c SHA256SUMS.txt)
rm -rf "$check"
printf '\nPublished https://github.com/%s/releases/tag/%s\n' "$repo" "$tag"
