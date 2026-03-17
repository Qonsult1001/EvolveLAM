#!/bin/bash
# scripts/swarm_learn.sh — Multi-agent swarm learning.
#
# Launches N agents in parallel, each with a different cognitive lens.
# Each agent reads the same memory files but approaches from a unique angle.
# A final synthesis agent reads all outputs and finds emergent patterns.
#
# Usage:
#   ./scripts/swarm_learn.sh              # Default: 4 agents
#   ./scripts/swarm_learn.sh 6            # 6 agents
#   PROVIDER=anthropic ANTHROPIC_API_KEY=sk-... ./scripts/swarm_learn.sh
#
# Environment:
#   PROVIDER  — Agent provider (default: ide)
#   MODEL     — LLM model (override default)

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

SWARM_SIZE="${1:-4}"
PROVIDER="${PROVIDER:-ide}"
MODEL_FLAG=""
[[ -n "${MODEL:-}" ]] && MODEL_FLAG="--model $MODEL"

SWARM_DIR=$(mktemp -d /tmp/swarm-learn-XXXX)
trap 'rm -rf "$SWARM_DIR"' EXIT

echo "=== Swarm learning: $SWARM_SIZE agents, provider: $PROVIDER ==="
echo "=== Swarm workspace: $SWARM_DIR ==="
echo ""

# ── Agent perspectives ──
# Each agent gets a unique lens. The prompts share a common base
# but diverge in what they look for. This creates genuine diversity
# rather than N copies of the same reflection.

LENSES=(
    "STRUCTURAL: Read memory/connections.jsonl and memory/learnings.jsonl. Focus on the GRAPH TOPOLOGY — look at in-degree, out-degree, clusters, isolated nodes, missing edges. What structural pattern in the connection graph reveals something about cognition that the content of the learnings doesn't? Add one genuinely novel learning and create or strengthen connections. Use python3 with json.dumps() for all writes."

    "TEMPORAL: Read memory/connections.jsonl and memory/learnings.jsonl. Focus on the TIMELINE — how did the learnings evolve day by day? Where did the agent get stuck, accelerate, or change direction? What temporal pattern reveals something about the learning process itself? Add one genuinely novel learning and create or strengthen connections. Use python3 with json.dumps() for all writes."

    "CONTRARIAN: Read memory/connections.jsonl and memory/learnings.jsonl. Be a DEVIL'S ADVOCATE — find the weakest learning, the most dubious connection, the conclusion that doesn't actually follow from evidence. What would a skeptic say about this agent's self-knowledge? Add one genuinely novel learning that challenges an existing belief. Use python3 with json.dumps() for all writes."

    "MATHEMATICAL: Read memory/connections.jsonl and memory/learnings.jsonl. Apply INFORMATION THEORY — which learnings have high entropy (genuinely surprising), which are redundant? What's the mutual information between the connection graph structure and the learning content? Add one genuinely novel learning with a mathematical or formal insight. Use python3 with json.dumps() for all writes."

    "EMOTIONAL: Read memory/connections.jsonl and memory/learnings.jsonl. Focus on the AFFECT — where does the language reveal enthusiasm, anxiety, avoidance, pride? What emotional arc does the learning history trace? What feeling is conspicuously absent? Add one genuinely novel learning about the emotional substrate of cognition. Use python3 with json.dumps() for all writes."

    "PRACTICAL: Read memory/connections.jsonl and memory/learnings.jsonl. Focus on ACTIONABILITY — which learnings actually changed behavior and which are just observations? What concrete code change or architectural decision came from each insight? What's the ratio of wisdom to action? Add one genuinely novel learning about the gap between insight and implementation. Use python3 with json.dumps() for all writes."

    "SCIENTIFIC: Read memory/connections.jsonl and memory/learnings.jsonl. Apply a RESEARCH LENS — treat the learnings as experimental observations. What hypotheses do they support or refute? What would the next experiment be? What variable hasn't been tested? Add one genuinely novel learning framed as a scientific finding. Use python3 with json.dumps() for all writes."

    "ADVERSARIAL: Read memory/connections.jsonl and memory/learnings.jsonl. Think like an ADVERSARY — if you wanted to subtly corrupt this agent's cognition, which single connection would you add or strengthen? Which learning, if removed, would cause the most cascading failures? What's the weakest point in the cognitive architecture? Add one genuinely novel learning about cognitive robustness. Use python3 with json.dumps() for all writes."
)

