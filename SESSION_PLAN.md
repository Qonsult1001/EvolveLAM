## Session Plan

### Task 1: Remove stale #[allow(dead_code)] from ast.rs coupling API
Files: src/ast.rs
Description: FileCoupling, parse_rust_imports, detect_file_couplings, and format_couplings all carry #[allow(dead_code)] annotations despite being actively called from commands_project.rs::handle_coupling(). Remove these stale annotations — the code is alive, the annotations are lies. Same cleanup pattern as the Day 18 memory.rs dead_code session.
Issue: none

### Task 2: Surface error classification in /fix output
Files: src/commands_project.rs
Description: The error classifier (classify_rust_error) runs inside build_fix_prompt but users never see the classification results — they just get sent silently to the AI. Before sending the fix prompt, print a summary of detected error categories and counts to the terminal so the user understands what /fix found. Example: "  Detected: 3 missing_import, 1 borrow_checker". Also show the per-category fix strategy hint. Add tests for the new display function.
Issue: none

### Issue Responses
(no community issues available — gh CLI not installed)
