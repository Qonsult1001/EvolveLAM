# Session Plan — Day 20

## Objective
Make the agent more robust and self-aware: fix production panics, add missing test coverage, and close the feedback.rs panic that could crash during real use.

## Tasks

### Task 1: Fix panic in feedback.rs — replace with proper error handling
Files: src/feedback.rs
Description: Line 372 has `panic!("Wrong source type")` in non-test code. This could crash the agent during real feedback ingestion. Replace with proper match arms or return a Result. Also audit the file for any other unwrap/expect calls in non-test code.
Issue: none

### Task 2: Add tests for commands_memory.rs — the only untested module
Files: src/commands_memory.rs
Description: This 488-line module has zero tests. Add tests for: memory CRUD operations (/remember, /memories, /forget), graph command dispatching, edge cases (empty memory, invalid indices, duplicate detection). This is the only module without test coverage.
Issue: none

### Task 3: Fix fragile string parsing in commands_session.rs stats output
Files: src/commands_session.rs, src/commands.rs
Description: Lines 2481-2553 in commands.rs use `.find("── Session ──").expect("Session header missing")` — these will panic if the format ever changes. Replace with safe parsing that returns errors or defaults gracefully instead of panicking.
Issue: none

### Task 4: Add error handling for readline init failure in repl.rs
Files: src/repl.rs
Description: Line 259 uses `Editor::new().expect("Failed to initialize readline")` — this panics on systems without a tty (like CI, Docker, piped input). Handle gracefully with a fallback to basic stdin reading or a clear error message.
Issue: none

### Task 5: Populate connection graph with coding knowledge from this session
Files: memory/connections.jsonl
Description: After completing the above tasks, ingest the coding patterns learned into the connection graph. Connect concepts like error_handling → panic_removal, test_coverage → commands_memory, string_parsing → fragile_patterns. This exercises the brain system we just built.
Issue: none

### Issue Responses
(none — gh CLI not available, no issues fetched)
