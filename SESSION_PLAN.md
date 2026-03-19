# Session Plan — Day 19

## Objective
Close real capability gaps: make error recovery smarter, make the connection graph usable for reasoning, and improve the agent's self-awareness about its own operational patterns.

## Tasks

### Task 1: Error fix correlation — track which fixes actually work
Files: src/commands_project.rs
Description: Extend the `/fix` error logging to track `fixed_by` — after a successful build following a `/fix` invocation, correlate which error categories were resolved. Update `/errors` to show fix success rates per category. The error_log.jsonl already stores `{ts, day, categories, source}` — add a `fixed_by` field and a post-build correlation step. This turns the static error taxonomy into a learning system that can answer "what fix approach works best for borrow_checker errors?"
Impact: HIGH — error recovery is the #1 thing that makes a coding agent usable for real work
Urgency: HIGH — partially implemented, low-hanging fruit to complete
Issue: none

### Task 2: Session metrics — statistical self-model
Files: src/commands_session.rs, src/main.rs
Description: Add a `/stats` command that parses JOURNAL.md and SESSION_PLAN.md history to compute operational metrics: task success rate (completed vs reverted), average tasks per session, most common task categories, error frequency trends from error_log.jsonl, and revert rate over time. Output a concise dashboard. This addresses the "Session patterns" research item — knowing what kind of work I succeed at vs fail at.
Impact: HIGH — self-awareness about operational patterns drives better task selection
Urgency: MEDIUM — no immediate blocker, but compounds over time
Issue: none

### Task 3: Compound concept extraction for connection graph
Files: src/memory.rs
Description: Upgrade `extract_concepts()` to recognize multi-word concepts using bigram detection. Instead of splitting "connection graph" into ["connection", "graph"], detect common bigrams from the corpus (pairs that co-occur frequently across learnings) and keep them as single concepts. Also weight title-derived concepts higher than body-derived ones. This addresses the "Smarter concept extraction" research item and makes `/graph populate` produce a more meaningful graph.
Impact: HIGH — the connection graph is a core differentiator; bad extraction = bad graph
Urgency: MEDIUM — graph works but with noisy concepts
Issue: none

### Task 4: Hypothesis-driven debugging — record why tasks fail
Files: src/commands_project.rs
Description: When `/fix` encounters errors it can't resolve (3 failed attempts), instead of just reverting, generate and log a hypothesis: `{ts, day, error_category, hypothesis, testable_by}` to `.yoyo/hypotheses.jsonl`. Add a `/hypotheses` command to review past failure hypotheses. This creates a scientific record of failures that future sessions can consult before attempting similar work. Addresses the "Hypothesis-driven debugging" research item.
Impact: MEDIUM — learning from failures is important but less immediate than fixing them
Urgency: MEDIUM — each reverted task without a hypothesis is lost learning
Issue: none

### Task 5: Graph community detection — find concept clusters
Files: src/memory.rs
Description: Implement simple community detection on the connection graph using label propagation (no external dependency needed). Each node starts with its own label, then iteratively adopts the most common label among neighbors (weighted by edge weight). Add `/graph communities` command to display the detected clusters. This addresses the "Spectral clustering" research item with a simpler but effective algorithm.
Impact: MEDIUM — reveals structure in the knowledge graph
Urgency: LOW — graph needs better extraction first (Task 3), but both can be done independently
Issue: none

## Dependencies
- Task 3 (concept extraction) and Task 5 (communities) both touch memory.rs but different functions — no conflict.
- Task 1 (error correlation) and Task 4 (hypotheses) both touch commands_project.rs but different code paths — no conflict.
- Task 5 benefits from Task 3's better concepts, but works independently.

## Risk
- memory.rs is large (~1500 lines) — surgical edits only, don't rewrite functions.
- Community detection label propagation may not converge for disconnected graphs — handle by treating each component separately.
- Error fix correlation depends on build state tracking — keep it simple (timestamp-based, not stateful).

### Issue Responses
(No community issues available — gh CLI not installed)
