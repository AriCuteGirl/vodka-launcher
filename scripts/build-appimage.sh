#!/usr/bin/env bash
# Builds the Linux AppImage and copies it to dist/ with a checksum.
#
# Tauri prepares the AppDir and fetches linuxdeploy; we then run linuxdeploy ourselves with a patched GTK plugin.
# The stock plugin searches all of /usr/lib, so copies of GTK libraries other programs keep in their own folders
# there (VMware's, for one) get bundled and break the build. Tauri rewrites its copy of the plugin on every build,
# so the patch is applied to a private copy.
set -euo pipefail
cd "$(dirname "$0")/.."

version=$(sed -n 's/.*"version": "\(.*\)".*/\1/p' src-tauri/tauri.conf.json | head -n 1)
bundle="src-tauri/target/release/bundle/appimage"
appdir="$bundle/Vodka Launcher.AppDir"
out="dist/vodka-launcher-$version-x86_64.AppImage"
cache="$HOME/.cache/tauri"
tools="src-tauri/target/appimage-tools"

# Tauri's own linuxdeploy step may fail on such machines; all we need from it is the AppDir and the tools.
APPIMAGE_EXTRACT_AND_RUN=1 pnpm tauri build --bundles appimage || true
[ -x "$appdir/usr/bin/vodka-launcher" ] || { echo "Tauri did not produce $appdir" >&2; exit 1; }

rm -rf "$tools" && mkdir -p "$tools"
ld=$(ls "$cache"/linuxdeploy-*-x86_64.AppImage | head -n 1)
cp "$ld" "$tools/linuxdeploy-x86_64.AppImage"
cp "$cache/linuxdeploy-plugin-appimage.AppImage" "$tools/linuxdeploy-plugin-appimage.AppImage"
cp "$cache/linuxdeploy-plugin-gtk.sh" "$tools/linuxdeploy-plugin-gtk.sh"
sed -i 's|find "$directory" \\( -type l -o -type f \\) -name "$library" -print0|find "$directory" -path "*/vmware" -prune -o \\( -type l -o -type f \\) -name "$library" -print0|' \
  "$tools/linuxdeploy-plugin-gtk.sh"
grep -q vmware "$tools/linuxdeploy-plugin-gtk.sh" || { echo "Couldn't patch the GTK plugin (did it change upstream?)" >&2; exit 1; }
chmod +x "$tools"/*

rm -f "$bundle"/*.AppImage
(
  cd "$bundle"
  APPIMAGE_EXTRACT_AND_RUN=1 NO_STRIP=1 DEPLOY_GTK_VERSION=3 OUTPUT="Vodka Launcher_${version}_amd64.AppImage" \
    "$OLDPWD/$tools/linuxdeploy-x86_64.AppImage" --appdir "Vodka Launcher.AppDir" --plugin gtk --output appimage
)

mkdir -p dist
cp "$bundle/Vodka Launcher_${version}_amd64.AppImage" "$out"
chmod +x "$out"
(cd dist && sha256sum "$(basename "$out")" > "$(basename "$out").sha256")
echo "Built $out"
