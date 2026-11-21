#!/usr/bin/env bash
# Builds the release AppImage inside Ubuntu 24.04 (glibc 2.39), so it also runs on older distros and on NixOS
# (through appimage-run or the flake). Needs Docker. Writes dist/vodka-launcher-<version>-x86_64.AppImage.
set -euo pipefail
cd "$(dirname "$0")/.."

image=vodka-appimage-builder
docker build -t "$image" scripts/appimage

# Build from a clean copy of the tracked files, so this machine's node_modules and target/ don't leak in.
# Caches for cargo, pnpm and target/ live in Docker volumes and make later builds faster.
docker run --rm \
  -v "$PWD:/repo:ro" \
  -v "$PWD/dist:/out" \
  -v vodka-cargo-registry:/opt/cargo/registry \
  -v vodka-pnpm-store:/root/.local/share/pnpm \
  -v vodka-target:/src/src-tauri/target \
  -v vodka-tauri-cache:/root/.cache/tauri \
  -e HOST_UID="$(id -u)" -e HOST_GID="$(id -g)" \
  "$image" bash -euo pipefail -c '
    cd /repo && git config --global --add safe.directory /repo
    git ls-files -co --exclude-standard -z | xargs -0 cp --parents -t /src 2>/dev/null || true
    cd /src
    pnpm install --frozen-lockfile
    ./scripts/build-appimage.sh
    cp dist/*.AppImage dist/*.AppImage.sha256 /out/
    chown "$HOST_UID:$HOST_GID" /out/*
  '
version=$(sed -n 's/.*"version": "\(.*\)".*/\1/p' src-tauri/tauri.conf.json | head -n 1)
echo "Built dist/vodka-launcher-$version-x86_64.AppImage (needs glibc 2.39 or newer)"
