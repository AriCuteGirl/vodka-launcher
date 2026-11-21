# Vodka Launcher from the prebuilt release AppImage. This is the flake's default package: nothing is compiled, so
# it installs quickly and works on any x86_64 CPU (building from source needs tools that require AVX2).
# For a new release, update `version` and `hash` (nix store prefetch-file <url> prints the hash).
{
  lib,
  appimageTools,
  fetchurl,
}:
let
  pname = "vodka-launcher";
  version = "0.5.2";
  src = fetchurl {
    url = "https://github.com/AriCuteGirl/vodka-launcher/releases/download/v${version}/vodka-launcher-${version}-x86_64.AppImage";
    hash = "sha256-6HJMN/dZYN3+aLSGSaZ5jD6s7qnI9x2ECjnOzQa2AnE=";
  };
  contents = appimageTools.extract { inherit pname version src; };
in
appimageTools.wrapType2 {
  inherit pname version src;

  # Vodka starts Mocktail through `flatpak`. The wrapper has its own /usr, so bring a flatpak into it; it uses the
  # system's Flatpak installation (/var/lib/flatpak) and your own (~/.local/share/flatpak) as usual.
  extraPkgs = pkgs: [ pkgs.flatpak ];

  extraInstallCommands = ''
    install -Dm644 "${contents}/vodka-launcher.desktop" "$out/share/applications/vodka-launcher.desktop" 2>/dev/null \
      || install -Dm644 "${contents}"/*.desktop "$out/share/applications/vodka-launcher.desktop"
    substituteInPlace "$out/share/applications/vodka-launcher.desktop" --replace-quiet 'Exec=AppRun' 'Exec=vodka-launcher'
    sed -i 's|^Exec=[^ ]*|Exec=vodka-launcher|' "$out/share/applications/vodka-launcher.desktop"
    for size in 32x32 128x128 256x256@2; do
      icon="${contents}/usr/share/icons/hicolor/$size/apps/vodka-launcher.png"
      [ -f "$icon" ] && install -Dm644 "$icon" "$out/share/icons/hicolor/$size/apps/vodka-launcher.png"
    done
    true
  '';

  meta = {
    description = "A Roblox launcher for Linux, built on Mocktail";
    homepage = "https://github.com/AriCuteGirl/vodka-launcher";
    license = lib.licenses.asl20;
    mainProgram = "vodka-launcher";
    platforms = [ "x86_64-linux" ];
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
  };
}
