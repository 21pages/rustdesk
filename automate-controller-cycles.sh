#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./automate-controller-cycles.sh \
    --app-binary <rustdesk_binary> \
    --remote-id <id> \
    [--password <password>] \
    [--cycles <count>] \
    [--cycle-seconds 60] \
    [--disconnect-seconds 30] \
    [--startup-seconds 5]

What it does:
  1. Starts a fresh controller RustDesk process with `--connect`.
  2. Waits for startup slack plus the built-in disconnect timeout to pass.
  4. Kills the controller process to reset the next cycle.
  5. Repeats on a 60-second cadence by default.

Notes:
  - This script is for controller-side automation.
  - Controlled-side memory logging should come from the modified `src/hbbs_http/sync.rs`.
  - If `--cycles` is omitted, it runs forever.
  - This script assumes the controller build accepts `--connect <id> --password <password>`.
EOF
}

app_binary=""
remote_id=""
password=""
cycles=""
cycle_seconds=60
disconnect_seconds=30
startup_seconds=5

while [[ $# -gt 0 ]]; do
  case "$1" in
    --app-binary)
      app_binary="$2"
      shift 2
      ;;
    --remote-id)
      remote_id="$2"
      shift 2
      ;;
    --password)
      password="$2"
      shift 2
      ;;
    --cycles)
      cycles="$2"
      shift 2
      ;;
    --cycle-seconds)
      cycle_seconds="$2"
      shift 2
      ;;
    --disconnect-seconds)
      disconnect_seconds="$2"
      shift 2
      ;;
    --startup-seconds)
      startup_seconds="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown arg: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

for required in app_binary remote_id; do
  if [[ -z "${!required}" ]]; then
    echo "Missing required arg: --${required//_/-}" >&2
    usage >&2
    exit 1
  fi
done

for n in cycle_seconds disconnect_seconds startup_seconds; do
  if ! [[ "${!n}" =~ ^[0-9]+$ ]] || (( ${!n} <= 0 )); then
    echo "Invalid numeric arg for $n: ${!n}" >&2
    exit 1
  fi
done

if [[ -n "$cycles" ]] && { ! [[ "$cycles" =~ ^[0-9]+$ ]] || (( cycles <= 0 )); }; then
  echo "Invalid numeric arg for cycles: $cycles" >&2
  exit 1
fi

if (( cycle_seconds <= disconnect_seconds + startup_seconds )); then
  echo "--cycle-seconds must be greater than --disconnect-seconds + --startup-seconds" >&2
  exit 1
fi

if [[ ! -x "$app_binary" ]]; then
  echo "App binary is not executable: $app_binary" >&2
  exit 1
fi

cleanup_pid() {
  local pid="$1"
  if [[ -z "$pid" ]]; then
    return
  fi
  if ps -p "$pid" >/dev/null 2>&1; then
    kill -9 "$pid" >/dev/null 2>&1 || true
    wait "$pid" 2>/dev/null || true
  fi
}

cycle=1
while true; do
  cycle_start="$(date +%s)"
  echo "[$(date '+%F %T')] cycle=$cycle start"

  app_args=("$app_binary" "--connect" "$remote_id")
  if [[ -n "$password" ]]; then
    app_args+=("--password" "$password")
  fi
  "${app_args[@]}" >/tmp/rustdesk-controller-cycle-"$cycle".log 2>&1 &
  controller_pid=$!
  echo "[$(date '+%F %T')] cycle=$cycle controller_pid=$controller_pid"

  sleep "$startup_seconds"
  echo "[$(date '+%F %T')] cycle=$cycle connect_started"

  sleep "$disconnect_seconds"
  echo "[$(date '+%F %T')] cycle=$cycle disconnect_wait_elapsed"

  cleanup_pid "$controller_pid"
  echo "[$(date '+%F %T')] cycle=$cycle controller_cleaned_up"

  elapsed=$(( $(date +%s) - cycle_start ))
  if (( elapsed < cycle_seconds )); then
    sleep $((cycle_seconds - elapsed))
  fi
  if [[ -n "$cycles" ]] && (( cycle >= cycles )); then
    break
  fi
  cycle=$((cycle + 1))
done

echo "[$(date '+%F %T')] all cycles finished"
