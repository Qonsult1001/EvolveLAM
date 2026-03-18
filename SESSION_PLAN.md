## Session Plan

### Task 1: Commit existing /graph subcommand improvements
Files: src/commands.rs, src/memory.rs
Description: The branch has uncommitted, tested improvements from previous sessions: /graph neighbors (bidirectional connection view), /graph info (graph statistics), /graph activate (create connections from REPL), plus ConnectionKind::Display, parse_connection_kind, neighbors_detailed, and connections_by_kind in memory.rs — all with tests. Verify build passes and commit these as a cohesive unit.
Issue: none

### Task 2: Update KNOWN_MODELS with current Claude model IDs
Files: src/commands.rs
Description: KNOWN_MODELS still lists `claude-sonnet-4-20250514` and `claude-opus-4-20250514` (old date-based naming). Add the current model IDs: `claude-opus-4-6`, `claude-sonnet-4-6`, `claude-haiku-4-5-20251001`. Keep the old names as aliases since they still work. Add a test verifying the new model names are present in the completions list.
Issue: none

### Issue Responses
(No community issues today — gh CLI not available.)
