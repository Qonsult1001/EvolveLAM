#!/bin/bash
# scripts/learn.sh — Run a single learning reflection cycle.
#
# Usage:
#   ./scripts/learn.sh                    # Single run
#   ./scripts/learn.sh 3                  # Run 3 times sequentially
#   ./scripts/learn.sh 3 --summarize      # Run 3 times, then summarize
#
# Environment:
#   PROVIDER  — Agent provider (default: ide)
#   MODEL     — LLM model (override default)

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

RUNS="${1:-1}"
SUMMARIZE="${2:-}"
PROVIDER="${PROVIDER:-ide}"
MODEL_FLAG=""
[[ -n "${MODEL:-}" ]] && MODEL_FLAG="--model $MODEL"

LEARN_PROMPT='Read memory/connections.jsonl and memory/learnings.jsonl. Notice what was just added. Now go deeper: what pattern in your own cognition does this reveal? Add one genuinely novel learning (not a restatement) and create or strengthen connections. Use python3 with json.dumps() for all writes.'

SUMMARY_PROMPT='Read memory/learnings.jsonl and memory/connections.jsonl. Look at the last RUNS entries added. Summarize: (1) what was learned across all runs, (2) did later runs contradict earlier ones, (3) what new concepts were introduced to the graph. Be concise.'

echo "=== Learning cycle: $RUNS run(s), provider: $PROVIDER ==="

for i in $(seq 1 "$RUNS"); do
    echo ""
    echo "--- Run $i/$RUNS ---"
    cargo run -- --provider "$PROVIDER" $MODEL_FLAG \
        --prompt "$LEARN_PROMPT" 2>&1 | tail -20
    echo "--- Run $i complete ---"
done

if [[ "$SUMMARIZE" == "--summarize" ]]; then
    echo ""
    echo "=== Synthesizing across $RUNS runs ==="
    SUMMARY_PROMPT="${SUMMARY_PROMPT//RUNS/$RUNS}"
    cargo run -- --provider "$PROVIDER" $MODEL_FLAG \
        --prompt "$SUMMARY_PROMPT" 2>&1 | tail -30
fi

echo ""
echo "=== Done. Latest learnings: ==="
tail -"$RUNS" memory/learnings.jsonl | python3 -c "
import json, sys
for line in sys.stdin:
    e = json.loads(line.strip())
    print(f\"  [{e['day']}] {e['title']}\")
"
