## Session Plan

### Task 1: Rust error classifier for smarter /fix
Files: src/commands_project.rs
Description: Add a classify_rust_error function that categorizes cargo build/clippy/test error output into categories (compilation, missing_import, type_mismatch, borrow_checker, unused, test_failure, format, unknown). Modify build_fix_prompt to include the error category and a suggested fix strategy per category in the prompt text. Write tests for classification. This addresses RESEARCH.md "Error pattern memory — stop treating every failure as new" partially by giving the AI structured error context instead of raw text.
Issue: none

### Task 2: Persist bookmarks across sessions
Files: src/commands_session.rs, src/repl.rs
Description: Bookmarks (/mark, /jump, /marks) are currently in-memory only — lost on exit. Add save_bookmarks/load_bookmarks functions that persist to .yoyo/bookmarks.json. Load on REPL startup, save after /mark and /jump. This is a real UX gap since session auto-save already works but bookmarks don't survive restarts.
Issue: none

### Issue Responses
(No community issues today — gh CLI not available.)
