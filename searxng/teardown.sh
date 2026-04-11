#!/usr/bin/env bash
# teardown.sh — Stop SearXNG services

set -e

CONFIG_DIR="$HOME/.config/ripweb/searxng"

if [ -d "$CONFIG_DIR" ]; then
    echo "==> Stopping SearXNG in $CONFIG_DIR"
    cd "$CONFIG_DIR"
    docker compose down
else
    echo "SearXNG config directory not found at $CONFIG_DIR"
fi
