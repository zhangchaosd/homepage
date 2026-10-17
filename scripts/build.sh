#!/bin/sh
# Builds the frontend and embeds it into a single release binary at target/release/homepage.
# Cross-compile with: TARGET=x86_64-unknown-linux-gnu scripts/build.sh
set -eu
cd "$(dirname "$0")/.."
npm --prefix web ci
npm --prefix web run build
if [ -n "${TARGET:-}" ]; then
  cargo build --release --locked --target "$TARGET"
  echo "built target/$TARGET/release/homepage"
else
  cargo build --release --locked
  echo "built target/release/homepage"
fi
