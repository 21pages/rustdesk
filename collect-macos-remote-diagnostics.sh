#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./collect-macos-remote-diagnostics.sh --pid <pid> [options]

Options:
  --pid <pid>                  RustDesk controlled-side pid to inspect
  --interval <seconds>         Snapshot interval, default: 30
  --out-dir <path>             Output directory, default: ./macos-diag-<pid>-<timestamp>
  --log-dir <path>             RustDesk log directory, default: ~/Library/Logs/RustDesk
  --tail-lines <count>         Number of recent matching log lines to save, default: 200
  --full-vmmap-every <count>   Also dump full vmmap every N snapshots, default: 0
  -h, --help                   Show this help

What it collects each snapshot:
  - ps rss/vsz/state/etime summary
  - vmmap -summary output
  - parsed vmmap memory summary:
      Physical footprint
      TOTAL virtual
      TOTAL resident
  - lsof TCP sockets for the pid
  - lsof UDP sockets for the pid
  - focused lsof TCP states: ESTABLISHED and CLOSE_WAIT
  - parsed socket counts:
      TCP total / IPv4 / IPv6
      UDP total / IPv4 / IPv6
      TCP CLOSE_WAIT / ESTABLISHED
  - netstat lines filtered to RustDesk TCP/UDP ports and common TCP states
  - recent RustDesk log excerpts:
      conn-monitor
      receive close reason
      Connection closed:
      Failed to accept connection
      Handshake failed
      Broken pipe
      Timeout
      Reset by the peer
      Peer close

Stop:
  Ctrl+C

Notes:
  - macOS only, requires: vmmap, lsof, netstat
  - ripgrep (`rg`) is optional; the script falls back to `grep`
  - full vmmap is much heavier than vmmap -summary; keep --full-vmmap-every at 0 unless needed
EOF
}

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Missing required command: $cmd" >&2
    exit 1
  fi
}

filter_logs() {
  local pattern="$1"
  local target="$2"
  if command -v rg >/dev/null 2>&1; then
    rg -n "$pattern" "$target"
  else
    grep -R -n -E "$pattern" "$target"
  fi
}

filter_netstat() {
  local pattern="$1"
  if command -v rg >/dev/null 2>&1; then
    rg "$pattern"
  else
    grep -E "$pattern"
  fi
}

timestamp_slug() {
  date '+%Y%m%d-%H%M%S'
}

timestamp_human() {
  date '+%F %T'
}

to_mib() {
  awk -v kib="$1" 'BEGIN { printf "%.2f", kib / 1024 }'
}

size_str_to_mib() {
  awk -v raw="$1" '
    BEGIN {
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", raw)
      if (raw == "") {
        exit 1
      }
      unit = substr(raw, length(raw), 1)
      value = raw
      factor = 1
      if (unit ~ /[KMGTP]/) {
        value = substr(raw, 1, length(raw) - 1)
        if (unit == "K") {
          factor = 1 / 1024
        } else if (unit == "M") {
          factor = 1
        } else if (unit == "G") {
          factor = 1024
        } else if (unit == "T") {
          factor = 1024 * 1024
        } else if (unit == "P") {
          factor = 1024 * 1024 * 1024
        }
      }
      printf "%.2f", value * factor
    }
  '
}

parse_vmmap_summary() {
  local file="$1"
  awk '
    /^Physical footprint:/ {
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", $3)
      physical = $3
    }
    /^TOTAL[[:space:]]+/ && total_virtual == "" {
      total_virtual = $2
      total_resident = $3
    }
    END {
      if (physical == "" || total_virtual == "" || total_resident == "") {
        exit 1
      }
      print physical "\t" total_virtual "\t" total_resident
    }
  ' "$file"
}

