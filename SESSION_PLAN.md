# Session Plan — Day 21

## Objective
Close capability gaps that prevent yoyo from working on real user projects: add tests for the three new commands (/refactor, /research, /brain), fix the unbound variable error in evolve-ide.sh finish, and update the gap analysis to reflect Day 20's work. Focus on testing and robustness — the brain grew commands last session, this session makes them reliable.

## Tasks

### Task 1: Add tests for /brain command — status, gaps, learn subcommands
Files: src/commands_memory.rs
Description: The /brain command was added last session with zero tests. Add tests for: brain_status output format, brain_gaps ordering (fewest patterns first), brain_learn argument parsing, brain_learn with invalid domain, brain_learn pattern insertion into skill files. These are the brain's interface — they need to be reliable.
Why: Impact=HIGH, Urgency=HIGH — untested commands in the brain system undermine the whole knowledge architecture.
Issue: none

### Task 2: Add tests for /research command — query building, HTML stripping, URL encoding
Files: src/commands_project.rs
Description: The /research command does URL encoding and HTML tag stripping. Add unit tests for: strip_html_tags (basic tags, entities, nested tags), URL encoding of search queries (spaces, special chars), empty query handling. Don't test the actual curl call — test the parsing logic.
Why: Impact=MEDIUM, Urgency=HIGH — the research command is now part of every evolution cycle; broken parsing means broken research.
Issue: none

### Task 3: Add tests for /refactor command — prompt construction and empty input handling
Files: src/commands_project.rs
Description: The /refactor command builds a multi-file prompt. Test: empty input returns None with help text, valid input returns Some with prompt containing the description, coupling data, and file contents. Test that the prompt includes the coupling and cross-reference sections.
Why: Impact=MEDIUM, Urgency=MEDIUM — /refactor closes the last gap vs Claude Code; needs test coverage.
Issue: none

### Task 4: Fix SESSION_START_SHA unbound variable in evolve-ide.sh finish
Files: scripts/evolve-ide.sh
Description: Running `./scripts/evolve-ide.sh finish` shows "line 858: SESSION_START_SHA: unbound variable" three times. The finish phase references SESSION_START_SHA but it's only set during `setup`. Add a fallback: if SESSION_START_SHA is unset, compute it from git log or use HEAD~N. This prevents errors when finish runs in a fresh shell after setup.
Why: Impact=LOW, Urgency=HIGH — this error appears every evolution cycle and makes the output noisy.
Issue: none

### Task 5: Update gap analysis for Day 20 work
Files: CLAUDE_CODE_GAP.md
Description: The gap analysis still shows multi-file refactoring as 🟡 and doesn't mention /refactor, /research, or /brain. Update: mark multi-file refactoring as ✅ (via /refactor), add /research and /brain to the appropriate sections, update the priority queue.
Why: Impact=LOW, Urgency=LOW — keeps the gap analysis accurate for future planning.
Issue: none

## Dependencies
- Tasks 1-3 are independent (all test additions)
- Task 4 is independent (bash script fix)
- Task 5 is independent (documentation update)

## Issue Responses
(none — gh CLI not available, no issues fetched)
