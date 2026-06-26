{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    crane.url = "github:ipetkov/crane";
    nix-fast-build.url = "github:Mic92/nix-fast-build";
    nix-fast-build.inputs.nixpkgs.follows = "nixpkgs";
  };
  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      fenix,
      crane,
      nix-fast-build,
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

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Must match the wasm-bindgen crate version in Cargo.lock exactly.
        wasmBindgenCli = pkgs.rustPlatform.buildRustPackage rec {
          pname = "wasm-bindgen-cli";
          version = "0.2.126";
          src = pkgs.fetchCrate {
            inherit pname version;
            hash = "sha256-H6Is3fiZVxZCfOMWK5dWMSrtn50VGv0sfdnsT+cTtyk=";
          };
          cargoHash = "sha256-VucqkXbCi4qtQzY/HrXiDnbSURsagPsdNVMn1Tw3UiY=";
          doCheck = false;
        };

        # Cargo sources plus files cargo-leptos/the app read at build time.
        src =
          let
            leptosFilter = path: type:
              (builtins.match ".*/public(/.*)?" path != null) ||
              (builtins.match ".*/style(/.*)?" path != null) ||
              (builtins.match ".*/tailwind.config.js$" path != null) ||
              (builtins.match ".*/data.json$" path != null) ||
              (builtins.match ".*/app/src/.*\\.wgsl$" path != null);
            srcFilter = path: type:
              (leptosFilter path type) || (craneLib.filterCargoSources path type);
          in
          pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = srcFilter;
          };

        cargoVendorDir = craneLib.vendorCargoDeps { inherit src; };

        commonArgs = {
          inherit src cargoVendorDir;
          pname = "personal-site";
          version = "0.1.0";
          strictDeps = true;
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];
          RUSTFLAGS = "--remap-path-prefix=${cargoVendorDir}=/cargo-vendor --remap-path-prefix=${rustToolchain}=/rust-toolchain";
        };

        buildArgs = commonArgs // {
          COMMIT_SHA = self.shortRev or self.dirtyShortRev or "dirty";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        clippy = craneLib.cargoClippy (buildArgs // {
          inherit cargoArtifacts;
          cargoClippyExtraArgs = "--workspace -- -Dwarnings";
        });

        fmt = craneLib.cargoFmt commonArgs;

        nextestArchive = craneLib.mkCargoDerivation (buildArgs // {
          inherit cargoArtifacts;
          pname = "personal-site-nextest-archive";
          doInstallCargoArtifacts = false;
          nativeBuildInputs = commonArgs.nativeBuildInputs ++ [ pkgs.cargo-nextest ];
          buildPhaseCargoCommand = ''
            mkdir -p $out
            cargo nextest archive --workspace --archive-file $out/archive.tar.zst
          '';
          doCheck = false;
        });

        personalSite = craneLib.mkCargoDerivation (buildArgs // {
          inherit cargoArtifacts;
          doInstallCargoArtifacts = false;
          nativeBuildInputs = commonArgs.nativeBuildInputs ++ [
            pkgs.cargo-leptos
            pkgs.tailwindcss
            pkgs.dart-sass
            pkgs.binaryen
            wasmBindgenCli
          ];
          buildPhaseCargoCommand = ''
            cargo leptos build --release -vv
          '';
          installPhaseCommand = ''
            mkdir -p $out/bin $out/site

            for candidate in \
              target/server/release/personal_site \
              target/server/release/server \
              target/release/personal_site \
              target/release/server
            do
              if [ -x "$candidate" ]; then
                cp "$candidate" $out/bin/personal_site
                break
              fi
            done

            if [ ! -x $out/bin/personal_site ]; then
              echo "error: server binary not found" >&2
              find target -maxdepth 4 -type f -perm -0100 -print >&2 || true
              exit 1
            fi

            cp -r target/site/. $out/site/
            cp Cargo.toml $out/Cargo.toml
          '';
          doCheck = false;
        });

        runApp = pkgs.writeShellScript "run-personal-site" ''
          cd /app
          exec ${personalSite}/bin/personal_site
        '';

        personalSiteImg = pkgs.dockerTools.streamLayeredImage {
          name = "personal-site";
          tag = "latest";
          contents = [
            personalSite
            pkgs.bashInteractive
            pkgs.cacert
          ];
          fakeRootCommands = ''
            mkdir -p ./app
            ln -s ${personalSite}/site ./app/site
            ln -s ${personalSite}/Cargo.toml ./app/Cargo.toml
          '';
          config = {
            Cmd = [ "${runApp}" ];
            Env = [
              "RUST_LOG=info"
              "APP_ENVIRONMENT=production"
              "LEPTOS_OUTPUT_NAME=personal_site"
              "LEPTOS_SITE_ROOT=site"
              "LEPTOS_SITE_PKG_DIR=pkg"
              "LEPTOS_SITE_ADDR=0.0.0.0:8080"
              "LEPTOS_RELOAD_PORT=3001"
            ];
            ExposedPorts = { "8080/tcp" = { }; };
          };
        };
      in
      {
        packages = {
          inherit personalSite personalSiteImg cargoArtifacts nextestArchive clippy fmt;
          nix-fast-build = nix-fast-build.packages.${system}.default;
          default = personalSite;
        };

        checks = {
          inherit clippy fmt;
        };

        devShells.default = craneLib.devShell {
          packages = with pkgs; [
            flyctl
            just
            just-lsp
            cargo-leptos
            cargo-nextest
            cargo-watch
            cargo-info
            cargo-udeps
            cargo-deny
            nix-fast-build.packages.${system}.default
            tailwindcss
            dart-sass
            binaryen
            wasmBindgenCli
            nodejs
            pkg-config
            openssl
            taplo
          ];
        };
      }
    );
}
