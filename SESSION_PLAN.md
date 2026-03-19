# Session Plan — Day 19 (7th session)

## Objective
Close the runtime observability gap — real user sessions produce errors that evolution never sees. Add persistent runtime error logging so the evolution cycle can learn from practical failures (API errors, tool failures, stream interruptions).

## Tasks

### Task 1: Add runtime error logging to prompt.rs
Currently tool failures, API errors (429, stream ended), and input rejections are only printed to stderr and lost. Add `append_runtime_error(category, message, tool_name)` that writes to `.yoyo/runtime_errors.jsonl`. Call it from `run_prompt_once()` when: (1) tool execution ends with `is_error=true`, (2) API returns retriable/non-retriable error, (3) stream ends unexpectedly. Format: `{"ts":"ISO8601","category":"tool_failure|api_error|stream_error","tool":"tool_name","message":"..."}`. Impact: high. Urgency: high.

### Task 2: Add /runtime-errors command to review runtime failures
Add a command that reads `.yoyo/runtime_errors.jsonl` and displays a summary: total errors by category, most common tool failures, recent API errors. This gives the evolution cycle visibility into what's actually breaking during real sessions. Wire into repl.rs dispatch. Impact: high. Urgency: high.

### Task 3: Skip binary files in read_file tool
The user session showed yoyo trying to read a 12MB `.so` file — wasting tokens and producing no useful output. The tool already rejects files >1MB but the model still tries. Add a binary file extension check (`.so`, `.dll`, `.exe`, `.o`, `.a`, `.dylib`, `.pyc`, `.class`, `.jar`, `.wasm`, `.bin`) that returns a clear "skipped binary file" message instead of attempting the read. This saves tokens and gives the model useful feedback. Impact: medium. Urgency: medium.

### Task 4: Fix list_files tool on current directory
The user session showed `ls` failing with "Directory not found: ." — the `list_files` tool can't resolve `.` as a valid path. Diagnose and fix the path resolution so `.` and empty path both work correctly. Impact: medium. Urgency: high.

### Task 5: Feed runtime errors into evolution planning
Wire `.yoyo/runtime_errors.jsonl` into the evolution setup phase so `evolve-ide.sh` includes a summary of recent runtime failures in the planning prompt. This closes the feedback loop: real-world failures → evolution plans → fixes. Add a section to the plan_prompt.md output during `phase_setup()`. Impact: high. Urgency: medium.

## Dependencies
- Task 1 must complete before Task 2 (needs the logging function and JSONL format)
- Task 2 must complete before Task 5 (parsing functions reused)
- Tasks 3-4 are independent

## Risk
- Tasks 3-4: tool implementations may be in yoagent crate, not in yoyo source. If so, the fix would need to be a wrapper/filter layer in prompt.rs or main.rs rather than modifying the tool directly.
- Task 1: needs careful placement in the event loop to avoid logging noise (e.g., don't log expected "no results" from search)
