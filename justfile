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
    docker tag personal-site:latest registry.fly.io/personal-site:{{ tag }}
    docker tag personal-site:latest registry.fly.io/personal-site:latest
    docker push registry.fly.io/personal-site:{{ tag }}
    docker push registry.fly.io/personal-site:latest

# Deploy to Fly.io
deploy tag=sha: (push tag)
    flyctl deploy --image registry.fly.io/personal-site:{{ tag }}