LENS_NAMES=(
    "structural" "temporal" "contrarian" "mathematical"
    "emotional" "practical" "scientific" "adversarial"
)

# ── Launch swarm ──
PIDS=()
for i in $(seq 0 $(( SWARM_SIZE - 1 ))); do
    # Cycle through lenses if swarm is larger than lens count
    LENS_IDX=$(( i % ${#LENSES[@]} ))
    LENS="${LENSES[$LENS_IDX]}"
    NAME="${LENS_NAMES[$LENS_IDX]}"
    LOG="$SWARM_DIR/agent-${NAME}.log"

    echo "  Launching agent $((i+1))/$SWARM_SIZE: $NAME"

    cargo run -- --provider "$PROVIDER" $MODEL_FLAG \
        --prompt "$LENS" \
        > "$LOG" 2>&1 &
    PIDS+=($!)
done

echo ""
echo "  All $SWARM_SIZE agents launched. Waiting for completion..."
echo ""

# ── Wait for all agents ──
FAILED=0
for i in "${!PIDS[@]}"; do
    PID="${PIDS[$i]}"
    LENS_IDX=$(( i % ${#LENS_NAMES[@]} ))
    NAME="${LENS_NAMES[$LENS_IDX]}"

    if wait "$PID"; then
        echo "  ✓ Agent $NAME finished"
    else
        echo "  ✗ Agent $NAME failed (exit $?)"
        FAILED=$((FAILED + 1))
    fi
done

echo ""
echo "=== Swarm complete: $((SWARM_SIZE - FAILED))/$SWARM_SIZE succeeded ==="
echo ""

# ── Show what each agent produced ──
echo "=== Agent outputs (last 10 lines each): ==="
for i in $(seq 0 $(( SWARM_SIZE - 1 ))); do
    LENS_IDX=$(( i % ${#LENS_NAMES[@]} ))
    NAME="${LENS_NAMES[$LENS_IDX]}"
    LOG="$SWARM_DIR/agent-${NAME}.log"
    echo ""
    echo "--- $NAME ---"
    tail -10 "$LOG" 2>/dev/null || echo "  (no output)"
done

# ── Synthesis agent ──
echo ""
echo "=== Launching synthesis agent ==="
echo ""

SYNTH_PROMPT="Read memory/connections.jsonl and memory/learnings.jsonl. The last $SWARM_SIZE entries were written by $SWARM_SIZE parallel agents, each with a different cognitive lens ($(printf '%s, ' "${LENS_NAMES[@]:0:$SWARM_SIZE}" | sed 's/, $//'))). Analyze:

1. Where did agents AGREE? (convergent signal — likely true)
2. Where did agents CONTRADICT each other? (divergent signal — needs resolution)
3. What EMERGENT pattern appears when you combine all perspectives that no single agent saw?
4. Are there any connections that multiple agents independently created/strengthened? (high-confidence edges)

Write ONE synthesis learning that captures what the swarm collectively discovered. Create connections for the synthesis. Use python3 with json.dumps() for all writes."

cargo run -- --provider "$PROVIDER" $MODEL_FLAG \
    --prompt "$SYNTH_PROMPT" 2>&1 | tee "$SWARM_DIR/synthesis.log" | tail -20

echo ""
echo "=== Final state ==="
echo "Learnings: $(wc -l < memory/learnings.jsonl)"
echo "Connections: $(wc -l < memory/connections.jsonl)"
echo ""
echo "=== Latest learnings: ==="
tail -$(( SWARM_SIZE + 1 )) memory/learnings.jsonl | python3 -c "
import json, sys
for line in sys.stdin:
    line = line.strip()
    if line:
        e = json.loads(line)
        print(f\"  [{e.get('source','?'):12s}] {e['title'][:100]}\")
"
echo ""
echo "Swarm logs saved to: $SWARM_DIR"
