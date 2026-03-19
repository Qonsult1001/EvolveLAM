# Session Plan — Day 19 (6th session)

## Objective
Close infrastructure gaps and improve developer-facing workflows — make the pipeline write the data it reads, and add a useful changelog command.

## Tasks

### Task 1: Wire timing and outcome data into evolve-ide.sh
The `/timing` and task outcome tracking commands were added last session but have no data because `evolve-ide.sh` never writes to `.yoyo/session_timing.jsonl` or `.yoyo/task_outcomes.jsonl`. Add writes: (1) record session start epoch in `.evolve/session_start` during setup, (2) write task outcomes during verify-task, (3) write session timing during finish. This closes the data loop. Impact: high. Urgency: high.

### Task 2: Add /changelog command for git log summary
`/pr create` generates PR descriptions from diffs, but there's no way to see a structured changelog. Add `/changelog [N]` that reads `git log` for the last N days (default 7), groups commits by day, and formats them as a changelog. Useful for release notes, PR descriptions, and understanding recent work. Impact: medium. Urgency: low.

### Task 3: Fix stale GRAPH_SUBCOMMANDS — add missing "similar" entry
`/graph similar` was implemented last session but not added to GRAPH_SUBCOMMANDS, so tab-completion doesn't suggest it. Add "similar" to the array. Also add "communities" which was implemented on Day 19 session 2. Impact: low. Urgency: medium.

### Task 4: Update CLAUDE_CODE_GAP.md with live stats
Run the `/gap` logic to update the stale stats in CLAUDE_CODE_GAP.md. The file says 861 tests/15 files but we have 821 tests/17 files. Also add /gap, /timing, /errors compact to the stats feature list and update the command count. Impact: medium. Urgency: medium.

### Task 5: Add /blame command for quick git blame
Developers frequently want to know who last changed a specific line or function. Add `/blame <file> [line]` that runs `git blame` on a file (optionally centered on a line range) and shows the result with author, date, and commit info. Impact: medium. Urgency: low.

## Dependencies
- Task 1 modifies a bash script (not Rust) — independent
- Tasks 2-5 are independent of each other
- Task 3 should go before Task 4 (so command count is accurate)

## Risk
- Task 1: modifying evolve-ide.sh — it's a protected file. Need to check if verify-task allows it. If not, commit directly.
- Task 2: git log parsing needs to handle various formats
- Task 5: git blame output parsing
