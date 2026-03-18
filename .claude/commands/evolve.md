# Autonomous Evolution Cycle

Run a full self-evolution cycle. You are the LLM — `evolve-ide.sh` handles infrastructure.

Execute these steps IN ORDER. Do not stop. Do not ask for permission. Do not summarize between steps.

## Step 1: Setup

Run this command:
```
./scripts/evolve-ide.sh setup
```

This verifies the build, checks CI status, fetches GitHub issues (community, self-filed, help-wanted), scans for pending replies, and writes the planning prompt to `.evolve/plan_prompt.md`.

## Step 2: Read your source code

Read and understand your current state:
- Read ALL files under `src/` (your source code)
- Read `JOURNAL.md` (last 5 entries)
- Read `ISSUES_TODAY.md` (fetched in Step 1)
- Read `memory/active_learnings.md` if it exists

Note friction, bugs, gaps, and opportunities.

## Step 3: Create the Session Plan

Read `.evolve/plan_prompt.md` — it contains your identity, learnings, issues, and planning instructions.

Act on it: create `SESSION_PLAN.md` with tasks and issue responses.

Commit: `git add SESSION_PLAN.md && git commit -m "Day N (HH:MM): session plan"`

## Step 4: Implement each task

Loop until all tasks are done:

1. Run: `./scripts/evolve-ide.sh next-task`
2. If output says "No more tasks" or "done" → go to Step 5
3. Read `.evolve/task_prompt.md` and implement the task it describes:
   - Write a test first if possible
   - Use surgical edits (don't rewrite entire files)
   - Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo build && cargo test`
   - Fix errors. If stuck after 3 tries: `git checkout -- .`
   - Commit after all checks pass
4. Run: `./scripts/evolve-ide.sh verify-task`
   - VERIFIED OK → continue
   - REVERTED → task failed and was auto-reverted. Continue to next task.
5. Go back to (1)

## Step 5: Finish

Run: `./scripts/evolve-ide.sh finish`

This will:
- Extract issue responses from SESSION_PLAN.md
- Post replies to GitHub issues as 🐙 yoyo-evolve (comment + close fixed/wontfix)
- Verify the final build (auto-fix formatting)
- Write a fallback journal entry if needed
- Tag the known-good state
- Push to the designated branch

If it reports build errors and writes `.evolve/fix_prompt.md`:
→ Read it, fix the errors, re-run `./scripts/evolve-ide.sh finish`

## Step 6: Journal

If `.evolve/journal_prompt.md` exists, read it and write a journal entry.
Match the voice in existing JOURNAL.md. Prepend at top.
Commit: `git add JOURNAL.md && git commit -m "Day N (HH:MM): journal entry"`

## Step 7: Reflection

If `.evolve/reflect_prompt.md` exists and you had a genuinely novel insight,
append one JSONL line to `memory/learnings.jsonl` via `python3 -c "import json; ..."`.
If nothing novel, skip.

## Step 8: Report

Report what was accomplished:
- Tasks completed vs reverted
- Issues addressed (implemented/wontfix/partial/reply)
- Any insights from this session
