{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
  };
  outputs =
    {
      nixpkgs,
      flake-utils,
      fenix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        rustToolchain =
          with fenix.packages.${system};
          combine [
            latest.rustc
            latest.cargo
            latest.rust-src
            latest.rust-analyzer
            latest.clippy
            latest.rustfmt
            targets.wasm32-unknown-unknown.latest.rust-std
          ];
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.cargo-leptos
            pkgs.tailwindcss
            pkgs.dart-sass
            pkgs.binaryen
            pkgs.nodejs
            pkgs.wasm-bindgen-cli
            pkgs.pkg-config
            pkgs.openssl
          ];
        };
      }
    );
}
