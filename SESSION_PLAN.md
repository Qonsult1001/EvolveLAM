## Session Plan

### Task 1: Track error frequency across sessions
Files: src/commands_project.rs, src/commands.rs
Description: Add error frequency tracking to a JSONL file (`.yoyo/error_log.jsonl`). When `/fix` classifies errors before sending to the AI, also append a record `{"ts":"ISO8601","day":N,"categories":{"missing_import":3,"borrow_checker":1},"source":"fix"}` to the log. Add a `/errors` command that reads the log and displays: (1) total error count by category across all sessions, (2) most common error type, (3) last 5 error events with timestamps. Write a parser for the JSONL file and 5+ tests for the parser and summary display. This addresses the RESEARCH.md item "Error frequency tracking — learn which fixes work."
Issue: none

### Task 2: Show per-check timing breakdown in /health summary
Files: src/commands_project.rs, src/commands.rs
Description: The `/health` command already times each check internally (via `Instant::now()` + `format_duration()`), but doesn't show a total timing summary at the end. After the per-check results, add a timing summary line: "Health check completed in X.Xs (build: Y.Ys, test: Z.Zs, ...)". Modify `run_health_checks_with_classification` to also return elapsed duration per check (as `Duration`), and use it to compute the summary. Add 3+ tests verifying the timing summary format.
Issue: none

### Issue Responses
(none — gh CLI not installed)
