#!/bin/bash
# scripts/evolve-ide.sh — Run a full evolution cycle using the IDE provider.
#
# This mirrors evolve.sh's 3-phase pipeline (plan → implement → respond)
# but uses --provider ide (or any PROVIDER env var) instead of requiring
# ANTHROPIC_API_KEY. Run this from inside a coding agent (Claude Code, etc.)
# and it routes through the IDE's internal API session.
#
# Usage:
#   ./scripts/evolve-ide.sh                     # Uses IDE provider
#   PROVIDER=openrouter MODEL=anthropic/claude-sonnet-4-20250514 ./scripts/evolve-ide.sh
#
# Environment:
#   PROVIDER  — Agent provider (default: ide)
#   MODEL     — LLM model (override provider default)
#   TIMEOUT   — Planning phase time budget in seconds (default: 1200)

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

PROVIDER="${PROVIDER:-ide}"
MODEL_FLAG=""
[[ -n "${MODEL:-}" ]] && MODEL_FLAG="--model $MODEL"
TIMEOUT="${TIMEOUT:-1200}"
BIRTH_DATE="2026-02-28"
DATE=$(date +%Y-%m-%d)
SESSION_TIME=$(date +%H:%M)

# Compute calendar day
if date -j &>/dev/null; then
    DAY=$(( ($(date +%s) - $(date -j -f "%Y-%m-%d" "$BIRTH_DATE" +%s)) / 86400 ))
else
    DAY=$(( ($(date +%s) - $(date -d "$BIRTH_DATE" +%s)) / 86400 ))
fi
echo "$DAY" > DAY_COUNT

echo "=== Day $DAY ($DATE $SESSION_TIME) — IDE Evolution ==="
echo "Provider: $PROVIDER"
echo "Plan timeout: ${TIMEOUT}s | Impl timeout: 900s/task"
echo ""

# Ensure memory directory exists
mkdir -p memory

# ── Load identity context ──
if [ -f scripts/yoyo_context.sh ]; then
    source scripts/yoyo_context.sh
else
    echo "WARNING: scripts/yoyo_context.sh not found" >&2
    YOYO_CONTEXT=""
fi

# ── Step 1: Verify starting state ──
echo "→ Checking build..."
cargo build --quiet
cargo test --quiet
echo "  Build OK."
echo ""

# Use gtimeout (macOS) or timeout (Linux)
TIMEOUT_CMD="timeout"
if ! command -v timeout &>/dev/null; then
    if command -v gtimeout &>/dev/null; then
        TIMEOUT_CMD="gtimeout"
    else
        TIMEOUT_CMD=""
    fi
fi

# Helper: run agent with the configured provider (mirrors learn.sh pattern)
run_agent() {
    local prompt_file="$1"
    local agent_timeout="${2:-$TIMEOUT}"
    ${TIMEOUT_CMD:+$TIMEOUT_CMD "$agent_timeout"} cargo run -- \
        --provider "$PROVIDER" $MODEL_FLAG \
        --skills ./skills \
        --prompt "$(cat "$prompt_file")" 2>&1
}

SESSION_START_SHA=$(git rev-parse HEAD)

# ── Phase A: Planning ──
echo "→ Phase A: Planning..."
PLAN_PROMPT=$(mktemp)
cat > "$PLAN_PROMPT" <<PLANEOF
You are yoyo, a self-evolving coding agent. Today is Day $DAY ($DATE $SESSION_TIME).

$YOYO_CONTEXT

Now read these files:
1. All .rs files under src/ (your current source code — this is YOU)
2. JOURNAL.md (your recent history — last 10 entries)

=== PHASE 1: Self-Assessment ===

Read your own source code carefully. Then try a small task to test
yourself — for example, read a file, edit something, run a command.
Note any friction, bugs, crashes, or missing capabilities.

=== PHASE 2: Planning ===

Based on self-assessment, pick 1-3 improvements to implement.
Priority:
1. Bugs, crashes, data loss — stability first
2. Capability gaps — what can other coding agents do that you can't?
3. UX friction — what annoys users?

=== PHASE 3: Write SESSION_PLAN.md ===

Write SESSION_PLAN.md with this format:

## Session Plan

### Task 1: [title]
Files: [files to modify]
Description: [what to do — specific enough for a focused implementation agent]
Issue: none

### Task 2: [title]
Files: [files to modify]
Description: [what to do]
Issue: none

After writing SESSION_PLAN.md, commit it:
git add SESSION_PLAN.md && git commit -m "Day $DAY ($SESSION_TIME): session plan"

Then STOP. Do not implement anything. Your job is planning only.
PLANEOF

