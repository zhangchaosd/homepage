#!/usr/bin/env bash
# Runs the README's Linux install block (or, with "uninstall", the block after
# it) verbatim, except for the steps that need a person: the release download
# installs the freshly built package instead, the password is piped in, and
# the editor step adds placeholder tokens.
# Usage: readme-install.sh install <package.tar.gz>
#        readme-install.sh uninstall
set -euo pipefail
mode=$1
heading='### Linux（systemd 用户服务）'
block=1
[[ $mode == uninstall ]] && block=2

script=$(mktemp)
awk -v h="$heading" -v want="$block" 'index($0, h) == 1 { section = 1 }
     section && /^```sh$/ { n++; if (n == want) { inblock = 1; next } }
     inblock && /^```$/ { exit }
     inblock' README.md > "$script"

if [[ $mode == install ]]; then
  package=$(realpath "$2")
  sed -i -E \
    -e "s#^curl -fsSL https://github\.com/.*[|] tar (.*)\$#tar \1 < '$package'#" \
    -e 's#\./homepage hash-password#echo "ci-test-password-123" | ./homepage hash-password#' \
    -e 's#^[$][{]EDITOR:-vi[}] .*$#printf "\\n[tokens]\\nvikunja = \\"x\\"\\nbookmarkd = \\"x\\"\\nhomeassistant = \\"x\\"\\n" >> secrets.toml#' \
    "$script"
  grep -q "^tar -xz --strip-components=1 < '$package'" "$script" || { echo "download line not found in README"; exit 1; }
  grep -q 'ci-test-password-123' "$script" || { echo "hash-password line not found in README"; exit 1; }
  grep -q '\[tokens\]' "$script" || { echo "editor line not found in README"; exit 1; }
fi

cat "$script"
bash -e "$script"
