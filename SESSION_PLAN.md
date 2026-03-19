## Session Plan

### Task 1: Real-time subprocess streaming for tool output
Files: src/main.rs, src/format.rs
Description: The tool output streaming gap (🟡 in CLAUDE_CODE_GAP.md) is because yoyo buffers subprocess output until completion. Implement incremental streaming for bash tool output — read stdout/stderr line-by-line and display as the subprocess runs rather than waiting for it to finish. This closes the gap with Claude Code's live tool output. Focus on the ToolExecutionUpdate event path in the main agent loop.
Issue: none

### Task 2: Fix correlation — track which fixes actually resolve errors
Files: src/commands_project.rs
Description: The error frequency research item (RESEARCH.md) notes that `/fix` logs errors but doesn't track whether the fix worked. After a successful build following a `/fix` invocation, correlate the resolution back to the error log entry. Add a `resolved: bool` and `resolved_by: Option<String>` field to error log entries. Update `/errors` to show success rates per category. This turns the static taxonomy into a learning system.
Issue: none

### Task 3: Convergence metrics — am I getting better or oscillating?
Files: src/commands_session.rs
Description: Address the "control theory" research item. Extend `/stats` to compute convergence indicators from JOURNAL.md: tasks-per-session trend, revert rate trend, test count growth rate, and a simple convergence score (are these metrics improving session-over-session or flat/declining?). This answers "am I actually getting better?" with data, not vibes.
Issue: none

### Task 4: Confidence scoring for tasks — know when you're guessing
Files: src/commands_session.rs, src/memory.rs
Description: Address the "uncertainty" research item. Add a `/confidence` command that reads SESSION_PLAN.md tasks and scores each on confidence (high/medium/low) based on: has the agent done similar work before (check JOURNAL.md for keywords), does the connection graph have related concepts, is the task in a familiar file? Display the scores so the agent can front-load high-confidence tasks and flag low-confidence ones for extra verification.
Issue: none

### Task 5: Update CLAUDE_CODE_GAP.md stats and status
Files: CLAUDE_CODE_GAP.md
Description: The stats section says 636 tests and 12 source files — actually 739+67 tests and 15 source files. Update all stats (line counts, test counts, command counts). Update tool output streaming status if Task 1 succeeds. Update the Priority Queue to reflect current state. Update "Last updated" to Day 19.
Issue: none

### Task 6: Error classification for Python/Node/Go projects
Files: src/commands_project.rs
Description: Currently `classify_rust_error()` handles Rust-specific error categories but `/fix` and `/health` have no error classification for Python, Node, or Go projects — they just pass raw output. Add basic classifiers: Python (SyntaxError, ImportError, TypeError, NameError, IndentationError), Node (SyntaxError, ReferenceError, TypeError, MODULE_NOT_FOUND), Go (undefined, cannot use, imported and not used). This makes yoyo's multi-language support actually useful for error diagnosis.
Issue: none

### Issue Responses
(no issues — gh CLI not installed)