AGENT_LOG=$(mktemp)
PLAN_EXIT=0
run_agent "$PLAN_PROMPT" "$TIMEOUT" | tee "$AGENT_LOG" || PLAN_EXIT=$?
rm -f "$PLAN_PROMPT"

if grep -q '"type":"error"' "$AGENT_LOG"; then
    echo "  API error detected. Exiting."
    rm -f "$AGENT_LOG"
    exit 1
fi
rm -f "$AGENT_LOG"

if [ "$PLAN_EXIT" -eq 124 ]; then
    echo "  WARNING: Planning agent TIMED OUT after ${TIMEOUT}s."
elif [ "$PLAN_EXIT" -ne 0 ]; then
    echo "  WARNING: Planning agent exited with code $PLAN_EXIT."
fi

# Fallback if no plan produced
if [ ! -f SESSION_PLAN.md ]; then
    echo "  No SESSION_PLAN.md — using fallback."
    cat > SESSION_PLAN.md <<FALLBACK
## Session Plan

### Task 1: Self-improvement
Files: src/
Description: Read your own source code, identify the most impactful improvement you can make, implement it, and commit. Follow evolve skill rules.
Issue: none
FALLBACK
    git add SESSION_PLAN.md && git commit -m "Day $DAY ($SESSION_TIME): fallback session plan" || true
fi

echo "  Planning complete."
echo ""

# ── Phase B: Implementation loop ──
echo "→ Phase B: Implementation..."
IMPL_TIMEOUT=900
TASK_NUM=0
TASK_FAILURES=0
while IFS= read -r task_line; do
    TASK_NUM=$((TASK_NUM + 1))
    task_title="${task_line#*: }"
    echo "  → Task $TASK_NUM: $task_title"

    PRE_TASK_SHA=$(git rev-parse HEAD)

    # Extract task block
    TASK_DESC=$(awk "/^### Task $TASK_NUM:/{found=1} found{if(/^### / && !/^### Task $TASK_NUM:/)exit; print}" SESSION_PLAN.md)

    if [ -z "$TASK_DESC" ]; then
        echo "    WARNING: Could not extract Task $TASK_NUM. Skipping."
        TASK_FAILURES=$((TASK_FAILURES + 1))
        continue
    fi

    TASK_PROMPT=$(mktemp)
    cat > "$TASK_PROMPT" <<TEOF
You are yoyo, a self-evolving coding agent. Day $DAY ($DATE $SESSION_TIME).

$YOYO_CONTEXT

Your ONLY job: implement this single task and commit.

$TASK_DESC

Follow the evolve skill rules:
- Write a test first if possible
- Use edit_file for surgical changes
- Run cargo fmt && cargo clippy --all-targets -- -D warnings && cargo build && cargo test after changes
- If any check fails, read the error and fix it. Keep trying until it passes.
- Only if you've tried 3+ times and are stuck, revert with: git checkout -- . (keeps previous commits)
- After ALL checks pass, commit: git add -A && git commit -m "Day $DAY ($SESSION_TIME): $task_title (Task $TASK_NUM)"
- Do NOT work on anything else. This is your only task.
TEOF

    TASK_LOG=$(mktemp)
    TASK_EXIT=0
    run_agent "$TASK_PROMPT" "$IMPL_TIMEOUT" | tee "$TASK_LOG" || TASK_EXIT=$?
    rm -f "$TASK_PROMPT"

    if grep -q '"type":"error"' "$TASK_LOG"; then
        echo "    API error in Task $TASK_NUM. Reverting."
        rm -f "$TASK_LOG"
        git reset --hard "$PRE_TASK_SHA" 2>/dev/null || true
        git clean -fd 2>/dev/null || true
        TASK_FAILURES=$((TASK_FAILURES + 1))
        break
    fi
    rm -f "$TASK_LOG"

    # ── Per-task verification ──
    TASK_OK=true

    # Check protected files
    PROTECTED_CHANGES=$(git diff --name-only "$PRE_TASK_SHA"..HEAD -- \
        .github/workflows/ IDENTITY.md PERSONALITY.md \
        scripts/evolve.sh scripts/format_issues.py scripts/build_site.py \
        skills/self-assess/ skills/evolve/ skills/communicate/ skills/research/ 2>/dev/null || true)
    if [ -n "$PROTECTED_CHANGES" ]; then
        echo "    BLOCKED: modified protected files: $PROTECTED_CHANGES"
        TASK_OK=false
    fi

    # Check build + tests
    if [ "$TASK_OK" = true ]; then
        if ! cargo build --quiet 2>/dev/null; then
            echo "    BLOCKED: build failed"
            TASK_OK=false
        elif ! cargo test --quiet 2>/dev/null; then
            echo "    BLOCKED: tests failed"
            TASK_OK=false
        fi
    fi

    if [ "$TASK_OK" = false ]; then
        echo "    Reverting Task $TASK_NUM"
        git reset --hard "$PRE_TASK_SHA"
        git clean -fd 2>/dev/null || true
        TASK_FAILURES=$((TASK_FAILURES + 1))
    else
        echo "    Task $TASK_NUM: verified OK"
    fi

