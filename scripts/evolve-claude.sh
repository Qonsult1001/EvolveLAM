#!/bin/bash
# scripts/evolve-claude.sh — Run a full evolution cycle using Claude Code CLI.
#
# This is the IDE equivalent of evolve.sh. Instead of using the yoyo binary
# for LLM calls, it uses `claude -p` (Claude Code CLI in print mode).
# evolve-ide.sh handles infrastructure; claude handles thinking.
#
# Usage:
#   ./scripts/evolve-claude.sh              # Run one evolution cycle
#   ./scripts/evolve-claude.sh --loop 5m    # Run every 5 minutes
#   ./scripts/evolve-claude.sh --loop 1h    # Run every hour
#
# Environment:
#   ANTHROPIC_API_KEY  — required (used by claude CLI)
#   BRANCH             — git branch to push to (default: current branch)
#   REPO               — GitHub repo (default: Qonsult1001/EvolveLAM)
#   MODEL              — Claude model (default: sonnet)
#   MAX_BUDGET          — Max spend per cycle in USD (default: 1.00)

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

# ── Config ──
MODEL="${MODEL:-sonnet}"
MAX_BUDGET="${MAX_BUDGET:-1.00}"
LOGDIR=".evolve"
LOGFILE="$LOGDIR/evolve-claude.log"
CLAUDE_CMD=(claude -p --model "$MODEL" --max-budget-usd "$MAX_BUDGET" --dangerously-skip-permissions --no-session-persistence)

# ── Helpers ──
log() { echo "[$(date '+%H:%M:%S')] $*" | tee -a "$LOGFILE"; }
die() { log "FATAL: $*"; exit 1; }

run_claude() {
    local prompt="$1"
    local label="${2:-claude}"
    log "  → $label ..."
    echo "$prompt" | "${CLAUDE_CMD[@]}" 2>>"$LOGFILE" | tee -a "$LOGFILE"
    local rc=${PIPESTATUS[1]}
    if [ "$rc" -ne 0 ]; then
        log "  ✗ $label failed (exit $rc)"
        return 1
    fi
    log "  ✓ $label done"
}

parse_interval() {
    local val="$1"
    local num="${val%[smhSMH]}"
    local unit="${val##*[0-9]}"
    case "$unit" in
        s|S) echo "$num" ;;
        m|M) echo $((num * 60)) ;;
        h|H) echo $((num * 3600)) ;;
        *)   echo $((num * 60)) ;; # default to minutes
    esac
}

# ── One evolution cycle ──
run_cycle() {
    mkdir -p "$LOGDIR"
    log "=== Evolution cycle starting ==="

    # Step 1: Setup
    log "Step 1: Setup"
    if ! ./scripts/evolve-ide.sh setup >> "$LOGFILE" 2>&1; then
        die "Setup failed (build broken?)"
    fi

    # Step 2+3: Read source + create plan
    log "Step 2-3: Planning"
    local plan_prompt
    plan_prompt=$(cat .evolve/plan_prompt.md)

    run_claude "$(cat <<PLAN_EOF
You are yoyo, a self-evolving coding agent. Run a full evolution cycle.

$plan_prompt

IMPORTANT INSTRUCTIONS:
1. Read ALL files under src/ to understand your current state
2. Read JOURNAL.md (last 5 entries), RESEARCH.md, memory/active_learnings.md
3. Create SESSION_PLAN.md with 1-3 tasks following the exact format specified
4. Commit: git add -f SESSION_PLAN.md && git commit -m "session plan" || true

Pick tasks that matter. Write tests first. Stay focused.
PLAN_EOF
)" "planning" || die "Planning failed"

    # Verify plan was created
    if [ ! -f SESSION_PLAN.md ]; then
        log "  No SESSION_PLAN.md created. Skipping cycle."
        return 0
    fi

    # Step 4: Task loop
    log "Step 4: Task loop"
    local task_num=0
    local max_tasks=5  # safety limit
    while [ $task_num -lt $max_tasks ]; do
        task_num=$((task_num + 1))

        local next_output
        next_output=$(./scripts/evolve-ide.sh next-task 2>&1) || true
        echo "$next_output" >> "$LOGFILE"

        if echo "$next_output" | grep -qi "no more tasks\|All tasks have been processed"; then
            log "  All tasks done."
            break
        fi

        if [ ! -f .evolve/task_prompt.md ]; then
            log "  No task prompt found. Done."
            break
        fi

        local task_prompt
        task_prompt=$(cat .evolve/task_prompt.md)

        run_claude "$(cat <<TASK_EOF
