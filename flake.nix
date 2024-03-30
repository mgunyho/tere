{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    crate2nix.url = "github:nix-community/crate2nix?tag=crate2nix-v0.12.0";
  };

  outputs = inputs@{ self, nixpkgs, flake-parts, crate2nix, ... }:
  flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem = { system, pkgs, lib, inputs', ... }:
      let
        mkTere = {pkgs ? pkgs}:
          ((inputs.crate2nix.tools.${system}.appliedCargoNix {
            name = "tere";
            src = ./.;
          }).rootCrate.build.override {
              runTests = true;
            } // {
            meta = with lib; {
              description = "A faster alternative to cd + ls";
              homepage = "https://github.com/mgunyho/tere";
              license = licenses.eupl12;
              maintainers = with maintainers; [ ProducerMatt ];
              mainProgram = "tere";
            };});

          ## run the tests via the script command so that the integration tests have a TTY
          #checkPhase = ''
          #script -c 'cargo test'
          #'';

          #buildInputs = [
          #  util-linux  # 'script' command
          #];
      in {
        checks.default = mkTere {};
        packages.default = mkTere {};
        checks.musl = mkTere {pkgs = pkgs.pkgsMusl;};
        packages.musl = mkTere {pkgs = pkgs.pkgsMusl;};
        devShells.default = with pkgs;
          mkShell {
            buildInputs = [
              # rust packages
              cargo
              rustc
              rustfmt
              rust-analyzer
              pre-commit
              rustPackages.clippy

              # Nix conveniences
              nixUnstable
              nil
              alejandra
              statix
              deadnix
            ];
            RUST_SRC_PATH = rustPlatform.rustLibSrc;
          };
      };};
}