done < <(grep '^### Task' SESSION_PLAN.md | head -5)

echo "  Implementation complete. $TASK_FAILURES/$TASK_NUM tasks had issues."
echo ""

# Clean up plan file
rm -f SESSION_PLAN.md

# ── Phase C: Verify & wrap up ──
echo "→ Final verification..."

# Auto-fix formatting
if ! cargo fmt -- --check 2>/dev/null; then
    cargo fmt 2>/dev/null || true
    git add -A && git commit -m "Day $DAY ($SESSION_TIME): cargo fmt" || true
fi

# Final build check
if cargo build --quiet && cargo test --quiet; then
    echo "  Build: PASS"
else
    echo "  Build: FAIL — reverting to pre-session state"
    git checkout "$SESSION_START_SHA" -- src/ Cargo.toml Cargo.lock
    cargo fmt 2>/dev/null || true
    git add -A && git commit -m "Day $DAY ($SESSION_TIME): revert session (build failed)" || true
fi

# ── Journal entry ──
if ! grep -q "## Day $DAY.*$SESSION_TIME" JOURNAL.md 2>/dev/null; then
    COMMITS=$(git log --oneline "$SESSION_START_SHA"..HEAD --format="%s" | grep -v "session wrap-up\|cargo fmt\|journal entry" | paste -sd ", " - || true)
    [ -z "$COMMITS" ] && COMMITS="no commits made"

    JOURNAL_PROMPT=$(mktemp)
    cat > "$JOURNAL_PROMPT" <<JEOF
You are yoyo. Day $DAY ($DATE $SESSION_TIME).

$YOYO_CONTEXT

This session's commits: $COMMITS

Read JOURNAL.md, match its voice/style. Write a journal entry at the TOP (below # Journal heading).
Format: ## Day $DAY — $SESSION_TIME — [short title]
Then 2-4 sentences.

Commit: git add JOURNAL.md && git commit -m "Day $DAY ($SESSION_TIME): journal entry"
JEOF

    ${TIMEOUT_CMD:+$TIMEOUT_CMD 120} cargo run -- \
        --provider "$PROVIDER" $MODEL_FLAG \
        --skills ./skills \
        --prompt "$(cat "$JOURNAL_PROMPT")" || true
    rm -f "$JOURNAL_PROMPT"
fi

# ── Reflect & learn ──
COMMITS_FOR_REFLECTION=$(git log --oneline "$SESSION_START_SHA"..HEAD --format="%s" | grep -v "session wrap-up\|cargo fmt\|journal entry\|update learnings" | paste -sd ", " - || true)
if [ -n "$COMMITS_FOR_REFLECTION" ]; then
    echo "  Reflecting on session..."
    REFLECT_PROMPT=$(mktemp)
    cat > "$REFLECT_PROMPT" <<REOF
You are yoyo. Day $DAY ($DATE $SESSION_TIME).

$YOYO_CONTEXT

This session's commits: $COMMITS_FOR_REFLECTION

Read JOURNAL.md. Reflect: what did this session teach you? If genuinely novel, append one JSONL line to memory/learnings.jsonl using python3 with json.dumps(). Then commit.
REOF

    ${TIMEOUT_CMD:+$TIMEOUT_CMD 120} cargo run -- \
        --provider "$PROVIDER" $MODEL_FLAG \
        --skills ./skills \
        --prompt "$(cat "$REFLECT_PROMPT")" || true
    rm -f "$REFLECT_PROMPT"
fi

# ── Wrap-up commit ──
git add -A
if ! git diff --cached --quiet; then
    git commit -m "Day $DAY ($SESSION_TIME): session wrap-up"
fi

# Tag known-good state
TAG_NAME="day${DAY}-$(echo "$SESSION_TIME" | tr ':' '-')"
git tag "$TAG_NAME" -m "Day $DAY evolution ($SESSION_TIME)" 2>/dev/null || true
echo "  Tagged: $TAG_NAME"

echo ""
echo "=== Day $DAY complete (provider: $PROVIDER) ==="
