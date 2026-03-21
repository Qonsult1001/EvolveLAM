#!/bin/bash
# scripts/evolve-claude.sh — Run /evolve via Claude Code CLI on a schedule.
#
# Usage:
#   ./scripts/evolve-claude.sh              # Run once
#   ./scripts/evolve-claude.sh --loop 5m    # Run every 5 minutes
#   ./scripts/evolve-claude.sh --loop 1h    # Run every hour
#
# Environment:
#   ANTHROPIC_API_KEY  — required (used by claude CLI)

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

parse_interval() {
    local val="$1"
    local num="${val%[smhSMH]}"
    local unit="${val##*[0-9]}"
    case "$unit" in
        s|S) echo "$num" ;;
        m|M) echo $((num * 60)) ;;
        h|H) echo $((num * 3600)) ;;
        *)   echo $((num * 60)) ;;
    esac
}

run_once() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Starting /evolve cycle..."
    claude -p --dangerously-skip-permissions --verbose "/evolve" 2>&1
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Cycle complete."
}

case "${1:-}" in
    --loop)
        interval_str="${2:-5m}"
        interval_secs=$(parse_interval "$interval_str")
        echo "Evolution loop: every ${interval_str} (${interval_secs}s). Ctrl-C to stop."
        while true; do
            run_once || echo "[$(date '+%H:%M:%S')] Cycle failed, will retry next interval."
            echo "Sleeping ${interval_secs}s..."
            sleep "$interval_secs"
        done
        ;;
    --help|-h)
        echo "Usage: $0 [--loop INTERVAL]"
        echo ""
        echo "  $0              Run /evolve once"
        echo "  $0 --loop 5m   Run every 5 minutes"
        echo "  $0 --loop 1h   Run every hour"
        ;;
    "")
        run_once
        ;;
    *)
        echo "Unknown option: $1. Use --help."
        exit 1
        ;;
esac
