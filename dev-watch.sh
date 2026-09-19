#!/usr/bin/env bash
# Dev loop: rebuild + relaunch agenda-gpui when sources change.
# NB: not real HMR — Rust/GPUI can't hot-patch a running process, so this
# kills the app, rebuilds and starts a fresh window on every save.
# Usage: ./dev-watch.sh   (Ctrl+C stops the watcher AND the app)
set -u
cd "$(dirname "$0")"

EXE="target/debug/agenda-gpui.exe"
PID_FILE="target/.dev-watch.pid"
LOG="target/dev-app.log"

snapshot() {
    find src assets build.rs Cargo.toml -type f -printf '%T@ %p\n' 2>/dev/null | sort -n | tail -1
}

stop_app() {
    if [ -f "$PID_FILE" ]; then
        taskkill //PID "$(cat "$PID_FILE")" //F >/dev/null 2>&1
        rm -f "$PID_FILE"
    fi
    taskkill //IM agenda-gpui.exe //F >/dev/null 2>&1
}

start_app() {
    "$EXE" >>"$LOG" 2>&1 &
    echo $! >"$PID_FILE"
    echo "[dev-watch] launched $(cat "$PID_FILE")"
}

trap 'stop_app; exit 0' INT TERM

echo "[dev-watch] watching src/, assets/, build.rs — Ctrl+C to stop"
stop_app
prev=$(snapshot)
if cargo build; then
    start_app
fi
while :; do
    sleep 1
    cur=$(snapshot)
    if [ "$cur" != "$prev" ]; then
        prev="$cur"
        echo "[dev-watch] change detected, rebuilding…"
        stop_app
        if cargo build; then
            start_app
        else
            echo "[dev-watch] build failed — fix and save again"
        fi
    fi
done
