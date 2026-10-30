{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    crane.url = "github:ipetkov/crane";
    nix-fast-build.url = "github:Mic92/nix-fast-build";
    nix-fast-build.inputs.nixpkgs.follows = "nixpkgs";
    # seanaye/feat/generic-pages; match Cargo.toml and app/Cargo.toml.
    topcoat = {
      url = "github:seanaye/topcoat/9a039d010e9fd75ba4f33ef2ed60ea8c04b79a77";
      flake = false;
    };
  };
  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      fenix,
      crane,
      nix-fast-build,
      topcoat,
    }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-darwin" ] (
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
        # Use upstream prebuilt binaries to avoid rebuilding/vendoring the CLI,
        # which can fail when crates.io blocks bulk API downloads with 403s.
        wasmBindgenCli =
          let
            version = "0.2.130";
            targets = {
              x86_64-linux = {
                target = "x86_64-unknown-linux-musl";
                hash = "sha256-lofzu1Ubn2bzftMqQNeROgD3li22UwVOVoV7wwbDIn8=";
              };
              aarch64-darwin = {
                target = "aarch64-apple-darwin";
                hash = "sha256-uQ3bk1f1hLGviX1YrXfkzOK0j8F+KpZ7wkmof3KAlEQ=";
              };
            };
            targetInfo = targets.${system};
          in
          pkgs.stdenvNoCC.mkDerivation {
            pname = "wasm-bindgen-cli";
            inherit version;
            src = pkgs.fetchurl {
              url = "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/${version}/wasm-bindgen-${version}-${targetInfo.target}.tar.gz";
              inherit (targetInfo) hash;
            };
            installPhase = ''
              runHook preInstall
              install -Dm755 wasm-bindgen $out/bin/wasm-bindgen
              install -Dm755 wasm-bindgen-test-runner $out/bin/wasm-bindgen-test-runner
              install -Dm755 wasm2es6js $out/bin/wasm2es6js
              runHook postInstall
            '';
          };

        # Build the CLI from exactly the same fork revision as the framework.
        topcoatCli = craneLib.buildPackage {
          src = topcoat;
          cargoVendorDir = craneLib.vendorCargoDeps { src = topcoat; };
          pname = "topcoat-cli";
          version = "0.11.0";
          cargoExtraArgs = "-p topcoat-cli --bin topcoat --bin cargo-topcoat";
          strictDeps = true;
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];
          doCheck = false;
        };

        # Use nixpkgs' Node-free Tailwind 4 package for hermetic builds.
        tailwindCli = pkgs.tailwindcss_4;

        # Cargo sources plus the static assets, shaders, and content read at build time.
        src =
          let
            siteFilter =
              path: type:
              (builtins.match ".*/public(/.*)?" path != null)
              || (builtins.match ".*/blog(/.*)?" path != null)
              || (builtins.match ".*/app/src/content(/.*)?" path != null)
              || (builtins.match ".*/immich-sdk/openapi(/.*)?" path != null)
              || (builtins.match ".*/app/tailwind.css$" path != null)
              || (builtins.match ".*/data.json$" path != null)
              || (builtins.match ".*/frontend/src/.*\\.js$" path != null)
              || (builtins.match ".*/frontend/src/.*\\.wgsl$" path != null);
            srcFilter = path: type: (siteFilter path type) || (craneLib.filterCargoSources path type);
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
          TAILWIND_CLI = "${tailwindCli}/bin/tailwindcss";
        };

        buildArgs = commonArgs // {
          COMMIT_SHA = self.shortRev or self.dirtyShortRev or "dirty";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        clippy = craneLib.cargoClippy (
          buildArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--workspace -- -Dwarnings";
          }
        );

        fmt = craneLib.cargoFmt commonArgs;

        nextestArchive = craneLib.mkCargoDerivation (
          buildArgs
          // {
            inherit cargoArtifacts;
            pname = "personal-site-nextest-archive";
            doInstallCargoArtifacts = false;
            nativeBuildInputs = commonArgs.nativeBuildInputs ++ [ pkgs.cargo-nextest ];
            buildPhaseCargoCommand = ''
              mkdir -p $out
              cargo nextest archive --workspace --archive-file $out/archive.tar.zst
            '';
            doCheck = false;
          }
        );

        personalSite = craneLib.mkCargoDerivation (
          buildArgs
          // {
            inherit cargoArtifacts;
            doInstallCargoArtifacts = false;
            nativeBuildInputs = commonArgs.nativeBuildInputs ++ [
              topcoatCli
              tailwindCli
              wasmBindgenCli
            ];
            buildPhaseCargoCommand = ''
              cargo build --release -p server
              cargo build --release -p frontend --target wasm32-unknown-unknown

              mkdir -p target/site/pkg
              cp -r public/. target/site/
              wasm-bindgen \
                --target web \
                --out-dir target/site/pkg \
                --out-name frontend \
                target/wasm32-unknown-unknown/release/frontend.wasm
              topcoat asset bundle --release --package server
            '';
            installPhaseCommand = ''
              mkdir -p $out/bin $out/site

              for candidate in \
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
              cp -r target/release/assets $out/bin/assets
            '';
            doCheck = false;
          }
        );

        # Encrypted secrets file (as a derivation so it can be included in the image)
        secretsEncFile = ./secrets.enc.json;
        secretsEnc = pkgs.runCommand "secrets-enc" { } ''
          mkdir -p $out
          cp ${secretsEncFile} $out/secrets.enc.json
        '';

        runApp = pkgs.writeShellScript "run-personal-site" ''
          set -euo pipefail
          cd /app

          # Decrypt secrets to the working directory where the app expects
          # them. Requires SOPS_AGE_KEY to be set (e.g. as a Fly secret);
          # the container fails hard if decryption fails.
          ${pkgs.sops}/bin/sops --decrypt ${secretsEnc}/secrets.enc.json > secrets.json

          exec ${personalSite}/bin/personal_site
        '';

        personalSiteImg = pkgs.dockerTools.streamLayeredImage {
          name = "personal-site";
          tag = "latest";
          contents = [
            personalSite
            pkgs.bashInteractive
            pkgs.cacert
            pkgs.sops
            secretsEnc
          ];
          fakeRootCommands = ''
            mkdir -p ./app
            ln -s ${personalSite}/site ./app/site
          '';
          config = {
            Cmd = [ "${runApp}" ];
            Env = [
              "RUST_LOG=info"
              "APP_ENVIRONMENT=production"
              "SITE_ROOT=site"
              "HOST=0.0.0.0"
              "PORT=8080"
            ];
            ExposedPorts = {
              "8080/tcp" = { };
            };
          };
        };
      in
      {
        packages = {
          inherit
            personalSite
            personalSiteImg
            cargoArtifacts
            nextestArchive
            clippy
            fmt
            ;
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
            cargo-nextest
            cargo-watch
            cargo-info
            cargo-udeps
            cargo-deny
            nix-fast-build.packages.${system}.default
            topcoatCli
            tailwindCli
            wasmBindgenCli
            pkg-config
            openssl
            taplo
            sops
          ];
          TAILWIND_CLI = "${tailwindCli}/bin/tailwindcss";
          # Topcoat launches the binary directly, without Cargo's native-library
          # search paths. rust-s3/native-tls needs OpenSSL at runtime on Linux.
          shellHook = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [ pkgs.openssl ]}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
          '';
        };
      }
    );
}
