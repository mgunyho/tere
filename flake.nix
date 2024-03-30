{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    #crate2nix.url = "github:nix-community/crate2nix";
  };

  outputs = inputs@{ self, nixpkgs, flake-parts, ... }:
  flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-linux"
        "aarch64-darwin"
      ];

      perSystem = { system, pkgs, lib, inputs', ... }:
      let
        pkgs = import nixpkgs { inherit system; };
        tere = with pkgs; rustPlatform.buildRustPackage {
          pname = "tere";
          version = "1.5.1";

          src = ./.;

          # this fails with git cargo dependencies, if needed switch to cargoHash
          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          preBuild = ''
          export RUST_BACKTRACE=1
          '';

          postPatch = ''
          rm .cargo/config.toml;
          '';

          meta = with lib; {
            description = "A faster alternative to cd + ls";
            homepage = "https://github.com/mgunyho/tere";
            license = licenses.eupl12;
            maintainers = with maintainers; [ ProducerMatt ];
            mainProgram = "tere";
          };
        };

      in {
        packages.default = tere;
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
              nil
              nixpkgs-fmt
            ];
            RUST_SRC_PATH = rustPlatform.rustLibSrc;
          };
      };};
}
