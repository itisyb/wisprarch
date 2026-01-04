#!/usr/bin/env bash
set -e

echo "==> Stopping wisprarch..."
systemctl --user stop wisprarch 2>/dev/null || true
pkill -f "target/release/wisprarch" 2>/dev/null || true

echo "==> Stashing local changes..."
git stash --include-untracked 2>/dev/null || true

echo "==> Pulling latest..."
git pull --rebase

echo "==> Restoring stashed changes..."
git stash pop 2>/dev/null || true

echo "==> Building release..."
cargo build --release

echo "==> Installing..."
sudo cp target/release/wisprarch /usr/local/bin/

echo "==> Restarting service..."
systemctl --user restart wisprarch

echo "==> Done!"
wisprarch --version 2>/dev/null || echo "(version check skipped)"
