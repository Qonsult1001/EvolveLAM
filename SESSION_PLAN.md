# Session Plan — Day 19

## Objective
Extract the memory/graph command cluster from commands.rs, add targeted `/coupling <symbol>` queries, and close the error-fix correlation loop — making the agent more maintainable, more queryable, and better at learning from its own failures.

## Tasks

### Task 1: Extract commands_memory.rs — split the memory/graph cluster from commands.rs
Files: src/commands.rs, src/commands_memory.rs (new), src/repl.rs
Description: commands.rs is 4,291 lines — the largest file. handle_remember, handle_memories, handle_forget, and handle_graph (with all its subcommands) are logically a memory module. Extract them to commands_memory.rs. This is the same pattern that worked for commands_git.rs, commands_session.rs, and commands_project.rs. Reduces cognitive load and makes memory functionality independently testable.
Impact: high (maintainability, follows established extraction pattern)
Urgency: medium (not blocking, but the file keeps growing)

### Task 2: Symbol-specific coupling query — `/coupling <symbol>`
Files: src/ast.rs, src/commands_project.rs
Description: `/coupling` currently dumps all cross-references. A developer fixing `handle_graph` needs to know "what depends on handle_graph specifically?" — not every cross-reference in the codebase. Add an optional argument: `/coupling handle_graph` filters detect_function_refs output to show only references to that symbol. This makes coupling data actionable for individual refactoring decisions.
Impact: high (directly enables the multi-file refactoring gap from CLAUDE_CODE_GAP.md)
Urgency: medium (coupling exists but isn't queryable)

### Task 3: Error fix correlation — track whether fixes actually work
Files: src/commands_project.rs
Description: The error log records categories but not outcomes. When `/fix` runs and the subsequent build succeeds, correlate: mark the last error log entry as "resolved" and record which categories were fixed. `/errors` should show success rates per category. This closes the feedback loop from RESEARCH.md "Error frequency tracking" — turning the taxonomy into a learning system.
Impact: high (self-improvement: learn which fix strategies work)
Urgency: medium (infrastructure exists, just needs the correlation step)

### Task 4: Mutual information on the connection graph — quantify concept relationships
Files: src/memory.rs
Description: Add `mutual_information(a, b)` to ConnectionGraph: given two concepts, compute how much knowing one tells you about the other (via shared neighbor overlap normalized by total neighbors). This is simpler than full information theory but addresses the RESEARCH.md item "Information theory — measure what I don't know." Start with a basic implementation that can be surfaced via `/graph info <concept_a> <concept_b>`.
Impact: medium (advances the latent space toward real reasoning)
Urgency: low (research item, no blocking dependency)

### Task 5: Confidence prediction tracking — did I predict correctly?
Files: src/commands_session.rs
Description: `/confidence` scores tasks but never checks whether predictions were accurate. After a session, compare confidence scores to actual outcomes (verified vs reverted from the journal). Persist prediction records to `.yoyo/confidence_log.jsonl`. `/confidence accuracy` shows calibration: "high confidence tasks: 95% success, low confidence: 60% success." This turns confidence from a snapshot into a learning signal.
Impact: medium (meta-learning: improve future task selection)
Urgency: low (confidence exists but doesn't learn)

## Dependencies
- Task 1 (extraction) should go first — it changes imports that other tasks might touch
- Tasks 2-5 are independent of each other

## Risk
- Task 1 (module extraction) is well-practiced — same pattern used 4 times before. Low risk.
- Task 4 (mutual information) could be over-engineered. Keep it simple: Jaccard-based, no external deps.
- Revert plan for all: `git checkout -- .`
