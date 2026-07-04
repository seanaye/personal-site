# Run the development server
server:
    cargo leptos watch

# Alias used while developing locally
[parallel]
dev: server

# Build the deployable container image
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
