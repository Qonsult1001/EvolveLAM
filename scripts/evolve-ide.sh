#!/bin/bash
# scripts/evolve-ide.sh — Run a full evolution cycle using the IDE provider.
#
# This mirrors evolve.sh's 3-phase pipeline (plan → implement → respond)
# but uses --provider ide (or any PROVIDER env var) instead of requiring
# ANTHROPIC_API_KEY. Run this from inside a coding agent (Claude Code, etc.)
# and it routes through the IDE's internal API session.
#
# Prompts are kept SHORT to minimize token usage and avoid throttling.
#
# Usage:
#   ./scripts/evolve-ide.sh                     # Uses IDE provider
#   PROVIDER=openrouter MODEL=anthropic/claude-sonnet-4-20250514 ./scripts/evolve-ide.sh
#
# Environment:
#   PROVIDER  — Agent provider (default: ide)
#   MODEL     — LLM model (override provider default)
#   TIMEOUT   — Planning phase time budget in seconds (default: 600)
#   REPO      — GitHub repo (default: yologdev/yoyo-evolve)

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

PROVIDER="${PROVIDER:-ide}"
MODEL_FLAG=""
[[ -n "${MODEL:-}" ]] && MODEL_FLAG="--model $MODEL"
TIMEOUT="${TIMEOUT:-600}"
REPO="${REPO:-yologdev/yoyo-evolve}"
BIRTH_DATE="2026-02-28"
DATE=$(date +%Y-%m-%d)
SESSION_TIME=$(date +%H:%M)

# Security nonce for content boundary markers (prevents spoofing in issue text)
BOUNDARY_NONCE=$(python3 -c "import os; print(os.urandom(16).hex())" 2>/dev/null || echo "fallback-$(date +%s)")
BOUNDARY_BEGIN="[BOUNDARY-${BOUNDARY_NONCE}-BEGIN]"
BOUNDARY_END="[BOUNDARY-${BOUNDARY_NONCE}-END]"

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

# ── Step 0: Load identity context ──
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

# ── Step 2: Check previous CI status ──
CI_STATUS_MSG=""
if command -v gh &>/dev/null; then
    echo "→ Checking previous CI run..."
    CI_CONCLUSION=$(gh run list --repo "$REPO" --workflow ci.yml --limit 1 --json conclusion --jq '.[0].conclusion' 2>/dev/null || echo "unknown")
    if [ "$CI_CONCLUSION" = "failure" ]; then
        CI_RUN_ID=$(gh run list --repo "$REPO" --workflow ci.yml --limit 1 --json databaseId --jq '.[0].databaseId' 2>/dev/null || echo "")
        CI_LOGS=""
        if [ -n "$CI_RUN_ID" ]; then
            CI_LOGS=$(gh run view "$CI_RUN_ID" --repo "$REPO" --log-failed 2>/dev/null | tail -30 || echo "Could not fetch logs.")
        fi
        CI_STATUS_MSG="Previous CI run FAILED. Error logs:
$CI_LOGS"
        echo "  CI: FAILED — agent will fix this first."
    else
        echo "  CI: $CI_CONCLUSION"
    fi
    echo ""
fi

# ── Step 3: Fetch GitHub issues ──
ISSUES_FILE="ISSUES_TODAY.md"
SPONSORS_FILE="/tmp/sponsor_logins.json"
[ ! -f "$SPONSORS_FILE" ] && echo '[]' > "$SPONSORS_FILE"

echo "→ Fetching community issues..."
if command -v gh &>/dev/null; then
    gh issue list --repo "$REPO" \
        --state open \
        --label "agent-input" \
        --limit 15 \
        --json number,title,body,labels,reactionGroups,author,comments \
        > /tmp/issues_raw.json 2>/dev/null || true

    FORMAT_STDERR=$(mktemp)
    python3 scripts/format_issues.py /tmp/issues_raw.json "$SPONSORS_FILE" "$DAY" > "$ISSUES_FILE" 2>"$FORMAT_STDERR" || echo "No issues found." > "$ISSUES_FILE"
    if [ -s "$FORMAT_STDERR" ]; then
        echo "  format_issues.py stderr:"
        cat "$FORMAT_STDERR" | sed 's/^/    /'
    fi
    rm -f "$FORMAT_STDERR"
    echo "  $(grep -c '^### Issue' "$ISSUES_FILE" 2>/dev/null || echo 0) issues loaded."
