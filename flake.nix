{
  description = "Build a cargo project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { nixpkgs, crane, fenix, flake-utils, advisory-db, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        inherit (pkgs) lib;

        rustWithWasiTarget = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "x86_64-unknown-linux-musl" ];
        };

        # NB: we don't need to overlay our custom toolchain for the *entire*
        # pkgs (which would require rebuidling anything else which uses rust).
        # Instead, we just want to update the scope that crane will use by appending
        # our specific toolchain there.
        craneLibMusl = (crane.mkLib pkgs).overrideToolchain rustWithWasiTarget;

        craneLib = crane.lib.${system};
        src = (craneLib.path ./.);

        # Common arguments can be set here to avoid repeating them later
        commonArgs = (buildMusl: {
          inherit src;
          strictDeps = true;

          buildInputs = [
            # Add additional build inputs here
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];

          preBuild = ''
          export RUST_BACKTRACE=1
          '';

          checkPhase = ''
            ${pkgs.unixtools.script}/bin/script -qfec 'cargo test' /dev/null
          '';

          doCheck = false;

          meta = with lib; {
            description = "A faster alternative to cd + ls";
            homepage = "https://github.com/mgunyho/tere";
            license = licenses.eupl12;
            maintainers = with maintainers; [ ProducerMatt ];
            mainProgram = "tere";
          };
        } // (lib.optionalAttrs buildMusl {
          target = "x86_64-unknown-linux-musl";
          cargoExtraArgs = "--target x86_64-unknown-linux-musl";
          #CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "${pkgs.llvmPackages.lld}/bin/lld";
        }));

        craneLibLLvmTools = craneLib.overrideToolchain
          (fenix.packages.${system}.complete.withComponents [
            "cargo"
            "llvm-tools"
            "rustc"
          ]);

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = buildMusl: (if buildMusl then craneLibMusl else craneLib).buildDepsOnly (commonArgs buildMusl);

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        my-crate = buildMusl: (if buildMusl then craneLibMusl else craneLib).buildPackage (commonArgs buildMusl // {
          cargoArtifacts = cargoArtifacts buildMusl;
        });
      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          my-crate = my-crate false;
          my-crate-musl = my-crate true;

          # Run clippy (and deny all warnings) on the crate source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          my-crate-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          my-crate-doc = craneLib.cargoDoc (commonArgs // {
            inherit cargoArtifacts;
          });

          # Check formatting
          my-crate-fmt = craneLib.cargoFmt {
            inherit src;
          };

          # Audit dependencies
          my-crate-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          # Audit licenses
          my-crate-deny = craneLib.cargoDeny {
            inherit src;
          };
        };
        packages = {
          default = my-crate false;
          musl = my-crate true;
        } // lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
          my-crate-llvm-coverage = craneLibLLvmTools.cargoLlvmCov (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        apps.default = flake-utils.lib.mkApp {
          drv = my-crate false;
        };

        apps.musl = flake-utils.lib.mkApp {
          drv = my-crate true;
        };

        devShells.default = with pkgs; craneLib.devShell {
          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = [
              # rust packages
              rustfmt
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
      });
}
