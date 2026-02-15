#!/bin/bash
set -e

SESSION="trefm-remote"
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Kill existing session if any
tmux kill-session -t "$SESSION" 2>/dev/null || true

# Create new tmux session with trefm-web
tmux new-session -d -s "$SESSION" -n "remote" \
  "bash $PROJECT_ROOT/scripts/trefm-web.sh; read"

# Split horizontally and run cloudflared tunnel
tmux split-window -h -t "$SESSION" \
  "echo '==> Waiting 3s for server...' && sleep 3 && cloudflared tunnel run trefm; read"

# Attach to the session
tmux attach-session -t "$SESSION"