count_lsof_rows() {
  local file="$1"
  awk '
    NR > 1 && $0 !~ /^\[command-exit=/ { c++ }
    END { print c + 0 }
  ' "$file"
}

count_lsof_family_rows() {
  local file="$1"
  local family="$2"
  awk -v family="$family" '
    NR > 1 && $0 !~ /^\[command-exit=/ && $5 == family { c++ }
    END { print c + 0 }
  ' "$file"
}

count_lsof_state_rows() {
  local file="$1"
  local state="$2"
  awk -v state="$state" '
    NR > 1 && $0 !~ /^\[command-exit=/ && $0 ~ state { c++ }
    END { print c + 0 }
  ' "$file"
}

read_ps_sample() {
  ps -o rss= -o vsz= -o state= -o etime= -o command= -p "$pid" | awk 'NF >= 5 { rss=$1; vsz=$2; state=$3; etime=$4; $1=""; $2=""; $3=""; $4=""; sub(/^ +/, "", $0); print rss "\t" vsz "\t" state "\t" etime "\t" $0; exit }'
}

run_capture() {
  local out="$1"
  shift
  if "$@" >"$out" 2>&1; then
    return 0
  fi
  local status=$?
  {
    echo
    echo "[command-exit=$status]"
  } >>"$out"
}

pid=""
interval=30
out_dir=""
log_dir="$HOME/Library/Logs/RustDesk"
tail_lines=200
full_vmmap_every=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pid)
      pid="$2"
      shift 2
      ;;
    --interval)
      interval="$2"
      shift 2
      ;;
    --out-dir)
      out_dir="$2"
      shift 2
      ;;
    --log-dir)
      log_dir="$2"
      shift 2
      ;;
    --tail-lines)
      tail_lines="$2"
      shift 2
      ;;
    --full-vmmap-every)
      full_vmmap_every="$2"
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

if [[ -z "$pid" ]]; then
  echo "Missing required arg: --pid" >&2
  usage >&2
  exit 1
fi

if ! [[ "$pid" =~ ^[0-9]+$ ]]; then
  echo "Invalid pid: $pid" >&2
  exit 1
fi

for n in interval tail_lines full_vmmap_every; do
  if ! [[ "${!n}" =~ ^[0-9]+$ ]]; then
    echo "Invalid numeric arg for $n: ${!n}" >&2
    exit 1
  fi
done

if (( interval <= 0 )); then
  echo "--interval must be greater than 0" >&2
  exit 1
fi

require_cmd vmmap
require_cmd lsof
require_cmd netstat
require_cmd ps
require_cmd awk
require_cmd grep

if ! ps -p "$pid" >/dev/null 2>&1; then
  echo "Process $pid does not exist" >&2
  exit 1
fi

if [[ -z "$out_dir" ]]; then
  out_dir="./macos-diag-${pid}-$(timestamp_slug)"
fi

mkdir -p "$out_dir"

summary_csv="$out_dir/summary.csv"
vmmap_memory_csv="$out_dir/vmmap-memory.csv"
meta_txt="$out_dir/README.txt"

cat >"$meta_txt" <<EOF
pid=$pid
started_at=$(timestamp_human)
interval_seconds=$interval
log_dir=$log_dir
tail_lines=$tail_lines
full_vmmap_every=$full_vmmap_every
EOF

printf "%s\n" \
"timestamp,rss_mib,vsz_mib,vmmap_physical_mib,vmmap_total_virtual_mib,vmmap_total_resident_mib,state,etime,close_wait_count,established_count,tcp_socket_count,tcp_ipv4_count,tcp_ipv6_count,udp_socket_count,udp_ipv4_count,udp_ipv6_count,vmmap_summary_file,lsof_tcp_file,lsof_udp_file,lsof_focus_file,netstat_file,log_excerpt_file" \
>"$summary_csv"

printf "%s\n" \
"timestamp,physical_footprint_mib,total_virtual_mib,total_resident_mib,physical_vs_base_mib,physical_vs_prev_mib,virtual_vs_base_mib,virtual_vs_prev_mib,resident_vs_base_mib,resident_vs_prev_mib,vmmap_summary_file" \
>"$vmmap_memory_csv"

echo "Collecting diagnostics for pid=$pid interval=${interval}s"
echo "Output dir: $out_dir"
echo "Press Ctrl+C to stop"

snapshot_index=0
vmmap_base_physical_mib=""
vmmap_base_virtual_mib=""
vmmap_base_resident_mib=""
vmmap_prev_physical_mib=""
vmmap_prev_virtual_mib=""
vmmap_prev_resident_mib=""

