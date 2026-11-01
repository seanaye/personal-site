FROM rustlang/rust:nightly-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install wasm-bindgen-cli --version 0.2.130 --locked
RUN cargo install topcoat-cli --git https://github.com/seanaye/topcoat --rev 9a039d010e9fd75ba4f33ef2ed60ea8c04b79a77 --locked
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app
COPY . .

RUN mkdir -p target/site/pkg \
    && cp -r public/. target/site/ \
    && cargo build --release -p server \
    && cargo build --release -p frontend --target wasm32-unknown-unknown \
    && wasm-bindgen \
      --target web \
      --out-dir target/site/pkg \
      --out-name frontend \
      target/wasm32-unknown-unknown/release/frontend.wasm \
    && topcoat asset bundle --release --package server

FROM debian:bookworm-slim AS runner
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/server /app/server
COPY --from=builder /app/target/site /app/site
COPY --from=builder /app/target/release/assets /app/assets
WORKDIR /app

ENV RUST_LOG="info"
ENV APP_ENVIRONMENT="production"
ENV SITE_ROOT="site"
ENV HOST="0.0.0.0"
ENV PORT="8080"
EXPOSE 8080

# Provide /app/secrets.json as a runtime secret or mount.
CMD ["/app/server"]
