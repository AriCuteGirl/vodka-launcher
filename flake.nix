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
      packages = forAll (pkgs: rec {
        vodka-launcher = pkgs.callPackage ./nix/package.nix { };
        default = vodka-launcher;
      });

      apps = forAll (pkgs: rec {
        vodka-launcher = {
          type = "app";
          program = "${self.packages.${pkgs.stdenv.hostPlatform.system}.vodka-launcher}/bin/vodka-launcher";
        };
        default = vodka-launcher;
      });

      overlays.default = final: _prev: {
        vodka-launcher = final.callPackage ./nix/package.nix { };
      };

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.vodka-launcher ];
          packages = [ pkgs.cargo-tauri pkgs.rust-analyzer ];
        };
      });
    };
}
