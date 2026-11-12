#!/usr/bin/env bash
# Builds Vodka Launcher and installs it for the current user:
# the program in ~/.local/bin, plus an icon and an app-menu entry.
set -euo pipefail
cd "$(dirname "$0")/.."

pnpm install --frozen-lockfile
pnpm tauri build --no-bundle

bin="$HOME/.local/bin"
apps="$HOME/.local/share/applications"
icons="$HOME/.local/share/icons/hicolor"
mkdir -p "$bin" "$apps" "$icons/scalable/apps" "$icons/128x128/apps"

install -m 755 src-tauri/target/release/vodka-launcher "$bin/vodka-launcher"
install -m 644 static/favicon.svg "$icons/scalable/apps/vodka-launcher.svg"
install -m 644 src-tauri/icons/128x128.png "$icons/128x128/apps/vodka-launcher.png"

cat > "$apps/vodka-launcher.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Vodka Launcher
GenericName=Roblox Launcher
Comment=A Roblox launcher for Linux, built on Mocktail
Exec=$bin/vodka-launcher
Icon=vodka-launcher
StartupWMClass=vodka-launcher
Categories=Game;
Terminal=false
DESKTOP

update-desktop-database "$apps" 2>/dev/null || true
gtk-update-icon-cache -q "$icons" 2>/dev/null || true
echo "Installed. Open \"Vodka Launcher\" from your app menu, or run: vodka-launcher"
