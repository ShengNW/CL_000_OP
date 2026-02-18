#!/usr/bin/env bash
set -euo pipefail

ROOT="/mnt/f/aw-omni/src"
LOG_DIR="/mnt/f/aw-omni/runtime/logs"
PID_DIR="/mnt/f/aw-omni/runtime/pid"
SIDECAR="$ROOT/sidecar/omni_sidecar_entry.py"

mkdir -p "$LOG_DIR" "$PID_DIR"

python3 "$SIDECAR" --mode mock --host 127.0.0.1 --port 8000 > "$LOG_DIR/sidecar_mock.log" 2>&1 &
SIDECAR_PID=$!
echo "$SIDECAR_PID" > "$PID_DIR/sidecar_mock.pid"

sleep 0.3

cd "$ROOT"
cargo run -p aw_omni_daemon -- --config config/local.wsl.toml health

wait "$SIDECAR_PID"
