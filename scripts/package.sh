#!/usr/bin/env bash
# Packages a built binary as <out>/homepage-<label>.tar.gz. Names carry no
# version, so releases/latest/download/homepage-<label>.tar.gz always works,
# and the in-app updater looks for exactly this name.
# Usage: package.sh <binary> <label> [out dir, default out]
set -euo pipefail
bin=$1
label=$2
out=${3:-out}
package="homepage-$label"
rm -rf "$out/$package"
mkdir -p "$out/$package"
install -m 755 "$bin" "$out/$package/homepage"
cp -R README.md LICENSE examples "$out/$package/"
tar -C "$out" -czf "$out/$package.tar.gz" "$package"
rm -r "$out/$package"
echo "$out/$package.tar.gz"