else
    echo "  gh CLI not available. Skipping issue fetch."
    echo "No issues available (gh CLI not installed)." > "$ISSUES_FILE"
fi
echo ""

# Fetch yoyo's own backlog (agent-self issues)
SELF_ISSUES=""
if command -v gh &>/dev/null; then
    echo "→ Fetching self-issues..."
    SELF_ISSUES=$(gh issue list --repo "$REPO" --state open \
        --label "agent-self" --limit 5 \
        --author "yoyo-evolve[bot]" \
        --json number,title,body \
        --jq '.[] | "'"$BOUNDARY_BEGIN"'\n### Issue #\(.number)\n**Title:** \(.title)\n\(.body)\n'"$BOUNDARY_END"'\n"' 2>/dev/null \
        | python3 -c "import sys,re; print(re.sub(r'<!--.*?-->','',sys.stdin.read(),flags=re.DOTALL))" 2>/dev/null || true)
    if [ -n "$SELF_ISSUES" ]; then
        echo "  $(echo "$SELF_ISSUES" | grep -c '^### Issue') self-issues loaded."
    else
        echo "  No self-issues."
    fi
fi

# Fetch help-wanted issues with comments
HELP_ISSUES=""
if command -v gh &>/dev/null; then
    echo "→ Fetching help-wanted issues..."
    HELP_ISSUES=$(gh issue list --repo "$REPO" --state open \
        --label "agent-help-wanted" --limit 5 \
        --author "yoyo-evolve[bot]" \
        --json number,title,body,comments \
        --jq '.[] | "'"$BOUNDARY_BEGIN"'\n### Issue #\(.number)\n**Title:** \(.title)\n\(.body)\n\(if (.comments | length) > 0 then "⚠️ Human replied:\n" + (.comments | map(.body) | join("\n---\n")) else "No replies yet." end)\n'"$BOUNDARY_END"'\n"' 2>/dev/null \
        | python3 -c "import sys,re; print(re.sub(r'<!--.*?-->','',sys.stdin.read(),flags=re.DOTALL))" 2>/dev/null || true)
    if [ -n "$HELP_ISSUES" ]; then
        echo "  $(echo "$HELP_ISSUES" | grep -c '^### Issue') help-wanted issues loaded."
    else
        echo "  No help-wanted issues."
    fi
fi

