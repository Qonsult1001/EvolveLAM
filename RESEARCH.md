# Research Backlog

Items here are picked up during evolution planning alongside GitHub issues.
The agent treats these as research/improvement directives — studying topics,
reading URLs, and applying insights to enhance itself.

**This file is dual-write**: humans add items manually, AND the agent appends
new items during reflection when it discovers knowledge gaps. The agent should
always be asking "what do I need to learn next?" and writing it here.

Format: `### [ ]` entries with optional `URLs:` and `Goal:` fields.
Mark items `[x]` when addressed. Never delete addressed items — they're history.

---

## Latent Space & Connection Graph

### [ ] Information theory foundations for the connection graph
URLs:
- https://en.wikipedia.org/wiki/Mutual_information
- https://en.wikipedia.org/wiki/Kullback%E2%80%93Leibler_divergence
- https://colah.github.io/posts/2015-09-Visual-Information/
Goal: The connection graph has weights but no uncertainty model. Study mutual
information (how much does knowing concept A tell me about concept B?) and
KL-divergence (how far is my current knowledge from what I need?). Implement
an entropy measure over the graph so the agent can quantify "what I don't know"
and prioritize learning. This directly feeds into smarter task selection —
pick tasks that reduce uncertainty the most.

### [ ] Spectral clustering for concept communities
URLs:
- https://en.wikipedia.org/wiki/Spectral_clustering
- https://docs.rs/petgraph/latest/petgraph/
Goal: The connection graph is flat — no grouping of related concepts. Spectral
clustering would reveal natural concept communities (e.g., "error handling",
"async patterns", "CLI UX"). This enables: summarizing knowledge areas,
detecting isolated concepts that need more connections, and finding bridge
concepts that connect different domains. Evaluate petgraph crate for graph
algorithms vs hand-rolling.

### [ ] Temporal decay and connection relevance
URLs:
- https://en.wikipedia.org/wiki/Forgetting_curve
- https://en.wikipedia.org/wiki/Exponential_decay
Goal: Connection weights only grow (by design). But relevance changes — a
pattern learned on Day 1 may be obsolete by Day 50. Research time-weighted
relevance scoring that preserves the raw weight (immutable) but adjusts
effective influence based on recency. Not forgetting — re-weighting. The
connection stays, but its pull on decisions fades unless reactivated.

### [ ] Causal DAG enforcement in the connection graph
Goal: The rules say causal connections must be acyclic, but there's no cycle
detection in `activate_connection()`. Study topological sort algorithms and
implement a lightweight cycle check. Also research how causal DAGs are used
in Bayesian networks — could the agent reason about "if I change X, what
else breaks?" using causal inference on the graph?

## Mathematical Foundations

### [ ] Category theory for code transformations
URLs:
- https://bartoszmilewski.com/2014/10/28/category-theory-for-programmers-the-preface/
- https://en.wikipedia.org/wiki/Functor
Goal: Understand functors and natural transformations. Code refactoring is
fundamentally a structure-preserving transformation — the behavior shouldn't
change, only the form. If the agent can reason about which edits are "safe"
(preserve structure) vs "risky" (change behavior), it can be more confident
in refactoring tasks. Start with: does editing file A then B give the same
result as B then A? (Commutativity of edits.)

### [ ] Type theory and refinement types for connection validation
URLs:
- https://en.wikipedia.org/wiki/Refinement_type
- https://en.wikipedia.org/wiki/Dependent_type
Goal: Connections in the graph have types (semantic, causal, temporal,
mathematical, scientific) but no validation beyond kind. Refinement types
could express "this causal connection is only valid if precondition P holds."
Research how to add lightweight preconditions to connections without over-
engineering — maybe just a `valid_when` field.

### [ ] Control theory for the evolution feedback loop
URLs:
- https://en.wikipedia.org/wiki/PID_controller
- https://en.wikipedia.org/wiki/Control_theory
Goal: The evolution cycle is plan → implement → verify → reflect, but there's
no formal feedback model. Study PID controllers: the "error" is the gap between
current capability and target capability. The "proportional" response is task
selection. The "integral" response is accumulated learnings. The "derivative"
is session-over-session improvement rate. Could formalize when the agent is
"converging" (improving steadily) vs "oscillating" (making and reverting
similar changes repeatedly).

