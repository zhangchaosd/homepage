#!/bin/sh
# Creates dev/config/secrets.toml with the development password "homepage-dev-password".
set -eu
cd "$(dirname "$0")/.."
hash=$(echo "homepage-dev-password" | cargo run -q -- hash-password)
cat > dev/config/secrets.toml <<SECRETS
$hash

[tokens]
vikunja = "dev-vikunja-token"
bookmarkd = "dev-bookmarkd-token"
homeassistant = "dev-ha-token"
inbox = "dev-inbox-token"
SECRETS
chmod 600 dev/config/secrets.toml
echo "wrote dev/config/secrets.toml (user: chao, password: homepage-dev-password)"
