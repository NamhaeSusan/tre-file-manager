#!/bin/bash
set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WEB_DIR="$PROJECT_ROOT/crates/trefm-web/web"
BIN="$PROJECT_ROOT/target/release/trefm-web"
export TREFM_WEB_CONFIG="$PROJECT_ROOT/config/trefm-web.toml"

usage() {
  echo "Usage: $0 [quick|full|stop|status]"
  echo ""
  echo "  quick   Kill + restart (no rebuild, default)"
  echo "  full    Kill + rebuild frontend & backend + restart"
  echo "  stop    Kill only"
  echo "  status  Show running process"
  exit 1
}

do_stop() {
  if pkill -f 'target/release/trefm-web' 2>/dev/null; then
    echo "==> Stopped trefm-web"
    sleep 1
  else
    echo "==> trefm-web is not running"
  fi
}

do_status() {
  if pgrep -f 'target/release/trefm-web' >/dev/null 2>&1; then
    echo "==> trefm-web is running (PID: $(pgrep -f 'target/release/trefm-web'))"
  else
    echo "==> trefm-web is not running"
  fi
}

do_build() {
  echo "==> Building frontend..."
  cd "$WEB_DIR"
  npm run build

  echo "==> Building backend (release)..."
  cd "$PROJECT_ROOT"
  cargo build -p trefm-web --bin trefm-web --release
}

do_start() {
  if [ ! -f "$BIN" ]; then
    echo "==> Binary not found. Run '$0 full' first."
    exit 1
  fi

  echo "==> Starting trefm-web (nohup)"
  nohup "$BIN" >/dev/null 2>&1 &
  sleep 1

  if pgrep -f 'target/release/trefm-web' >/dev/null 2>&1; then
    echo "==> trefm-web started (PID: $(pgrep -f 'target/release/trefm-web'))"
  else
    echo "==> Failed to start"
    exit 1
  fi
}

CMD="${1:-quick}"

case "$CMD" in
  quick)
    do_stop
    do_start
    ;;
  full)
    do_stop
    do_build
    do_start
    ;;
  stop)
    do_stop
    ;;
  status)
    do_status
    ;;
  *)
    usage
    ;;
esac
