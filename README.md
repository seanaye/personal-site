# seanaye.ca

Sean Aye's personal site, built with the [Topcoat generic-pages fork](https://github.com/seanaye/topcoat/tree/seanaye/feat/generic-pages), server-rendered Rust components, Tailwind CSS, and WASM web components.

## Verification

```sh
cargo check --workspace
cargo check -p frontend --target wasm32-unknown-unknown
cargo nextest run --workspace
just build-local
```

## Build and deployment

Build the release server and static site locally:

```sh
just build-local
```

The deployable files are `target/release/server`, `target/release/assets/`, and `target/site/`. Keep `assets/` beside the server binary so Topcoat can load its asset catalog. Set `SITE_ROOT=site`, `HOST=0.0.0.0`, and `PORT=8080` when running the binary from a directory containing `site/`.

The canonical container build uses Nix:

```sh
just build
just deploy
```

The Nix image decrypts `secrets.enc.json` at startup using `SOPS_AGE_KEY`, then runs the Topcoat server on port 8080.

