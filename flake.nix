{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";
    naersk.url = "github:nix-community/naersk";

    rust-overlay.url = "github:oxalica/rust-overlay";
    # crane.url = "github:ipetkov/crane";
    # crane.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;

      perSystem = { config, self', pkgs, lib, system, ... }:
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
          ] ++ lib.optionals pkgs.stdenv.isLinux [
            pkgs.glib
            pkgs.gtk3
            pkgs.libsoup_3
            pkgs.webkitgtk_4_1
            pkgs.xdotool
          ] ++ lib.optionals pkgs.stdenv.isDarwin (with pkgs.darwin.apple_sdk.frameworks; [
            IOKit
            Carbon
            WebKit
            Security
            Cocoa
          ]);

          # This is useful when building crates as packages
          # Note that it does require a `Cargo.lock` which this repo does not have
          # craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;
        in rec
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              inputs.rust-overlay.overlays.default
            ];
          };

          packages.dx = let
            naersk' = pkgs.callPackage inputs.naersk {};
          in naersk'.buildPackage {
            name = "dioxus-cli";
            src = ./.;
            cargoBuildOptions = prev: prev ++ [ "-p" "dioxus-cli" ];
          };

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

          devShells.web = pkgs.mkShell {
            name = "dioxus-web-devShell";
            buildInputs = with pkgs; [
              openssl
              pkg-config
              cacert

              (rust-bin.stable.latest.default.override {
                extensions = [ "rust-src" "rust-analyzer" "rust-std" ];
                targets = [ "wasm32-unknown-unknown" ];
              })
              (pkgs.writeShellApplication {
                name = "rustup";
                text = ''
                    #!/bin/sh
                    echo "installed targets for active toolchain"
                    echo "--------------------------------------"
                    echo "wasm32-unknown-unknown"
                    echo "x86_64-unknown-linux-gnu"
                '';
              }) # mock rustup to pass dx toolchain verification
              packages.dx
            ];
            shellHook = ''

            '';
          };

        };
    };
}
