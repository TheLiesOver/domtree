#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
    echo "Cargo is not installed."
    echo "Install it with:"
    echo "  sudo apt update && sudo apt install cargo"
    exit 1
fi

cargo build --release
sudo install -m 755 target/release/domtree /usr/local/bin/domtree

echo
echo "Installed /usr/local/bin/domtree"
domtree --version
