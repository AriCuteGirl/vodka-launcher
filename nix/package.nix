# Builds Vodka Launcher from this checkout. Used by ../flake.nix.
# After pnpm-lock.yaml changes, set pnpmDeps.hash to "" once, build, and paste the hash Nix prints.
{
  lib,
  rustPlatform,
  cargo-tauri,
  pnpm_12,
  fetchPnpmDeps,
  pnpmConfigHook,
  nodejs,
  pkg-config,
  wrapGAppsHook3,
  glib-networking,
  openssl,
  aws-lc,
  dbus,
  webkitgtk_4_1,
}:
let
  pnpm = pnpm_12;
  fs = lib.fileset;
  root = ../.;
  # Only the files the build reads, so editing docs doesn't rebuild everything.
  source = fs.toSource {
    inherit root;
    fileset = fs.difference (fs.unions [
      (root + "/package.json")
      (root + "/pnpm-lock.yaml")
      (root + "/pnpm-workspace.yaml")
      (root + "/tsconfig.json")
      (root + "/vite.config.js")
      (root + "/src")
      (root + "/static")
      (root + "/src-tauri")
    ]) (fs.maybeMissing (root + "/src-tauri/target"));
  };
in
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "vodka-launcher";
  version = (lib.importJSON (root + "/package.json")).version;
  src = source;

  cargoRoot = "src-tauri";
  buildAndTestSubdir = finalAttrs.cargoRoot;
  cargoLock.lockFile = root + "/src-tauri/Cargo.lock";

  pnpmDeps = fetchPnpmDeps {
    inherit (finalAttrs) pname version src;
    inherit pnpm;
    fetcherVersion = 5;
    hash = "sha256-6bw+h7yT76KO4/SyDODqGdvFUVwewzVVugi5W6H0ASo=";
  };

  nativeBuildInputs = [
    cargo-tauri.hook
    pnpmConfigHook
    pnpm
    nodejs
    pkg-config
    wrapGAppsHook3
  ];

  buildInputs = [
    glib-networking
    openssl
    aws-lc
    dbus
    webkitgtk_4_1
  ];

  # rustls uses AWS-LC; take nixpkgs' build instead of compiling it again.
  env.AWS_LC_SYS_USE_SYSTEM = true;

  # The tests need a keyring and network access.
  doCheck = false;

  meta = {
    description = "A Roblox launcher for Linux, built on Mocktail";
    homepage = "https://github.com/AriCuteGirl/vodka-launcher";
    license = lib.licenses.asl20;
    mainProgram = "vodka-launcher";
    platforms = lib.platforms.linux;
  };
})
