{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";

    rust-overlay.url = "github:oxalica/rust-overlay";
    # crane.url = "github:ipetkov/crane";
    # crane.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;

      perSystem =
        {
          config,
          self',
          pkgs,
          lib,
          system,
          ...
        }:
        let
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
              "clippy"
            ];
          };
          rustBuildInputs = [
            pkgs.openssl
            pkgs.libiconv
            pkgs.pkg-config
          ]
          ++ lib.optionals pkgs.stdenv.isLinux [
            pkgs.glib
            pkgs.gtk3
            pkgs.libsoup_3
            pkgs.webkitgtk_4_1
            pkgs.xdotool
          ]
          ++ lib.optionals pkgs.stdenv.isDarwin [
            pkgs.apple-sdk
            pkgs.libiconv
          ];

          cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);

          rustPackage =
            package:
            {
              binary ? package,
              features ? [ ],
            }:
            (pkgs.makeRustPlatform {
              cargo = rustToolchain;
              rustc = rustToolchain;
            }).buildRustPackage
              {
                pname = package;
                version = cargoToml.package.version;
                src = pkgs.lib.cleanSource ./.;
                cargoLock.lockFile = ./Cargo.lock;
                buildInputs = rustBuildInputs;
                nativeBuildInputs = [
                  rustToolchain
                  pkgs.pkg-config
                ];
                buildPhase = ''
                  mkdir -p .cargo
                  cp ${./Cargo.lock} Cargo.lock
                  cargo build --release --package ${package} ${
                    lib.concatStringsSep " " (map (f: "--features ${f}") features)
                  }
                '';
                installPhase = ''
                  mkdir -p $out/bin
                  ls -alR target/release
                  cp target/release/${binary} $out/bin/
                '';
                doCheck = false; # Disable tests to avoid building deps for them
              };

          # This is useful when building crates as packages
          # Note that it does require a `Cargo.lock` which this repo does not have
          # craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;
        in
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              inputs.rust-overlay.overlays.default
            ];
          };

          packages.dioxus-cli = (
            rustPackage "dioxus-cli" {
              binary = "dx";
              features = [ "no-downloads" ];
            }
          );

          devShells.default = pkgs.mkShell {
            name = "dioxus-dev";
            buildInputs = rustBuildInputs;
            nativeBuildInputs = [
              # Add shell dependencies here
              rustToolchain
            ];
            shellHook = ''
              # For rust-analyzer 'hover' tooltips to work.
              export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library";
            '';
          };
        };
    };
}
