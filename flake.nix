{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        tere = with pkgs; rustPlatform.buildRustPackage {
          pname = "tere";
          version = "1.5.1";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

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
        defaultPackage = tere;
        devShell = with pkgs;
          mkShell {
            buildInputs = [
              # rust packages
              cargo
              rustc
              rustfmt
              pre-commit
              rustPackages.clippy

              # Nix conveniences
              nil
              nixpkgs-fmt
            ];
            RUST_SRC_PATH = rustPlatform.rustLibSrc;
          };
      });
}
