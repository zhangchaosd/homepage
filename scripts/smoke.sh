#!/bin/sh
# Smoke-tests a release archive: version banner, check-config, serve, health and
# login page. Runs anywhere the binary runs; everything happens in a temp dir.
# Usage: smoke.sh <homepage-<platform>.tar.gz> <expected version>
set -eu
archive=$1
expected=$2
temp=$(mktemp -d)
pid=
trap '[ -n "$pid" ] && kill "$pid" 2>/dev/null; rm -rf "$temp"' EXIT
tar -xzf "$archive" -C "$temp" --strip-components=1
cd "$temp"
banner=$(./homepage --version)
echo "$banner"
[ "$banner" = "homepage $expected" ] || { echo "unexpected version banner (want $expected)"; exit 1; }
port=$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1])')
mkdir config
cat > config/config.toml <<EOF
[server]
listen = "127.0.0.1:$port"
public_url = "http://localhost:$port"
[auth]
username = "smoke"
[[services]]
name = "Example"
url = "https://example.com"
check = ""
EOF
echo "smoke-test-password-123" | ./homepage hash-password > config/secrets.toml
chmod 600 config/secrets.toml
./homepage -c config check-config
./homepage -c config > log 2>&1 &
pid=$!
for _ in $(seq 1 100); do
  if curl -fsS "http://127.0.0.1:$port/healthz" >/dev/null 2>&1; then
    curl -fsS "http://127.0.0.1:$port/" | grep -q '<div id="app">'
    [ "$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$port/api/v1/dashboard")" = 401 ]
    echo "smoke ok"
    exit 0
  fi
  sleep 0.1
done
cat log
exit 1
