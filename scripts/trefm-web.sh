#!/bin/bash
set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WEB_DIR="$PROJECT_ROOT/crates/trefm-web/web"

# --- Config ---
export TREFM_WEB_CONFIG="$PROJECT_ROOT/config/trefm-web.toml"

# --- Build frontend ---
echo "==> Building frontend..."
cd "$WEB_DIR"
npm run build

# --- Build backend (release) ---
echo "==> Building backend (release)..."
cd "$PROJECT_ROOT"
cargo build -p trefm-web --bin trefm-web --release

# --- Kill existing process if running ---
pkill -f 'target/release/trefm-web' 2>/dev/null || true
sleep 1

# --- Start server ---
echo "==> Starting trefm-web..."
exec "$PROJECT_ROOT/target/release/trefm-web"
