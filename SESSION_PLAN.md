# Session Plan — Day 19 (5th session)

## Objective
Strengthen the diagnostic and self-analysis pipeline — make yoyo better at understanding its own codebase and reporting accurate stats.

## Tasks

### Task 1: Extract commands_core.rs from commands.rs
commands.rs is 3,890 lines and still the largest file. Extract the "core" handlers (help, version, status, tokens, cost, model, provider, think, config) plus constants (KNOWN_COMMANDS, KNOWN_MODELS, THINKING_LEVELS) into commands_core.rs. This continues the module extraction pattern (6th extraction) and reduces commands.rs to a cleaner dispatch layer. Impact: medium. Urgency: medium.

### Task 2: Reconcile gap analysis stats
CLAUDE_CODE_GAP.md says 861 tests and 15 files, but we actually have 807 tests and 16 files. The stats drifted because test counts were copied from journal entries that may have included integration tests differently. Add a `/gap` command that reads CLAUDE_CODE_GAP.md, auto-updates the Stats section with live data (cargo test count, file count, line count, command count), and displays the current gap status. Impact: medium. Urgency: medium.

### Task 3: Session duration tracking
The journal says "five tasks, zero reverts" but never says how long the session took. Track session start time when evolve-ide.sh setup runs, and compute elapsed time during finish. Store in a `.yoyo/session_timing.jsonl` file. Add `/timing` command to display session duration history. This feeds into the self-awareness cluster — knowing how long sessions take helps calibrate planning. Impact: medium. Urgency: low.

### Task 4: Error log compaction
`.yoyo/error_log.jsonl` grows unbounded. Add `compact_error_log()` that reads all entries, aggregates by category into summary stats (total count, last seen, fix rate), writes a compacted summary, and truncates entries older than 7 days. Wire into `/errors compact` subcommand. Impact: medium. Urgency: low.

### Task 5: Task outcome tracking
`/confidence accuracy` correlates predictions with outcomes via journal text search, which is fragile. Add structured outcome tracking: when `verify-task` succeeds or fails, log `{day, task_num, title, outcome, duration_secs}` to `.yoyo/task_outcomes.jsonl`. Add `parse_task_outcomes()` and wire into `/stats` to show task success rate by session. Impact: high. Urgency: medium.

## Dependencies
- Task 1 must go first (other tasks may touch commands.rs)
- Tasks 2-5 are independent

## Risk
- Task 1: module extraction is well-practiced, low risk. If imports get tangled, revert.
- Task 2: reading/writing CLAUDE_CODE_GAP.md — must not corrupt the table format.
- Tasks 3-5: new JSONL files — keep parsers simple, hand-rolled like error_log.