while true; do
  if ! ps -p "$pid" >/dev/null 2>&1; then
    echo "Process $pid exited"
    exit 0
  fi

  snapshot_index=$((snapshot_index + 1))
  ts_human="$(timestamp_human)"
  ts_slug="$(timestamp_slug)"

  sample="$(read_ps_sample)"
  if [[ -z "$sample" ]]; then
    echo "Failed to read ps sample for pid $pid" >&2
    exit 1
  fi

  IFS=$'\t' read -r rss_kib vsz_kib state etime command_text <<<"$sample"

  vmmap_summary_file="$out_dir/vmmap-summary-${ts_slug}.txt"
  lsof_tcp_file="$out_dir/lsof-tcp-${ts_slug}.txt"
  lsof_udp_file="$out_dir/lsof-udp-${ts_slug}.txt"
  lsof_focus_file="$out_dir/lsof-tcp-focus-${ts_slug}.txt"
  netstat_file="$out_dir/netstat-filtered-${ts_slug}.txt"
  log_excerpt_file="$out_dir/log-excerpt-${ts_slug}.txt"

  run_capture "$vmmap_summary_file" vmmap -summary "$pid"
  run_capture "$lsof_tcp_file" lsof -nP -p "$pid" -a -iTCP
  run_capture "$lsof_udp_file" lsof -nP -p "$pid" -a -iUDP
  run_capture "$lsof_focus_file" lsof -nP -p "$pid" -a -iTCP -sTCP:CLOSE_WAIT,ESTABLISHED

  vmmap_sample="$(parse_vmmap_summary "$vmmap_summary_file" 2>/dev/null || true)"
  if [[ -z "$vmmap_sample" ]]; then
    vmmap_physical_mib=""
    vmmap_total_virtual_mib=""
    vmmap_total_resident_mib=""
    vmmap_physical_vs_base_mib=""
    vmmap_physical_vs_prev_mib=""
    vmmap_virtual_vs_base_mib=""
    vmmap_virtual_vs_prev_mib=""
    vmmap_resident_vs_base_mib=""
    vmmap_resident_vs_prev_mib=""
  else
    IFS=$'\t' read -r vmmap_physical_raw vmmap_total_virtual_raw vmmap_total_resident_raw <<<"$vmmap_sample"
    vmmap_physical_mib="$(size_str_to_mib "$vmmap_physical_raw")"
    vmmap_total_virtual_mib="$(size_str_to_mib "$vmmap_total_virtual_raw")"
    vmmap_total_resident_mib="$(size_str_to_mib "$vmmap_total_resident_raw")"

    if [[ -z "$vmmap_base_physical_mib" ]]; then
      vmmap_base_physical_mib="$vmmap_physical_mib"
      vmmap_base_virtual_mib="$vmmap_total_virtual_mib"
      vmmap_base_resident_mib="$vmmap_total_resident_mib"
    fi

    if [[ -z "$vmmap_prev_physical_mib" ]]; then
      vmmap_prev_physical_mib="$vmmap_physical_mib"
      vmmap_prev_virtual_mib="$vmmap_total_virtual_mib"
      vmmap_prev_resident_mib="$vmmap_total_resident_mib"
    fi

    vmmap_physical_vs_base_mib="$(awk -v now="$vmmap_physical_mib" -v base="$vmmap_base_physical_mib" 'BEGIN { printf "%+.2f", now - base }')"
    vmmap_physical_vs_prev_mib="$(awk -v now="$vmmap_physical_mib" -v prev="$vmmap_prev_physical_mib" 'BEGIN { printf "%+.2f", now - prev }')"
    vmmap_virtual_vs_base_mib="$(awk -v now="$vmmap_total_virtual_mib" -v base="$vmmap_base_virtual_mib" 'BEGIN { printf "%+.2f", now - base }')"
    vmmap_virtual_vs_prev_mib="$(awk -v now="$vmmap_total_virtual_mib" -v prev="$vmmap_prev_virtual_mib" 'BEGIN { printf "%+.2f", now - prev }')"
    vmmap_resident_vs_base_mib="$(awk -v now="$vmmap_total_resident_mib" -v base="$vmmap_base_resident_mib" 'BEGIN { printf "%+.2f", now - base }')"
    vmmap_resident_vs_prev_mib="$(awk -v now="$vmmap_total_resident_mib" -v prev="$vmmap_prev_resident_mib" 'BEGIN { printf "%+.2f", now - prev }')"

    vmmap_prev_physical_mib="$vmmap_physical_mib"
    vmmap_prev_virtual_mib="$vmmap_total_virtual_mib"
    vmmap_prev_resident_mib="$vmmap_total_resident_mib"
  fi

  {
    printf "### tcp\n"
    netstat -anv -p tcp \
      | filter_netstat 'CLOSE_WAIT|ESTABLISHED|\.21115 |\.21116 |\.21117 |:21115|:21116|:21117' || true
    printf "\n### udp\n"
    netstat -anv -p udp \
      | filter_netstat '\.21115 |\.21116 |\.21117 |:21115|:21116|:21117' || true
  } >"$netstat_file"

  if [[ -d "$log_dir" ]]; then
    filter_logs 'conn-monitor|receive close reason|Connection closed:|Failed to accept connection|Handshake failed|Broken pipe|Timeout|Reset by the peer|Peer close' "$log_dir" \
      | tail -n "$tail_lines" \
      >"$log_excerpt_file" || true
  else
    printf "Log directory not found: %s\n" "$log_dir" >"$log_excerpt_file"
  fi

  if (( full_vmmap_every > 0 )); then
    if (( snapshot_index % full_vmmap_every == 0 )); then
    full_vmmap_file="$out_dir/vmmap-full-${ts_slug}.txt"
    run_capture "$full_vmmap_file" vmmap "$pid"
    fi
  fi

  close_wait_count="$(count_lsof_state_rows "$lsof_focus_file" 'CLOSE_WAIT')"
  established_count="$(count_lsof_state_rows "$lsof_focus_file" 'ESTABLISHED')"
  tcp_socket_count="$(count_lsof_rows "$lsof_tcp_file")"
  tcp_ipv4_count="$(count_lsof_family_rows "$lsof_tcp_file" 'IPv4')"
  tcp_ipv6_count="$(count_lsof_family_rows "$lsof_tcp_file" 'IPv6')"
  udp_socket_count="$(count_lsof_rows "$lsof_udp_file")"
  udp_ipv4_count="$(count_lsof_family_rows "$lsof_udp_file" 'IPv4')"
  udp_ipv6_count="$(count_lsof_family_rows "$lsof_udp_file" 'IPv6')"

  printf "%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n" \
    "$ts_human" \
    "$(to_mib "$rss_kib")" \
    "$(to_mib "$vsz_kib")" \
    "$vmmap_physical_mib" \
    "$vmmap_total_virtual_mib" \
    "$vmmap_total_resident_mib" \
    "$state" \
    "$etime" \
    "$close_wait_count" \
    "$established_count" \
    "$tcp_socket_count" \
    "$tcp_ipv4_count" \
    "$tcp_ipv6_count" \
    "$udp_socket_count" \
    "$udp_ipv4_count" \
    "$udp_ipv6_count" \
    "$(basename "$vmmap_summary_file")" \
    "$(basename "$lsof_tcp_file")" \
    "$(basename "$lsof_udp_file")" \
    "$(basename "$lsof_focus_file")" \
    "$(basename "$netstat_file")" \
    "$(basename "$log_excerpt_file")" \
    >>"$summary_csv"

  if [[ -n "$vmmap_physical_mib" ]]; then
    printf "%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n" \
      "$ts_human" \
      "$vmmap_physical_mib" \
      "$vmmap_total_virtual_mib" \
      "$vmmap_total_resident_mib" \
      "$vmmap_physical_vs_base_mib" \
      "$vmmap_physical_vs_prev_mib" \
      "$vmmap_virtual_vs_base_mib" \
      "$vmmap_virtual_vs_prev_mib" \
      "$vmmap_resident_vs_base_mib" \
      "$vmmap_resident_vs_prev_mib" \
      "$(basename "$vmmap_summary_file")" \
      >>"$vmmap_memory_csv"
  fi

  printf "[%s] rss=%s MiB vsz=%s MiB vmmap_physical=%s MiB vmmap_virtual=%s MiB state=%s etime=%s close_wait=%s established=%s tcp=%s(v4=%s,v6=%s) udp=%s(v4=%s,v6=%s) snapshot=%s\n" \
    "$ts_human" \
    "$(to_mib "$rss_kib")" \
    "$(to_mib "$vsz_kib")" \
    "${vmmap_physical_mib:-n/a}" \
    "${vmmap_total_virtual_mib:-n/a}" \
    "$state" \
    "$etime" \
    "$close_wait_count" \
    "$established_count" \
    "$tcp_socket_count" \
    "$tcp_ipv4_count" \
    "$tcp_ipv6_count" \
    "$udp_socket_count" \
    "$udp_ipv4_count" \
    "$udp_ipv6_count" \
    "$snapshot_index"

  sleep "$interval"
done
