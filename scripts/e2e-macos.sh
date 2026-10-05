#!/usr/bin/env bash
# End-to-end run of Agenda against a throwaway Mundus Engine, with screenshots.
#
# Usage: scripts/e2e-macos.sh [out-dir]
#   MUNDUS_ENGINE_BIN  Engine binary (default: ../cortex/target/debug/mundus-engine)
#
# Scenarios (src/e2e/): create → persist (fresh Agenda process, same data)
# → offline (Engine stopped and restarted mid-run). Each writes PNGs and
# report.txt into its own subdirectory; exit status is non-zero on any FAIL.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
out="${1:-$root/target/e2e}"
engine_bin="${MUNDUS_ENGINE_BIN:-$root/../cortex/target/debug/mundus-engine}"
[ -x "$engine_bin" ] || { echo "Engine binary not found: $engine_bin (build it: cargo build -p engine --bin mundus-engine in cortex)" >&2; exit 1; }

data="$(mktemp -d /tmp/agenda-e2e.XXXXXX)"
rm -rf "$out"
mkdir -p "$out"
engine_pid=""

start_engine() {
	rm -f "$data/engine.lock.json"
	MUNDUS_DATA_DIR="$data" "$engine_bin" >>"$out/engine.log" 2>&1 &
	engine_pid=$!
	for _ in $(seq 1 150); do
		[ -s "$data/engine.lock.json" ] && return 0
		sleep 0.2
	done
	echo "Engine did not start, see $out/engine.log" >&2
	exit 1
}

stop_engine() {
	[ -n "$engine_pid" ] || return 0
	kill "$engine_pid" 2>/dev/null || true
	wait "$engine_pid" 2>/dev/null || true
	engine_pid=""
}

cleanup() {
	stop_engine
	rm -rf "$data"
}
trap cleanup EXIT

# Own target dir: a `cargo watch` on the same checkout rebuilds
# target/debug/agenda-gpui without the feature and would swap the binary.
(cd "$root" && cargo build --features e2e --quiet --target-dir "$root/target/e2e-build")
agenda="$root/target/e2e-build/debug/agenda-gpui"

run_scenario() {
	local name="$1"
	mkdir -p "$out/$name"
	AGENDA_E2E="$name" AGENDA_E2E_OUT="$out/$name" MUNDUS_DATA_DIR="$data" \
		"$agenda" >"$out/$name/agenda.log" 2>&1 &
	local pid=$!
	local deadline=$((SECONDS + 300))
	# The offline scenario asks us to stop and restart Engine via marker files.
	while kill -0 "$pid" 2>/dev/null; do
		if [ -e "$out/$name/stop-engine" ] && [ ! -e "$out/$name/engine-stopped" ]; then
			stop_engine
			touch "$out/$name/engine-stopped"
		fi
		if [ -e "$out/$name/start-engine" ] && [ ! -e "$out/$name/engine-started" ]; then
			start_engine
			touch "$out/$name/engine-started"
		fi
		if [ "$SECONDS" -ge "$deadline" ]; then
			kill "$pid" 2>/dev/null || true
			echo "timeout" >"$out/$name/FAILED"
			break
		fi
		sleep 0.2
	done
	wait "$pid" || true
	echo "== $name"
	if [ -s "$out/$name/report.txt" ]; then
		cat "$out/$name/report.txt"
	else
		echo "FAIL no report (crash or timeout? see $out/$name/agenda.log)"
		touch "$out/$name/FAILED"
	fi
}

start_engine
run_scenario create
run_scenario persist
run_scenario offline

if grep -rqs "^FAIL" "$out"/*/report.txt || ls "$out"/*/FAILED >/dev/null 2>&1; then
	echo "E2E: FAIL (screenshots and logs in $out)"
	exit 1
fi
echo "E2E: PASS (screenshots in $out)"
