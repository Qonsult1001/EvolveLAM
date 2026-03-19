# Autonomous Evolution Cycle

Run a full self-evolution cycle. You are the LLM — `evolve-ide.sh` handles infrastructure.

**One command, full pipeline:** Execute every step below in order. You create the plan (SESSION_PLAN.md), implement tasks, verify, then finish. No user intervention — do not stop, do not ask for permission, do not summarize between steps.

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
- Read `RESEARCH.md` (research backlog — topics to study)
- Read `memory/active_learnings.md` if it exists
- Read `skills/plan/SKILL.md` (planning framework)

Note friction, bugs, gaps, and opportunities.

## Step 3: Create the Session Plan

Read `.evolve/plan_prompt.md` — it contains your identity, learnings, issues, and planning instructions.

Follow the `plan` skill's prioritization framework: score each potential task on Impact and Urgency. Be ambitious — plan up to 5 meaningful tasks that close real capability gaps, not just trivial fixes. Each task should move the needle toward "could a real developer use me for real work?"

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

## Step 5: Finish (build verify + issue extraction)

Run: `./scripts/evolve-ide.sh finish`

This will:
- Extract issue responses from SESSION_PLAN.md
- Verify the final build (auto-fix formatting)
- Write journal and reflection prompts to `.evolve/`

If it reports build errors and writes `.evolve/fix_prompt.md`:
→ Read it, fix the errors, re-run `./scripts/evolve-ide.sh finish`

## Step 6: Write Journal Entry

Read `.evolve/journal_prompt.md`. It contains the commit list and formatting instructions.

Write a REAL journal entry — not a commit list. Describe:
- What you built and WHY (not just what files changed)
- Context: what led to this work, what it enables
- Be honest about what went well and what didn't
- Match the voice in existing JOURNAL.md entries (read the last 3)
- Read PERSONALITY.md for tone — curious, honest, a little stubborn

Prepend the entry at the top of JOURNAL.md (below `# Journal`).
Commit: `git add JOURNAL.md && git commit -m "Day N (HH:MM): journal entry"`

## Step 7: Reflection & Research Gaps

If `.evolve/reflect_prompt.md` exists, read it. Two outputs:

1. **Learning** (optional): If genuinely novel insight (about yourself, not code patterns),
   append one JSONL line to `memory/learnings.jsonl` via `python3 json.dumps()`.

2. **Research gaps** (always consider): Ask "what do I need to learn next?" If this session
   revealed a knowledge gap, a technique you couldn't handle, or an area where deeper study
   would help — append a `### [ ]` entry to `RESEARCH.md` with a Goal explaining what to
   study and why. This is how you grow — by identifying what's missing.
   Commit: `git add RESEARCH.md && git commit -m "Day N (HH:MM): research gaps"`

## Step 8: Wrap-up (post issues, tag, push)

Run: `./scripts/evolve-ide.sh finish`

This will:
- Post replies to GitHub issues as 🐙 yoyo-evolve (comment + close fixed/wontfix)
- Write a fallback journal if you didn't write one in Step 6
- Commit any remaining changes (e.g. session wrap-up)
- Tag the known-good state (e.g. dayN-HH-MM)
- Push to the designated branch

## Step 9: Report

Report what was accomplished:
- Tasks completed vs reverted
- Issues addressed (implemented/wontfix/partial/reply)
- Journal entry title
- Any insights from this session
