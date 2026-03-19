## Session Plan

### Task 1: Richer /spawn orchestration — result aggregation and task decomposition
Files: src/commands_session.rs, src/commands.rs, src/repl.rs
Description: The gap analysis's top priority item. Currently /spawn runs a task in a fresh agent context but just prints the result. Upgrade to support: (1) `/spawn` returns structured output that can be referenced later, (2) multiple spawned agents can run and their results aggregated, (3) add a `/spawn list` subcommand to see active/completed spawns. This closes the 🟡 subagent orchestration gap toward ✅.
Issue: none

### Task 2: Graceful degradation on partial tool failures
Files: src/prompt.rs, src/main.rs
Description: The second gap analysis 🟡 item. When a tool partially fails (e.g., bash returns non-zero but has useful stdout, or edit_file fails on one file but succeeds on others in parallel), yoyo currently treats it as a hard error. Add logic to: (1) capture partial success from tool results, (2) continue the agent loop with the successful results rather than aborting, (3) surface the failure clearly but non-fatally. Check how yoagent's ToolResult handles partial success and adapt.
Issue: none

### Task 3: Mark control theory research item as partially done
Files: RESEARCH.md
Description: The "Control theory — am I converging or oscillating?" research item was addressed by the convergence metrics in `/stats` (linear regression on revert rate, test growth, activity trend). Mark it as partially implemented with a note about what's done and what's left (PID-style feedback loop). Also mark "Uncertainty" and "Session patterns" as partially done since `/confidence` and `/stats` address those.
Issue: none

### Task 4: Deep file coupling — function-level cross-reference tracking
Files: src/ast.rs, src/commands_project.rs
Description: Address the "deep file coupling" research item. Currently /coupling parses `use crate::module` for module-level deps. Extend to track which public functions/types from module B are actually referenced in file A. Use the existing ast.rs symbol finder to extract function names, then grep for their usage across files. Output: "if you change handle_graph in commands.rs, these files reference it: repl.rs:429, commands_session.rs:548". This turns /coupling from "which modules depend on each other" to "which specific functions create the coupling."
Issue: none

### Task 5: Update CLAUDE_CODE_GAP.md — close remaining gaps
Files: CLAUDE_CODE_GAP.md
Description: If Tasks 1 and 2 succeed, update the subagent orchestration and graceful degradation rows from 🟡 to ✅. Update test counts and stats. Update Priority Queue section. Mark Day 19 completions.
Issue: none

### Issue Responses
(no issues — gh CLI not installed)
