{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";

    rust-overlay.url = "github:oxalica/rust-overlay";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;

      perSystem =
        {
          self',
          system,
          ...
        }:
        let
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              inputs.rust-overlay.overlays.default
            ];
          };
          lib = pkgs.lib;
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
              "clippy"
            ];
          };
          craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;
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
          fullSrc = pkgs.lib.cleanSource ./.;
          cargoSrc = craneLib.cleanCargoSource fullSrc;

          commonArgs = {
            src = fullSrc;
            strictDeps = true;
            buildInputs = rustBuildInputs;
            nativeBuildInputs = [
              pkgs.pkg-config
            ];
          };

          cargoArtifacts = craneLib.buildDepsOnly (
            commonArgs
            // {
              src = cargoSrc;
              pname = "dioxus-deps";
              version = cargoToml.package.version;
            }
          );

          rustPackage =
            package:
            {
              binary ? package,
              features ? [ ],
            }:
            craneLib.buildPackage (
              commonArgs
              // {
                pname = package;
                version = cargoToml.package.version;
                inherit cargoArtifacts;
                cargoExtraArgs = "--locked --package ${package} ${
                  lib.concatStringsSep " " (map (f: "--features ${f}") features)
                }";
                doCheck = false; # Disable tests to avoid building deps for them
                installPhaseCommand = ''
                  mkdir -p $out/bin
                  cp target/release/${binary} $out/bin/
                '';
              }
            );
        in
        {
          packages.dioxus-cli = (
            rustPackage "dioxus-cli" {
              binary = "dx";
              features = [ "no-downloads" ];
            }
          );
          packages.default = self'.packages.dioxus-cli;
          checks.dioxus-cli = self'.packages.dioxus-cli;

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
