# Immich-backed photo publishing design

## Goal

Use Immich as the source of truth for photo assets and catalog state, while serving `seanaye.ca` with responsive, CDN-resized images.

The home Immich server is not publicly reachable. `seanaye.ca` will reach it through an Iroh/dumbpipe TCP tunnel, and Cloudflare will resize/cache images in front of `seanaye.ca`.

## Research notes

### Iroh

- Iroh connects peers by public key / endpoint id instead of by stable IP address.
- It attempts direct QUIC connectivity with NAT traversal / hole punching.
- If direct connectivity fails, it can fall back to Iroh relays.
- Connections are authenticated and encrypted by QUIC/TLS.
- This is useful for a home-network Immich server because the public web server does not need inbound access to the home LAN.

Source checked: `https://github.com/n0-computer/iroh` README and `https://docs.iroh.computer` availability.

### dumbpipe

- dumbpipe is a small netcat-like tool built on Iroh.
- It can forward stdin/stdout, TCP sockets, or Unix sockets over an Iroh connection.
- For TCP forwarding:
  - `dumbpipe listen-tcp --host localhost:2283` runs near the target service and forwards incoming Iroh streams to that local TCP service.
  - `dumbpipe connect-tcp --addr 127.0.0.1:32283 <ticket>` runs near the client and exposes a local TCP listener that forwards through Iroh.
- If `IROH_SECRET` is unset, dumbpipe creates a random endpoint identity. For a long-running service, set `IROH_SECRET` so the listener has a stable endpoint and `generate-ticket` can produce a reusable ticket.
- The ticket should be treated as a secret capability: anyone with the ticket can attempt to connect to the listener.

Sources checked: `https://github.com/n0-computer/dumbpipe` README and `dumbpipe --help` from nixpkgs `dumbpipe-0.33.0`.

## Final architecture

```text
Camera card
  ↓
/mnt/storage2/photos/originals
  NEF + XMP sidecars
  ↓ external library
Immich on home server
  ↓
darktable edits RAWs
  ↓
/mnt/storage2/photos/exports
  full-resolution edited JPEGs
  ↓ external library
Immich stack
  JPEG edited export = stack cover / published asset
  NEF original = raw/source asset
  ↓ Immich API over local-only tunnel
Iroh/dumbpipe
  home Immich :2283 → seanaye.ca local 127.0.0.1:32283
  ↓
seanaye.ca backend
  catalog API + `/img/...` image origin endpoint
  ↓
Cloudflare Image Resizing
  `/cdn-cgi/image/.../img/...`
  ↓
Browser
```

## Asset ownership

Immich owns:

- NEF originals as external-library assets.
- XMP sidecars next to NEFs.
- Full-resolution edited JPEG exports as external-library assets.
- Stacks associating each JPEG with its NEF.
- Albums/tags that determine what is public.

`seanaye.ca` owns:

- Public routing, slugs, and presentation.
- The Immich API key.
- A small cache/index of published Immich metadata.
- Public image origin endpoints for Cloudflare to fetch.

Cloudflare owns:

- Responsive transformations.
- AVIF/WebP negotiation via `format=auto`.
- Edge cache for generated sizes.

## Filesystem layout on the home server

```text
/mnt/storage2/immich
  Immich internal managed state only

/mnt/storage2/photos/originals
  YYYY/MM/DD/DSC_1234.NEF
  YYYY/MM/DD/DSC_1234.NEF.xmp

/mnt/storage2/photos/exports
  YYYY/MM/DD/DSC_1234.jpg
```

Both `originals` and `exports` are configured as Immich external libraries.

## Editing workflow

1. Copy camera NEFs into `/mnt/storage2/photos/originals/YYYY/MM/DD/`.
2. Immich external library scan discovers the NEFs.
3. Edit the NEFs in darktable. Darktable writes `.xmp` sidecars.
4. Export full-resolution JPEGs into `/mnt/storage2/photos/exports/YYYY/MM/DD/`.
5. Immich external library scan discovers the JPEGs.
6. Stack JPEG + NEF in Immich.
7. Make the JPEG the stack cover.
8. Add the stack/JPEG to a publishing mechanism, likely a `Published` album.
9. `seanaye.ca` syncs or lazily reads the published assets from Immich.

## Tunnel design

### Home server side

Run dumbpipe next to Immich. It connects incoming Iroh streams to the private Immich HTTP port.

```bash
IROH_SECRET=<stable-home-listener-secret> \
  dumbpipe listen-tcp --host 127.0.0.1:2283
```

Generate a reusable ticket from the same secret:

```bash
IROH_SECRET=<stable-home-listener-secret> dumbpipe generate-ticket
```

Store the resulting ticket as a secret on the public `seanaye.ca` host.

### Public web server side

Run dumbpipe next to the `seanaye.ca` process. It exposes Immich only on loopback.

```bash
dumbpipe connect-tcp \
  --addr 127.0.0.1:32283 \
  "$IMMICH_IROH_TICKET"
```

Then configure the app with:

```text
IMMICH_BASE_URL=http://127.0.0.1:32283
IMMICH_API_KEY=<private immich api key>
```

The tunnel listener must not bind to `0.0.0.0` on the public host. Only the app should reach it.

## seanaye.ca API design

### Metadata endpoints

```http
GET /api/photos
GET /api/photos/:slug
```

These endpoints should return public metadata only, for example:

```json
{
  "id": "lake-sunset",
  "immichAssetId": "redacted-or-opaque",
  "title": "Lake at sunset",
  "takenAt": "2026-06-24T18:30:00Z",
  "width": 6048,
  "height": 4024,
  "aspectRatio": 1.503,
  "imageUrl": "https://seanaye.ca/img/lake-sunset/v/asset-checksum.jpg"
}
```