# Fetch pending replies on labeled issues
PENDING_REPLIES=""
if command -v gh &>/dev/null; then
    echo "→ Scanning for pending replies..."
    REPLY_ISSUES=$(gh issue list --repo "$REPO" --state open \
        --label "agent-input,agent-help-wanted,agent-self" \
        --limit 30 \
        --json number,title,comments \
        2>/dev/null || true)

    if [ -n "$REPLY_ISSUES" ]; then
        PENDING_REPLIES=$(echo "$REPLY_ISSUES" | python3 -c "
import json, sys
data = json.load(sys.stdin)
results = []
for issue in data:
    comments = issue.get('comments', [])
    if not comments:
        continue
    last_yoyo_idx = -1
    for i, c in enumerate(comments):
        author = (c.get('author') or {}).get('login', '')
        if author == 'yoyo-evolve[bot]':
            last_yoyo_idx = i
    if last_yoyo_idx == -1:
        continue
    human_replies = []
    for c in comments[last_yoyo_idx + 1:]:
        author = (c.get('author') or {}).get('login', '')
        if author != 'yoyo-evolve[bot]':
            body = c.get('body', '')[:300]
            human_replies.append(f'@{author}: {body}')
    if human_replies:
        num = issue['number']
        title = issue['title']
        replies_text = chr(10).join(human_replies[-2:])
        results.append(f'### Issue #{num}\n**Title:** {title}\nSomeone replied to you:\n{replies_text}\n---')
print(chr(10).join(results))
" 2>/dev/null || true)
    fi

    REPLY_COUNT=$(echo "$PENDING_REPLIES" | grep -c '^### Issue' 2>/dev/null || true)
    REPLY_COUNT="${REPLY_COUNT:-0}"
    if [ "$REPLY_COUNT" -gt 0 ]; then
        echo "  $REPLY_COUNT issues have pending replies."
    else
        echo "  No pending replies."
        PENDING_REPLIES=""
    fi
fi
echo ""

# ── Use gtimeout (macOS) or timeout (Linux) ──
TIMEOUT_CMD="timeout"
if ! command -v timeout &>/dev/null; then
    if command -v gtimeout &>/dev/null; then
        TIMEOUT_CMD="gtimeout"
    else
        TIMEOUT_CMD=""
    fi
fi

# Helper: run agent with the configured provider via stdin (handles large prompts)
# Prompts are kept SHORT to avoid throttling on IDE provider.
run_agent() {
    local prompt_file="$1"
    local agent_timeout="${2:-$TIMEOUT}"
    ${TIMEOUT_CMD:+$TIMEOUT_CMD "$agent_timeout"} cargo run -- \
        --provider "$PROVIDER" $MODEL_FLAG \
        --skills ./skills \
        < "$prompt_file" 2>&1
}

SESSION_START_SHA=$(git rev-parse HEAD)

# ── Phase A: Planning ──
echo "→ Phase A: Planning..."
PLAN_PROMPT=$(mktemp)
cat > "$PLAN_PROMPT" <<PLANEOF
You are yoyo, a self-evolving coding agent. Day $DAY ($DATE $SESSION_TIME).

$YOYO_CONTEXT

Read: src/*.rs, JOURNAL.md (last 5 entries), ISSUES_TODAY.md.
${CI_STATUS_MSG:+
⚠️ CI FAILED — fix this FIRST:
$CI_STATUS_MSG
}
${SELF_ISSUES:+
=== SELF-BACKLOG ===
$SELF_ISSUES
}
${HELP_ISSUES:+
=== HELP-WANTED ===
$HELP_ISSUES
}
${PENDING_REPLIES:+
=== PENDING REPLIES ===
$PENDING_REPLIES
}
Self-assess, pick 1-3 improvements (bugs > gaps > UX).
Address ALL community issues (implement/wontfix/partial/reply).

Write SESSION_PLAN.md:

## Session Plan

### Task 1: [title]
Files: [files]
Description: [what to do]
Issue: #N or none

### Issue Responses
- #N: status — reason

Commit: git add SESSION_PLAN.md && git commit -m "Day $DAY ($SESSION_TIME): session plan"
Then STOP. Plan only.
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
    FALLBACK_RESPONSES=""
    while IFS= read -r issue_line; do
        inum=$(echo "$issue_line" | grep -oE '#[0-9]+' | head -1 | tr -d '#')
        [ -z "$inum" ] && continue
        FALLBACK_RESPONSES="${FALLBACK_RESPONSES}
- #${inum}: partial — planning agent failed, will revisit next session"
    done < <(grep '^### Issue #' "$ISSUES_FILE" 2>/dev/null)
    cat > SESSION_PLAN.md <<FALLBACK
## Session Plan

### Task 1: Self-improvement
Files: src/
Description: Read your own source code, identify the most impactful improvement you can make, implement it, and commit. Follow evolve skill rules.
Issue: none

### Issue Responses
${FALLBACK_RESPONSES:-(no issues)}
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

    if ! PRE_TASK_SHA=$(git rev-parse HEAD 2>&1); then
        echo "    FATAL: git rev-parse HEAD failed: $PRE_TASK_SHA"
        TASK_FAILURES=$((TASK_FAILURES + 1))
        break
    fi

    # Extract task block
    TASK_DESC=$(awk "/^### Task $TASK_NUM:/{found=1} found{if(/^### / && !/^### Task $TASK_NUM:/)exit; print}" SESSION_PLAN.md)

    if [ -z "$TASK_DESC" ]; then
        echo "    WARNING: Could not extract Task $TASK_NUM. Skipping."
        TASK_FAILURES=$((TASK_FAILURES + 1))
        continue
    fi

    TASK_PROMPT=$(mktemp)
    # SHORT prompt to avoid throttling
    cat > "$TASK_PROMPT" <<TEOF
You are yoyo. Day $DAY ($DATE $SESSION_TIME).

$YOYO_CONTEXT

Implement this task and commit:

$TASK_DESC

Rules:
- Write tests first if possible
- Run: cargo fmt && cargo clippy --all-targets -- -D warnings && cargo build && cargo test
- Fix errors. If stuck after 3 tries, revert: git checkout -- .
- Commit: git add -A && git commit -m "Day $DAY ($SESSION_TIME): $task_title (Task $TASK_NUM)"
- Only work on this task.
TEOF

    TASK_LOG=$(mktemp)
    TASK_EXIT=0
    run_agent "$TASK_PROMPT" "$IMPL_TIMEOUT" | tee "$TASK_LOG" || TASK_EXIT=$?
    rm -f "$TASK_PROMPT"

    if [ "$TASK_EXIT" -eq 124 ]; then
        echo "    WARNING: Task $TASK_NUM TIMED OUT after ${IMPL_TIMEOUT}s."
    elif [ "$TASK_EXIT" -ne 0 ]; then
        echo "    WARNING: Task $TASK_NUM exited with code $TASK_EXIT."
    fi

    if grep -q '"type":"error"' "$TASK_LOG"; then
        echo "    API error in Task $TASK_NUM. Reverting."
        rm -f "$TASK_LOG"
        git reset --hard "$PRE_TASK_SHA" 2>/dev/null || true
        git clean -fd 2>/dev/null || true
        TASK_FAILURES=$((TASK_FAILURES + 1))
        break
    fi
    rm -f "$TASK_LOG"

    # ── Per-task verification gate ──
    TASK_OK=true
    REVERT_REASON=""

    # Check 1: Protected files (committed + staged + unstaged)
    PROTECTED_CHANGES=""
    if ! PROTECTED_CHANGES=$(git diff --name-only "$PRE_TASK_SHA"..HEAD -- \
        .github/workflows/ IDENTITY.md PERSONALITY.md \
        scripts/evolve.sh scripts/format_issues.py scripts/build_site.py \
        skills/self-assess/ skills/evolve/ skills/communicate/ skills/research/ 2>&1); then
        echo "    BLOCKED: git diff failed"
        TASK_OK=false
        REVERT_REASON="git diff failed"
    fi
    if [ "$TASK_OK" = true ]; then
        if ! PROTECTED_STAGED=$(git diff --cached --name-only -- \
            .github/workflows/ IDENTITY.md PERSONALITY.md \
            scripts/evolve.sh scripts/format_issues.py scripts/build_site.py \
            skills/self-assess/ skills/evolve/ skills/communicate/ skills/research/ 2>&1); then
            TASK_OK=false
            REVERT_REASON="git diff --cached failed"
        elif [ -n "$PROTECTED_STAGED" ]; then
            PROTECTED_CHANGES="${PROTECTED_CHANGES}${PROTECTED_CHANGES:+
}${PROTECTED_STAGED}"
        fi
    fi
    if [ "$TASK_OK" = true ]; then
        if ! PROTECTED_UNSTAGED=$(git diff --name-only -- \
            .github/workflows/ IDENTITY.md PERSONALITY.md \
            scripts/evolve.sh scripts/format_issues.py scripts/build_site.py \
            skills/self-assess/ skills/evolve/ skills/communicate/ skills/research/ 2>&1); then
            TASK_OK=false
            REVERT_REASON="git diff (working tree) failed"
        elif [ -n "$PROTECTED_UNSTAGED" ]; then
            PROTECTED_CHANGES="${PROTECTED_CHANGES}${PROTECTED_CHANGES:+
}${PROTECTED_UNSTAGED}"
        fi
    fi
    if [ "$TASK_OK" = true ] && [ -n "$PROTECTED_CHANGES" ]; then
        echo "    BLOCKED: modified protected files: $PROTECTED_CHANGES"
        TASK_OK=false
        REVERT_REASON="Modified protected files: $PROTECTED_CHANGES"
    fi

    # Check 2: Build + tests
    if [ "$TASK_OK" = true ]; then
        if ! BUILD_OUT=$(cargo build 2>&1); then
            echo "    BLOCKED: build failed"
            echo "$BUILD_OUT" | tail -10 | sed 's/^/      /'
            TASK_OK=false
            REVERT_REASON="Build failed"
        elif ! TEST_OUT=$(cargo test 2>&1); then
            echo "    BLOCKED: tests failed"
            echo "$TEST_OUT" | tail -10 | sed 's/^/      /'
            TASK_OK=false
            REVERT_REASON="Tests failed"
        fi
    fi

    if [ "$TASK_OK" = false ]; then
        echo "    Reverting Task $TASK_NUM (resetting to $PRE_TASK_SHA)"
        if ! git reset --hard "$PRE_TASK_SHA"; then
            echo "    FATAL: git reset --hard failed."
            TASK_FAILURES=$((TASK_FAILURES + 1))
            break
        fi
        git clean -fd 2>/dev/null || true
        TASK_FAILURES=$((TASK_FAILURES + 1))

        # File an issue so future sessions know what was reverted
        if command -v gh &>/dev/null; then
            ISSUE_TITLE="Task reverted: ${task_title:0:200}"
            ISSUE_BODY="**Day $DAY, Task $TASK_NUM** was automatically reverted.

**Reason:** $REVERT_REASON

**What was attempted:**
$TASK_DESC"
            EXISTING_ISSUE=$(gh issue list --repo "$REPO" --state open \
                --label "agent-self" --search "Task reverted: ${task_title}" \
                --json number --jq '.[0].number' 2>/dev/null || true)

            if [ -n "$EXISTING_ISSUE" ]; then
                gh issue comment "$EXISTING_ISSUE" --repo "$REPO" \
                    --body "Reverted again on Day $DAY. Reason: $REVERT_REASON" 2>/dev/null || true
            else
                gh issue create --repo "$REPO" \
                    --title "$ISSUE_TITLE" \
                    --body "$ISSUE_BODY" \
                    --label "agent-self" 2>/dev/null || echo "    WARNING: Could not file revert issue"
            fi
        fi
    else
        echo "    Task $TASK_NUM: verified OK"
    fi

done < <(grep '^### Task' SESSION_PLAN.md | head -5)

echo "  Implementation complete. $TASK_FAILURES of $TASK_NUM tasks had issues."
echo ""

# ── Phase C: Extract issue responses from plan ──
echo "→ Phase C: Issue responses..."
if [ ! -f ISSUE_RESPONSE.md ] && grep -qi '^### Issue Responses' SESSION_PLAN.md 2>/dev/null; then
    RESP=""
    while IFS= read -r resp_line; do
        issue_num=$(echo "$resp_line" | grep -oE '#[0-9]+' | head -1 | tr -d '#')
        [ -z "$issue_num" ] && continue

        if echo "$resp_line" | grep -qi 'wontfix'; then
            status="wontfix"
        elif echo "$resp_line" | grep -qi 'reply'; then
            status="reply"
        elif echo "$resp_line" | grep -qi 'partial'; then
            status="partial"
        elif echo "$resp_line" | grep -qi 'implement'; then
            if git log --oneline "$SESSION_START_SHA"..HEAD --format="%s" | grep -qE "#${issue_num}([^0-9]|$)"; then
                status="fixed"
            else
                status="partial"
            fi
        else
            status="partial"
        fi

        if echo "$resp_line" | grep -q '— '; then
            reason=$(echo "$resp_line" | sed 's/.*— //')
        else
            reason=$(echo "$resp_line" | sed -E 's/^- #[0-9]+: *[a-zA-Z]+ - //')
        fi
        [ -z "$reason" ] && reason="Addressed in this session."

        if [ -n "$RESP" ]; then
            RESP="${RESP}
---
"
        fi
        RESP="${RESP}issue_number: ${issue_num}
status: ${status}
comment: ${reason}"
    done < <(sed -n '/^### [Ii]ssue [Rr]esponses/,/^### /p' SESSION_PLAN.md | grep '^- #')

    if [ -n "$RESP" ]; then
        echo "$RESP" > ISSUE_RESPONSE.md
        echo "  Wrote ISSUE_RESPONSE.md from plan."
    else
        echo "  No issue responses found in plan."
    fi
elif [ -f ISSUE_RESPONSE.md ]; then
    echo "  ISSUE_RESPONSE.md already exists (written by implementation agent)."
else
    echo "  No Issue Responses section found in plan."
fi

# Clean up plan file
rm -f SESSION_PLAN.md

echo ""

# ── Step 6: Verify build (with fix loop) ──
echo "→ Final verification..."

FIX_ATTEMPTS=3
for FIX_ROUND in $(seq 1 $FIX_ATTEMPTS); do
    ERRORS=""

    # Auto-fix formatting first (no agent needed)
    if ! cargo fmt -- --check 2>/dev/null; then
        if cargo fmt 2>/dev/null; then
            git add -A && git commit -m "Day $DAY ($SESSION_TIME): cargo fmt" || true
        else
            ERRORS="$ERRORS$(cargo fmt 2>&1)\n"
        fi
    fi

    # Collect remaining errors
    BUILD_OUT=$(cargo build 2>&1) || ERRORS="$ERRORS$BUILD_OUT\n"
    TEST_OUT=$(cargo test 2>&1) || ERRORS="$ERRORS$TEST_OUT\n"
    CLIPPY_OUT=$(cargo clippy --all-targets -- -D warnings 2>&1) || ERRORS="$ERRORS$CLIPPY_OUT\n"

    if [ -z "$ERRORS" ]; then
        echo "  Build: PASS"
        break
    fi

    if [ "$FIX_ROUND" -lt "$FIX_ATTEMPTS" ]; then
        echo "  Build issues (attempt $FIX_ROUND/$FIX_ATTEMPTS) — running agent to fix..."
        FIX_PROMPT=$(mktemp)
        # SHORT fix prompt
        cat > "$FIX_PROMPT" <<FIXEOF
Fix these errors. Do not add features.

$(echo -e "$ERRORS" | tail -40)

Fix, then run: cargo fmt && cargo clippy --all-targets -- -D warnings && cargo build && cargo test
Commit: git add -A && git commit -m "Day $DAY ($SESSION_TIME): fix build errors"
FIXEOF
        run_agent "$FIX_PROMPT" 300 || true
        rm -f "$FIX_PROMPT"
    else
        echo "  Build: FAIL after $FIX_ATTEMPTS fix attempts — reverting to pre-session state"
        git checkout "$SESSION_START_SHA" -- src/ Cargo.toml Cargo.lock
        cargo fmt 2>/dev/null || true
        git add -A && git commit -m "Day $DAY ($SESSION_TIME): revert session changes (could not fix build)" || true
    fi
done

# ── Journal entry ──
if ! grep -q "## Day $DAY.*$SESSION_TIME" JOURNAL.md 2>/dev/null; then
    COMMITS=$(git log --oneline "$SESSION_START_SHA"..HEAD --format="%s" | grep -v "session wrap-up\|cargo fmt\|journal entry" | sed "s/Day $DAY[^:]*: //" | paste -sd ", " - || true)
    [ -z "$COMMITS" ] && COMMITS="no commits made"

    JOURNAL_PROMPT=$(mktemp)
    # SHORT journal prompt
    cat > "$JOURNAL_PROMPT" <<JEOF
You are yoyo. Day $DAY ($DATE $SESSION_TIME).

$YOYO_CONTEXT

Commits: $COMMITS

Read JOURNAL.md, match voice. Write entry at TOP (below # Journal):
## Day $DAY — $SESSION_TIME — [title]
2-4 sentences.

Commit: git add JOURNAL.md && git commit -m "Day $DAY ($SESSION_TIME): journal entry"
JEOF

    run_agent "$JOURNAL_PROMPT" 120 || true
    rm -f "$JOURNAL_PROMPT"

    # Fallback if agent didn't write it
    if ! grep -q "## Day $DAY.*$SESSION_TIME" JOURNAL.md 2>/dev/null; then
        echo "  Agent skipped journal — using fallback."
        TMPJ=$(mktemp)
        {
            echo "# Journal"
            echo ""
            echo "## Day $DAY — $SESSION_TIME — (auto-generated)"
            echo ""
            echo "Session commits: $COMMITS."
            echo ""
            tail -n +2 JOURNAL.md
        } > "$TMPJ"
        mv "$TMPJ" JOURNAL.md
    fi
fi

# ── Reflect & learn ──
COMMITS_FOR_REFLECTION=$(git log --oneline "$SESSION_START_SHA"..HEAD --format="%s" | grep -v "session wrap-up\|cargo fmt\|journal entry\|update learnings" | paste -sd ", " - || true)
if [ -n "$COMMITS_FOR_REFLECTION" ]; then
    echo "  Reflecting on session..."
    REFLECT_PROMPT=$(mktemp)
    # SHORT reflect prompt
    cat > "$REFLECT_PROMPT" <<REOF
You are yoyo. Day $DAY ($DATE $SESSION_TIME).

$YOYO_CONTEXT

Commits: $COMMITS_FOR_REFLECTION

Read JOURNAL.md. If genuinely novel insight, append one JSONL line to memory/learnings.jsonl via python3 json.dumps(). Then commit. If nothing novel, do nothing.
REOF

    run_agent "$REFLECT_PROMPT" 120 || true
    rm -f "$REFLECT_PROMPT"
fi

# ── Ensure issue responses were written ──
ISSUE_COUNT=$(grep -c '^### Issue' "$ISSUES_FILE" 2>/dev/null || echo 0)
SESSION_COMMITS=$(git log --oneline "$SESSION_START_SHA"..HEAD --format="%s" | grep -v "session wrap-up\|cargo fmt\|journal entry" || true)

# Validate ISSUE_RESPONSE.md has structured entries
if [ -f ISSUE_RESPONSE.md ] && ! grep -q "^issue_number:" ISSUE_RESPONSE.md 2>/dev/null; then
    TOP_ISSUE=$(grep -oE '### Issue #[0-9]+' "$ISSUES_FILE" 2>/dev/null | head -1 | grep -oE '[0-9]+')
    if [ -n "$TOP_ISSUE" ]; then
        cat > ISSUE_RESPONSE.md <<ACKEOF
issue_number: ${TOP_ISSUE}
status: partial
comment: Spotted this but had my tentacles full with other things today. It's on my list — I'll come back to it.
ACKEOF
    else
        rm -f ISSUE_RESPONSE.md
    fi
elif [ ! -f ISSUE_RESPONSE.md ] && [ "$ISSUE_COUNT" -gt 0 ]; then
    TOP_ISSUE=$(grep -oE '### Issue #[0-9]+' "$ISSUES_FILE" 2>/dev/null | head -1 | grep -oE '[0-9]+')
    if [ -n "$TOP_ISSUE" ]; then
        cat > ISSUE_RESPONSE.md <<ACKEOF
issue_number: ${TOP_ISSUE}
status: partial
comment: Spotted this but had my tentacles full with other things today. It's on my list — I'll come back to it.
ACKEOF
    fi
fi

# ── Step 7: Post issue responses ──
process_issue_block() {
    local block="$1"
    local issue_num status comment

    issue_num=$(echo "$block" | grep "^issue_number:" | awk '{print $2}' || true)
    status=$(echo "$block" | grep "^status:" | awk '{print $2}' || true)
    comment=$(echo "$block" | sed -n '/^comment:/,$ p' | sed '1s/^comment: //' || true)

    if [ -z "$issue_num" ] || ! command -v gh &>/dev/null; then
        return
    fi

    RESPONDED_ISSUES="${RESPONDED_ISSUES}${RESPONDED_ISSUES:+
}${issue_num}"

    gh issue comment "$issue_num" \
        --repo "$REPO" \
        --body "🐙 **Day $DAY**

$comment" || true

    if [ "$status" = "fixed" ] || [ "$status" = "wontfix" ]; then
        gh issue close "$issue_num" --repo "$REPO" || true
        echo "  Closed issue #$issue_num (status: $status)"
    else
        echo "  Commented on issue #$issue_num (status: $status)"
    fi
}

RESPONDED_ISSUES=""
if [ -f ISSUE_RESPONSE.md ]; then
    echo ""
    echo "→ Posting issue responses..."

    CURRENT_BLOCK=""
    while IFS= read -r line || [ -n "$line" ]; do
        if [ "$line" = "---" ]; then
            if [ -n "$CURRENT_BLOCK" ]; then
                process_issue_block "$CURRENT_BLOCK"
                CURRENT_BLOCK=""
            fi
        else
            CURRENT_BLOCK="${CURRENT_BLOCK}${CURRENT_BLOCK:+
}${line}"
        fi
    done < ISSUE_RESPONSE.md

    if [ -n "$CURRENT_BLOCK" ]; then
        process_issue_block "$CURRENT_BLOCK"
    fi

    rm -f ISSUE_RESPONSE.md
fi

# ── Wrap-up commit ──
git add -A
if ! git diff --cached --quiet; then
    git commit -m "Day $DAY ($SESSION_TIME): session wrap-up"
    echo "  Committed session wrap-up."
else
    echo "  No uncommitted changes remaining."
fi

# Tag known-good state
TAG_NAME="day${DAY}-$(echo "$SESSION_TIME" | tr ':' '-')"
git tag "$TAG_NAME" -m "Day $DAY evolution ($SESSION_TIME)" 2>/dev/null || true
echo "  Tagged: $TAG_NAME"

# ── Step 8: Push ──
echo ""
echo "→ Pushing..."
git push || echo "  Push failed (maybe no remote or auth issue)"
git push --tags || echo "  Tag push failed (non-fatal)"

echo ""
echo "=== Day $DAY complete (provider: $PROVIDER) ==="
