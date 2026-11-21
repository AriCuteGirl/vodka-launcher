{
  description = "Vodka Launcher, a Roblox launcher for Linux built on Mocktail";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      # vodka-launcher: the prebuilt release AppImage, nothing to compile (x86_64 only).
      # vodka-launcher-source: built from this checkout; needs a CPU with AVX2 and takes a while.
      packages = forAll (
        pkgs:
        let
          source = pkgs.callPackage ./nix/package.nix { };
          vodka = if pkgs.stdenv.hostPlatform.isx86_64 then pkgs.callPackage ./nix/appimage.nix { } else source;
        in
        {
          vodka-launcher = vodka;
          vodka-launcher-source = source;
          default = vodka;
        }
      );

      apps = forAll (pkgs: rec {
        vodka-launcher = {
          type = "app";
          program = "${self.packages.${pkgs.stdenv.hostPlatform.system}.vodka-launcher}/bin/vodka-launcher";
        };
        default = vodka-launcher;
      });

      overlays.default = final: _prev: {
        vodka-launcher = self.packages.${final.stdenv.hostPlatform.system}.vodka-launcher;
      };

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.vodka-launcher-source ];
          packages = [ pkgs.cargo-tauri pkgs.rust-analyzer ];
        };
      });
    };
}
