# Session Plan — Day 19

## Objective
Make yoyo a production-ready IDE backend by hardening the serve endpoint, improving the init experience for external projects, and cleaning up compile warnings from the enhanced init code that was written but never verified.

### Task 1: Fix and verify enhanced /init implementation
Files: src/commands_project.rs
Description: The enhanced `generate_init_content()` was written earlier today (README extraction, dependency parsing, source file summaries, entry point detection) but never compiled through clippy. Run `cargo clippy` to find issues, fix them, add tests for `extract_readme_description`, `extract_cargo_dependencies`, `scan_source_summaries`, and `detect_entry_point`. Verify the code handles edge cases: missing README, no Cargo.toml, empty src directory.
Issue: none

### Task 2: Add /health endpoint to serve mode
Files: src/serve.rs
Description: IDE clients (Continue, Cursor) probe `/health` or `/v1/health` before connecting. Add a simple health check endpoint that returns `{"status": "ok", "model": "...", "provider": "..."}`. Also add `GET /` with a human-readable status page. This prevents "connection refused" confusion when clients can't verify the server is running.
Issue: none

### Task 3: Serve mode conversation isolation
Files: src/serve.rs
Description: Currently the server uses a single shared Agent behind `Arc<Mutex<Agent>>`, so all clients share one conversation. For IDE use, each request should get a fresh agent context (or at minimum, support a `conversation_id` concept). Implement request-scoped agent creation for chat completions — each POST creates a fresh agent, runs the prompt, and discards it. This prevents conversation bleed between different IDE windows/tabs.
Issue: none

### Task 4: Wire /init to read actual file contents for external projects
Files: src/commands_project.rs
Description: The current `/init` detects project type and lists files but doesn't read package.json `description`, pyproject.toml `[project].description`, or Cargo.toml `[package].description`. Add extraction of project description from the canonical config file for each project type. Also extract the `name` field. This makes `/init` produce useful YOYO.md for any project type, not just ones with README.md.
Issue: none

### Task 5: Improve runtime error handling for connection failures
Files: src/prompt.rs, src/serve.rs
Description: All 54 runtime errors are "error sending request for url (localhost:11434)" — ollama not running. The error message is unhelpful. Add connection error detection: when the error contains "error sending request" or "connection refused", print a clear diagnostic: "Cannot reach [provider] at [url]. Is the service running?" Also add a `/doctor` command that checks if the configured provider endpoint is reachable (simple TCP connect test).
Issue: none

### Issue Responses
(no issues fetched — gh CLI not available)
