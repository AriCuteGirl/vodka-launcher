#!/usr/bin/env bash
# Cross-builds the Windows version of Vodka Launcher from Linux and zips it.
# Needs: rustup target x86_64-pc-windows-gnu and a mingw-w64 gcc on PATH.
set -euo pipefail
cd "$(dirname "$0")/.."

target=x86_64-pc-windows-gnu
command -v x86_64-w64-mingw32-gcc >/dev/null || { echo "mingw-w64 gcc (x86_64-w64-mingw32-gcc) is not on PATH." >&2; exit 1; }
rustup target list --installed | grep -qx "$target" || { echo "Run: rustup target add $target" >&2; exit 1; }

# Some mingw packages (Nix's, for one) keep the threading library in a separate folder the linker isn't told about.
flags=""
if ! echo 'int main(){return 0;}' | x86_64-w64-mingw32-gcc -x c - -o /dev/null -l:libpthread.a 2>/dev/null; then
  lib=$(find /nix/store /usr/x86_64-w64-mingw32 /usr/lib/gcc/x86_64-w64-mingw32 -maxdepth 4 -name libpthread.a -path '*mingw*' 2>/dev/null | head -n 1 || true)
  [ -n "$lib" ] || { echo "Couldn't find mingw's libpthread.a. Install mingw-w64 winpthreads." >&2; exit 1; }
  flags="-L native=$(dirname "$lib")"
fi

pnpm install --frozen-lockfile
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="$flags" pnpm tauri build --target "$target" --no-bundle

out=src-tauri/target/$target/release
dist=dist/vodka-launcher-windows
rm -rf "$dist" && mkdir -p "$dist"
cp "$out/vodka-launcher.exe" "$dist/Vodka Launcher.exe"
# Tauri's Windows builds made with mingw load the WebView2 loader from a DLL next to the program.
cp "$out/WebView2Loader.dll" "$dist/"
cp LICENSE NOTICE "$dist/"
# Plain-text copy of the Windows notes, readable with Notepad.
cp docs/WINDOWS.md "$dist/READ ME FIRST.txt"
(cd dist && rm -f vodka-launcher-windows.zip && zip -qr vodka-launcher-windows.zip vodka-launcher-windows)
echo "Built dist/vodka-launcher-windows.zip"