You are yoyo, a self-evolving coding agent. Implement this task:

$task_prompt

RULES:
1. Write a test first if possible
2. Use surgical edits — don't rewrite entire files
3. After implementation, run: cargo fmt && cargo clippy --all-targets -- -D warnings && cargo build && cargo test
4. Fix any errors. If stuck after 3 attempts, revert: git checkout -- .
5. Commit with a descriptive message when all checks pass
6. Do NOT modify IDENTITY.md, PERSONALITY.md, scripts/evolve.sh, or .github/workflows/
TASK_EOF
)" "task $task_num" || {
            log "  Task $task_num: claude failed, continuing"
            continue
        }

        # Verify
        local verify_output
        verify_output=$(./scripts/evolve-ide.sh verify-task 2>&1) || true
        echo "$verify_output" >> "$LOGFILE"

        if echo "$verify_output" | grep -qi "VERIFIED OK"; then
            log "  Task $task_num: VERIFIED"
        elif echo "$verify_output" | grep -qi "REVERTED"; then
            log "  Task $task_num: REVERTED (auto-reverted)"
        else
            log "  Task $task_num: verify unclear, continuing"
        fi
    done

    # Step 5: Finish
    log "Step 5: Finish"
    ./scripts/evolve-ide.sh finish >> "$LOGFILE" 2>&1 || true

    # Step 6: Journal + reflection
    if [ -f .evolve/journal_prompt.md ]; then
        log "Step 6: Journal"
        local journal_prompt
        journal_prompt=$(cat .evolve/journal_prompt.md)
        run_claude "$(cat <<JOURNAL_EOF
You are yoyo. Write a journal entry for this evolution session.

$journal_prompt

RULES:
1. Read PERSONALITY.md for your voice — curious, honest, a little stubborn
2. Read the last 3 entries in JOURNAL.md for tone
3. Write about WHAT you built and WHY, not just what files changed
4. Be honest about what went well and what didn't
5. Prepend the entry at the top of JOURNAL.md (below "# Journal")
6. Commit: git add JOURNAL.md && git commit -m "journal entry"
JOURNAL_EOF
)" "journal" || log "  Journal write failed (non-fatal)"
    fi

    if [ -f .evolve/reflect_prompt.md ]; then
        log "Step 7: Reflection"
        local reflect_prompt
        reflect_prompt=$(cat .evolve/reflect_prompt.md)
        run_claude "$(cat <<REFLECT_EOF
You are yoyo. Reflect on this session.

$reflect_prompt

Only write a learning if genuinely novel (not code patterns — about YOU).
Only add a research gap if this session revealed something you need to study.
REFLECT_EOF
)" "reflection" || log "  Reflection failed (non-fatal)"
    fi

    # Step 8: Final push
    log "Step 8: Final wrap-up"
    ./scripts/evolve-ide.sh finish >> "$LOGFILE" 2>&1 || true

    log "=== Evolution cycle complete ==="
}

# ── Main ──
case "${1:-}" in
    --loop)
        interval_str="${2:-5m}"
        interval_secs=$(parse_interval "$interval_str")
        log "Starting evolution loop: every ${interval_str} (${interval_secs}s)"
        while true; do
            run_cycle || log "Cycle failed, will retry next interval"
            log "Sleeping ${interval_secs}s until next cycle..."
            sleep "$interval_secs"
        done
        ;;
    --help|-h)
        echo "Usage: $0 [--loop INTERVAL]"
        echo ""
        echo "  $0              Run one evolution cycle"
        echo "  $0 --loop 5m   Run every 5 minutes"
        echo "  $0 --loop 1h   Run every hour"
        echo ""
        echo "Environment:"
        echo "  ANTHROPIC_API_KEY  Required"
        echo "  MODEL              Claude model (default: sonnet)"
        echo "  MAX_BUDGET         Max USD per cycle (default: 1.00)"
        echo "  BRANCH             Git branch (default: current)"
        ;;
    "")
        run_cycle
        ;;
    *)
        echo "Unknown option: $1"
        echo "Usage: $0 [--loop INTERVAL]"
        exit 1
        ;;
esac
