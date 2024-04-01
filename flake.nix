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

          # this fails with git cargo dependencies, if needed switch to cargoHash
          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          preBuild = ''
          export RUST_BACKTRACE=1
          '';

          # Make ncurses availble in PATH so that the tput command can be run, see https://github.com/mgunyho/tere/pull/100
          checkPhase = ''
          export PATH=${pkgs.ncurses}/bin:$PATH
          cargo test
          '';

          postPatch = ''
          rm .cargo/config.toml;
          '';

          buildInputs = [
            ncurses  # provides the tput command needed for integration tests
          ];

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
