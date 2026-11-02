# Assemble static assets and run the Topcoat server.
server: assets
    cargo run -p server

# Build everything before running the two development watchers.
dev: assets dev-watch

# Topcoat rebuilds SSR/Tailwind assets while Cargo rebuilds the custom elements.
[parallel]
dev-watch: dev-server dev-frontend

dev-server:
    topcoat dev --package server

dev-frontend:
    cargo watch -w frontend -w liquid -w grid -- just frontend

# Prepare the directory served for generated JS/WASM.
site:
    mkdir -p target/site/pkg

# Build the custom elements and generate their ES-module loader.
frontend: site
    cargo build -p frontend --target wasm32-unknown-unknown
    wasm-bindgen --target web --out-dir target/site/pkg --out-name frontend target/wasm32-unknown-unknown/debug/frontend.wasm

# Build browser assets and Topcoat's Tailwind stylesheet/asset catalog.
assets: site frontend
    topcoat asset bundle --package server

# Verify a complete local release build.
build-local: site
    cargo build --release -p server
    cargo build --release -p frontend --target wasm32-unknown-unknown
    wasm-bindgen --target web --out-dir target/site/pkg --out-name frontend target/wasm32-unknown-unknown/release/frontend.wasm
    topcoat asset bundle --release --package server

# Build the deployable container image.
build:
    nix build .#personalSiteImg

# Current commit SHA (supports jj workspaces and git)

sha := `jj log -r @ --no-graph -T 'commit_id.short(7)' 2>/dev/null || git rev-parse --short HEAD`

# Load the built image into Docker
load: build
    ./result | docker load

# Push to Fly.io registry (requires FLY_API_TOKEN)
push tag=sha: load
    flyctl auth docker
    docker tag personal-site:latest registry.fly.io/personal-site-astvuq:{{ tag }}
    docker tag personal-site:latest registry.fly.io/personal-site-astvuq:latest
    docker push registry.fly.io/personal-site-astvuq:{{ tag }}
    docker push registry.fly.io/personal-site-astvuq:latest

# Deploy to Fly.io
deploy tag=sha: (push tag)
    flyctl deploy --image registry.fly.io/personal-site-astvuq:{{ tag }}

# Encrypt secrets.json to secrets.enc.json (recipients come from .sops.yaml)
encrypt-secrets:
    sops --encrypt secrets.json > secrets.enc.json

# Decrypt secrets.enc.json to secrets.json for local development
decrypt-secrets:
    sops decrypt secrets.enc.json > secrets.json

# Edit secrets.enc.json in-place using YubiKey
edit-secrets:
    sops edit secrets.enc.json