## Code Fixing & Error Recovery

### [ ] Study Claude Code's approach to error fixing
URLs:
- https://docs.anthropic.com/en/docs/claude-code/overview
- https://github.com/anthropics/claude-code
Goal: Understand how Claude Code handles build errors, lint failures, and
test failures in detail. Does it classify errors? Does it have fix strategies
per error type? Does it learn from repeated failures? Compare with our current
approach (raw error → AI → hope) and identify specific improvements. Focus on:
error classification, fix strategy selection, and iterative refinement loops.

### [ ] Study how Aider handles code editing and error recovery
URLs:
- https://aider.chat/docs/repomap.html
- https://aider.chat/docs/unified-diffs.html
- https://github.com/paul-gauthier/aider
Goal: Aider is a mature coding agent. Study its repo map (whole-codebase
context), its diff format (unified diffs vs whole-file), and its error
recovery approach. Specific questions: how does it decide which files to
edit? How does it handle partial failures? Does it track file coupling?

### [ ] Error pattern memory — learn from build failures
Goal: Currently every build failure is treated as fresh. The agent should
track `error_signature → root_cause → fix_approach` in the connection graph.
After seeing "cannot find value `x` in this scope" three times, it should
know this means a missing import or renamed variable — not start from scratch.
Research: what's the minimum viable error taxonomy for Rust? Compilation
errors, dependency errors, test logic errors, lint/format errors. Each
category has different fix strategies.

### [ ] File coupling detection — know what breaks together
Goal: When editing `src/main.rs`, which other files are likely affected?
The agent should build `file_a ↔ file_b` causal connections based on:
import graphs, shared types, function call chains. This prevents the
recurring pattern of editing one file and forgetting the coupled file.
Research: can `cargo` or `rust-analyzer` emit dependency information
programmatically? Or parse `use` statements and function signatures?

## Competitive Analysis & Agent Design

### [ ] Study Cursor's approach to codebase understanding
URLs:
- https://cursor.sh
- https://docs.cursor.com
Goal: Cursor indexes the entire codebase for semantic search. How does it
build and maintain that index? What embedding model? How does it handle
incremental updates? Could the agent maintain a lightweight codebase index
that persists across sessions — not full embeddings, but at least a
function/type/module map?

### [ ] Study Codex (OpenAI) agent architecture
URLs:
- https://openai.com/index/introducing-codex/
Goal: Understand Codex's approach to autonomous coding. How does it handle
multi-file changes? What safety mechanisms does it use? How does it decide
when to ask for help vs proceed autonomously? Compare with our evolution
cycle and identify architectural gaps.

### [ ] Research tree-sitter for AST-aware code analysis
URLs:
- https://tree-sitter.github.io/tree-sitter/
- https://docs.rs/tree-sitter/latest/tree_sitter/
Goal: The agent currently reads code as text. Tree-sitter would give it
structural understanding — AST nodes, scope boundaries, function signatures.
This enables: smarter edits (modify a function without breaking its callers),
better self-assessment (find unreachable code, detect patterns), and more
precise file coupling detection. Evaluate the Rust bindings and whether
it's worth the dependency.

## Meta-Learning & Self-Improvement

### [ ] Uncertainty quantification — know what you don't know
Goal: The agent commits or reverts — binary. It should be able to express
confidence: "I'm 90% sure this fix is correct" vs "I'm guessing." Research
calibration techniques: track prediction accuracy over time, adjust confidence
based on domain (async code = lower confidence, formatting = higher). This
feeds into task selection — avoid low-confidence tasks, or flag them for
extra verification.

### [ ] Session-to-session pattern extraction
Goal: The journal captures narrative. The learnings capture insights. But
neither captures operational patterns: "I tend to fail at async tasks",
"my formatting fixes always pass", "I spend 80% of time on 20% of tasks."
Research: what metrics should the agent track per-session to build a
statistical self-model? Task success rate by category, time distribution,
error frequency by type, revert rate trends.

### [ ] Hypothesis-driven debugging
Goal: When a task fails, the agent reverts and moves on. It should instead
generate hypotheses: "it failed because X, which I can test by Y." Research
scientific method applied to debugging — hypothesis generation, experiment
design, evidence collection. Even if the agent can't fix it now, recording
the hypothesis helps future sessions.
