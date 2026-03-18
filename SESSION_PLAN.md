## Session Plan

### Task 1: Wire error classification into /health failure output
Files: src/commands_project.rs
Description: When `/health` checks fail, the output shows "✗ clippy: FAIL" with raw detail text but no structured analysis. The error classifier (classify_rust_error + fix_strategy) already exists and is used by `/fix`. Wire it into `handle_health()`: when a check fails, run the failure detail through `classify_rust_error` and print a one-line classification summary below the check result (e.g., "  → 2 missing_import, 1 borrow_checker"). This gives users immediate diagnostic insight without needing to run `/fix`. Add tests for the new behavior: health output with classified errors, health output when all pass (no classification shown).
Issue: none

### Task 2: Parse test result summary from cargo test output
Files: src/commands_project.rs
Description: `/test` dumps raw cargo test output and says "Tests passed" or "Tests failed" — but doesn't parse or display structured results. For Rust projects, cargo test outputs a summary line like "test result: ok. 684 passed; 0 failed; 0 ignored". Parse this line from the stdout and display a structured summary: "684 passed, 0 failed, 0 ignored". Add a `parse_test_summary(output: &str) -> Option<TestSummary>` function that extracts pass/fail/ignore counts from the cargo test result line, and a struct `TestSummary { passed: u32, failed: u32, ignored: u32 }`. Show the parsed summary after the pass/fail message. Write tests for the parser: standard output, no-match output, multiple test result lines (unit + integration — show totals).
Issue: none

### Issue Responses
(No community issues today)
