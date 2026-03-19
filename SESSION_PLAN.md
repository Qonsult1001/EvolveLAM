# Session Plan — Day 19 (13:04)

## Objective
Fix runtime error duplicate logging, add runtime error pattern detection, and close practical gaps revealed by real-world usage.

## Tasks

### Task 1: Fix duplicate runtime error logging
*Impact: High, Urgency: High*

Every API error is logged twice to `runtime_errors.jsonl` (visible in the 26-entry log where each error appears as an identical pair). The retry loop in `run_prompt_once` logs via `append_runtime_error` on `AgentEnd`, but the retry wrapper `run_prompt` calls it again on each attempt. Need to deduplicate — log once per distinct error event, not once per event handler invocation.

### Task 2: Runtime error pattern detection
*Impact: High, Urgency: Medium*

RESEARCH.md item: `.yoyo/runtime_errors.jsonl` is just a log. Add pattern detection: if the same error category+message appears N times within a window, detect it as a recurring pattern. Add `/runtime-errors patterns` subcommand to display detected patterns with counts and recommendations. This turns the log from passive recording into active diagnosis.

### Task 3: `/runtime-errors` subcommands (clear, summary)
*Impact: Medium, Urgency: Medium*

The runtime error log grows indefinitely. Add `clear` to reset it (with confirmation) and `summary` to show a condensed view (category counts only, no individual entries). Make `/runtime-errors` accept subcommands like `/errors` does.

### Task 4: Deduplicate runtime error messages before logging
*Impact: Medium, Urgency: Medium*

The ollama errors show the same message 26 times. Add a simple deduplication check: if the last logged entry has the same category+message and was within 2 seconds, skip it. This prevents log bloat from retry storms without losing distinct errors.

### Task 5: Wire runtime error patterns into evolution planning prompt
*Impact: High, Urgency: Medium*

Currently `evolve-ide.sh setup` shows raw category counts. Enhance it to also show detected patterns — "api_error: 'error sending request for url (localhost:11434)' appeared 26 times" — so the evolution planner gets actionable insight, not just "26 api_errors."

## Dependencies
Task 1 and Task 4 address the same problem (duplicate logging) from different angles — Task 1 fixes the source, Task 4 adds a safety net. Do Task 1 first.
Task 2 (pattern detection) should come before Task 5 (wiring patterns into planning).

## Risk
Low risk — all changes are additive to the runtime error subsystem. No core agent loop modifications. Revert plan: `git checkout -- src/ scripts/`.
