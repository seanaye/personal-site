# seanaye.ca

Sean Aye's personal site, built with the [Topcoat generic-pages fork](https://github.com/seanaye/topcoat/tree/seanaye/feat/generic-pages), server-rendered Rust components, Tailwind CSS, and WASM web components.

The framework and CLI are pinned to commit `9a039d010e9fd75ba4f33ef2ed60ea8c04b79a77`. Keep the pins in `Cargo.toml`, `app/Cargo.toml`, `flake.nix`, and `Dockerfile` synchronized when updating the branch. Generic `#[page]` handlers are registered explicitly with `.page(handler::<State>::default())`; shared state is registered with `.with(state)` and read with `context::<State>(cx)`.

## Architecture

- `app/src/domain/` — blog models/services and the bucket-storage and shared-photo ports.
- `app/src/web/` and `app/src/web.rs` — Topcoat pages, layouts, components, Markdown rendering, photo grids, and route setup.
- `server/src/domain/` — token claims and signing; `inbound/` — streaming image HTTP handler; `outbound/` — bucket and shared-photo adapters. `main.rs` wires the concrete adapters, configuration, and static assets.
- `frontend/` — `wasm-bindgen` browser controllers and the WGPU droplet renderer.
- `liquid/` — palette generation, color calculations, and liquid event types shared by SSR and WASM.
- `grid/`, `photogrid/`, `photo-search/` — responsive photo layout and filtering.
- `images/src/domain/` — image models, repository port, and generic image service; `images/src/outbound/` — Immich and in-memory adapters.
- `immich-sdk/`, `dumbpipe-client/` — Immich HTTP client and tunnel transport.

Ports use `impl Future + Send` and generic consumers rather than trait objects or boxed futures. `SharedPhotosFetcher` is implemented by `ImmichSharedPhotosFetcher` and selected when constructing the web router. Image byte streams remain boxed.

HTTP clients use rustls, including the shared `rust-s3` dependency. The server does not require OpenSSL libraries or a runtime library-path override.

The browser registers these light-DOM custom elements over server-rendered markup:

- `<liquid-canvas>` owns the WGPU canvas, animation, resizing, and droplet input.
- `<poline-background>` updates static page backgrounds.
- `<poline-palette>` updates the color swatches along the left edge.
- `<hue-slider>` saves its raw value (0–360) in a site-wide `liquid-hue-v2` cookie. SSR uses that value directly for the slider and adds the fixed 171° offset to generate the Poline background and palette before WASM loads.

The WGSL simulation and display shaders remain in `frontend/src/shaders/`. Topcoat's Cargo build integration compiles Tailwind 4.3.3 with CSS-native configuration and serves the result as a content-hashed asset. The avatar, favicon, and bootstrap script use `asset!` and ship in the same bundle; only generated JS/WASM is served from `target/site/pkg/` at `/pkg/`.

## Development

Enter the Nix development shell:

```sh
nix develop
```

Then start the watched server and asset builds:

```sh
just dev
```

Open <http://127.0.0.1:3000>. For a single unwatched run, use `just server`. `just dev` runs `topcoat dev` for SSR/Tailwind alongside a Cargo watcher for the custom-element WASM.

Useful individual commands:

```sh
just frontend   # WASM + wasm-bindgen -> target/site/pkg/
just assets     # Topcoat Tailwind/asset catalog, WASM, and public assets
just server     # Build assets and run the Topcoat server
```

The server reads `SITE_ROOT` (default `target/site`), `HOST` (default `127.0.0.1`), and `PORT` (default `3000`). Local secrets are loaded from `secrets.example.json` and the gitignored `secrets.json`; see `just decrypt-secrets` when a fresh local file is needed.

Without Nix, install a Rust nightly new enough for Topcoat, the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` 0.2.130 (matching `Cargo.lock`), `cargo-watch`, and `just`. Install the CLI from the same fork revision:

```sh
cargo install topcoat-cli --git https://github.com/seanaye/topcoat --rev 9a039d010e9fd75ba4f33ef2ed60ea8c04b79a77 --locked
```

Topcoat downloads and caches the pinned Tailwind CLI automatically; no separate Tailwind installation is needed.

## Routes

- `/` — bio and interactive liquid canvas
- `/blog` and `/blog/{slug}` — blog index and Markdown posts (`?draft=true` includes drafts)
- `/photo` — photography grid
- `/search?a=...&b=...&r=...` — filtered photo grid
- `/photo/{token}` — signed shared-photo page
- `/image/{token}?size=thumbnail|preview|fullsize` — signed streaming image bytes
- `/file/{path...}` — redirect to a one-hour presigned R2 URL for `public/{path...}`

## Private R2 downloads

`data.json` stores unsigned S3 URLs under `https://ce4df2c17b89e05aec7b31303383defe.r2.cloudflarestorage.com/personal-photos/`. The bucket adapter uses the configured R2 credentials to generate one-hour SigV4 GET URLs for each gallery response and `/file/...` redirect. Signing is local: image bytes go directly from R2 to the browser. The cached catalog never stores expiring signatures, and URLs from another origin or bucket are rejected.

Configure `r2.account_id`, `r2.bucket_name`, `r2.access_key`, and `r2.secret_key` in `secrets.json` (and the encrypted deployment configuration). The account and bucket must match the catalog; the credentials must grant object-read access to that bucket. Reload a gallery page to renew URLs after an hour, including images that have not yet lazy-loaded. The public pages and file route still issue links to any visitor; this makes direct bucket access require a signature, not the website itself private. Immich share links are unchanged.

After deploying and verifying signed downloads, disable/remove the old `cdn.seanaye.ca` R2 custom domain, disable the public `r2.dev` URL, and remove any other public bucket domains. These Cloudflare settings are a separate manual cutover; changing this code does not disable existing public access. No Worker or WAF rule is required.

## Full-size photos

Immich generates full-size renditions with `image.fullsize.enabled`, including for new RAW imports. Existing images can be backfilled using Immich's bulk thumbnail-regeneration job; previews remain at 1440px on the short edge.

Clicking a shared photo opens its already-generated full-size image in a new tab through the signed image endpoint. The site never submits generation jobs or polls job status. Until an asset is processed, Immich may return its preview instead. Full-size links use a refreshed cache key and a five-minute cache lifetime so old preview fallbacks are not retained indefinitely.

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