The API can either:

- read Immich live through the tunnel, or
- sync Immich metadata into a small local cache/database and serve from that.

Prefer a local cache for page rendering so the public site is resilient to temporary home-network outages.

### Image origin endpoint

```http
GET /img/:slug/v/:revision.jpg
```

Behavior:

1. Resolve `slug` to an Immich asset from the local published cache.
2. Verify the asset is public according to Immich catalog state, e.g. in `Published`.
3. Verify the asset is an edited JPEG export, not a NEF.
4. Fetch the full-resolution JPEG bytes from Immich through `IMMICH_BASE_URL` using the private API key.
5. Stream the bytes back with strong cache headers.

Recommended headers for versioned image URLs:

```http
Content-Type: image/jpeg
Cache-Control: public, max-age=31536000, immutable
```

Use a revision derived from Immich metadata, checksum, or file modified time. If the JPEG is re-exported, the revision changes and Cloudflare naturally fetches a new origin URL.

## Cloudflare resizing

The full-resolution endpoint is the origin image:

```text
https://seanaye.ca/img/lake-sunset/v/asset-checksum.jpg
```

Cloudflare Image Resizing fetches that URL through its special prefix:

```text
https://seanaye.ca/cdn-cgi/image/width=1200,quality=85,format=auto/img/lake-sunset/v/asset-checksum.jpg
```

Example responsive markup:

```html
<img
  src="https://seanaye.ca/cdn-cgi/image/width=1200,quality=85,format=auto/img/lake-sunset/v/asset-checksum.jpg"
  srcset="
    https://seanaye.ca/cdn-cgi/image/width=400,quality=85,format=auto/img/lake-sunset/v/asset-checksum.jpg 400w,
    https://seanaye.ca/cdn-cgi/image/width=800,quality=85,format=auto/img/lake-sunset/v/asset-checksum.jpg 800w,
    https://seanaye.ca/cdn-cgi/image/width=1200,quality=85,format=auto/img/lake-sunset/v/asset-checksum.jpg 1200w,
    https://seanaye.ca/cdn-cgi/image/width=2000,quality=85,format=auto/img/lake-sunset/v/asset-checksum.jpg 2000w
  "
  sizes="(max-width: 900px) 100vw, 900px"
  alt="Lake at sunset"
/>
```

Square thumbnail example:

```text
https://seanaye.ca/cdn-cgi/image/width=320,height=320,fit=cover,quality=80,format=auto/img/lake-sunset/v/asset-checksum.jpg
```

Cloudflare flow:

1. Browser requests a `/cdn-cgi/image/...` URL.
2. Cloudflare fetches `/img/...` from `seanaye.ca` on cache miss.
3. `seanaye.ca` fetches the full-res JPEG from Immich through Iroh/dumbpipe.
4. Cloudflare resizes/converts the image.
5. Cloudflare caches that exact transformed variant.
6. Later requests for the same variant are served from Cloudflare edge cache.

## Security model

- Immich is not exposed publicly.
- The dumbpipe connector on the public host binds only `127.0.0.1`.
- The Immich API key is only stored on the `seanaye.ca` server.
- The browser never receives the Immich API key.
- `/img/...` enforces published visibility before proxying bytes.
- `/img/...` refuses RAW files and only serves approved edited JPEG assets.
- Dumbpipe tickets and `IROH_SECRET` are secrets.
- Cloudflare may cache public JPEG derivatives, so only public images should ever be reachable through `/img/...`.

## Failure behavior

If the home network or Immich is unavailable:

- Existing Cloudflare-cached image variants continue to serve.
- Metadata pages can still render from the local `seanaye.ca` cache.
- New uncached image sizes may fail until the tunnel/Immich returns.

This makes aggressive Cloudflare caching and stable versioned URLs important.

## Implementation phases

### Phase 1: Immich catalog source

- Create a `Published` album in Immich.
- Create an Immich API key for `seanaye.ca`.
- Add a backend Immich client module.
- Implement a metadata sync/cache for published JPEG assets.
- Keep current R2/resized-photo path until the Immich path is proven.

### Phase 2: Iroh/dumbpipe tunnel

- Add a home-server systemd service for `dumbpipe listen-tcp --host 127.0.0.1:2283` with stable `IROH_SECRET`.
- Generate and store a dumbpipe ticket.
- Add a public-host systemd service for `dumbpipe connect-tcp --addr 127.0.0.1:32283 <ticket>`.
- Point `seanaye.ca` at `http://127.0.0.1:32283`.
- Verify `GET /api/server/ping` through the tunnel from the public host.

### Phase 3: Image origin endpoint

- Implement `/img/:slug/v/:revision.jpg`.
- Fetch only published edited JPEGs from Immich.
- Stream bytes with long-lived immutable cache headers.
- Add logging for cache misses and Immich/tunnel failures.

### Phase 4: Cloudflare resizing

- Enable Cloudflare Image Resizing for `seanaye.ca`.
- Update frontend photo components to generate `/cdn-cgi/image/.../img/...` URLs.
- Use `srcset`/`sizes` for responsive layouts.
- Use `format=auto` and width-specific variants.

### Phase 5: Remove old resized bucket path

- Replace the current R2 `resized/` listing with Immich-backed metadata.
- Keep R2 only if it remains useful for other static assets.

## Open questions

- Should public URLs use Immich asset IDs, slugs, or generated opaque IDs? Slugs are preferable for stable human-readable URLs.
- Should the published cache live in SQLite, `data.json`, or another store? SQLite is preferable once syncing becomes automatic.
- Should stacking be automated by basename matching, or handled manually in Immich?
- Do we want a separate `Portfolio` album in addition to `Published` for curation?
