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
    hash = "sha256-WLUyU7H+x7u5M33Kxs+V0NtTWDUcTcNLgZgNW4Pr0UM=";
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

  env = {
    # rustls uses AWS-LC; take nixpkgs' build instead of compiling it again.
    AWS_LC_SYS_USE_SYSTEM = true;
    # Rolldown's native build needs AVX2 and dies with "illegal instruction" (exit 132) on older CPUs. Its
    # WebAssembly build is slower but runs anywhere.
    NAPI_RS_FORCE_WASI = "true";
  };

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
