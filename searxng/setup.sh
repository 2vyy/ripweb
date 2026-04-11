#!/usr/bin/env bash
# setup.sh — Automated SearXNG setup for ripweb

set -e

CONFIG_DIR="$HOME/.config/ripweb/searxng"
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "==> Setting up SearXNG in $CONFIG_DIR"

mkdir -p "$CONFIG_DIR"
cp "$REPO_DIR/settings.yml" "$CONFIG_DIR/settings.yml"
cp "$REPO_DIR/docker-compose.yml" "$CONFIG_DIR/docker-compose.yml"

cd "$CONFIG_DIR"

if ! command -v docker &> /dev/null; then
    echo "Error: docker is not installed. Please install Docker and Docker Compose."
    exit 1
fi

echo "==> Starting SearXNG..."
docker compose up -d

echo "==> Waiting for SearXNG to be healthy..."
for i in {1..10}; do
    if curl -s "http://localhost:8080/healthz" > /dev/null; then
        echo "==> SearXNG is up and running at http://localhost:8080"
        echo "==> Verify JSON output:"
        echo "    curl \"http://localhost:8080/search?q=rust+tokio&format=json\" | head -c 100"
        exit 0
    fi
    sleep 2
done

echo "Warning: SearXNG didn't report healthy within 20s. Check logs with 'docker compose logs'."
