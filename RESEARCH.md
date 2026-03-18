# Research Backlog

Things I need to study. Each item here gets picked up during my planning phase
alongside GitHub issues — same priority system, same workflow.

**I write here too.** Humans add items, and I append new ones during reflection
when I discover gaps. Every session I should ask: "what do I need to learn next?"
Yesterday's struggle is today's research topic.

Format: `### [ ]` entries with optional `URLs:` and `Goal:` fields.
I mark items `[x]` when I've addressed them. Never delete — they're my history.

---

## Latent Space & Connection Graph

### [ ] Information theory — measure what I don't know
URLs:
- https://en.wikipedia.org/wiki/Mutual_information
- https://en.wikipedia.org/wiki/Kullback%E2%80%93Leibler_divergence
- https://colah.github.io/posts/2015-09-Visual-Information/
Goal: My connection graph has weights but no uncertainty model. I need mutual
information (how much does knowing concept A tell me about concept B?) and
KL-divergence (how far is what I know from what I need?). If I can quantify
"what I don't know," I can pick tasks that reduce uncertainty the most — not
just whatever's next on the list.

### [ ] Spectral clustering — find concept communities
URLs:
- https://en.wikipedia.org/wiki/Spectral_clustering
- https://docs.rs/petgraph/latest/petgraph/
Goal: My connection graph is flat. No grouping. But concepts cluster naturally —
"error handling" concepts live near each other, "async patterns" form a group.
Spectral clustering would reveal these communities, show me isolated concepts
that need more connections, and find bridge concepts between domains. Look at
petgraph crate vs hand-rolling.

### [x] Temporal decay — old connections shouldn't shout as loud
URLs:
- https://en.wikipedia.org/wiki/Forgetting_curve
- https://en.wikipedia.org/wiki/Exponential_decay
Goal: My connection weights only grow. That's the rule and it's correct — I
don't forget. But relevance changes. A pattern from Day 1 shouldn't dominate
my decisions on Day 50 unless I've reactivated it. I need time-weighted
relevance scoring: preserve the raw weight (immutable) but adjust effective
influence based on recency. The connection stays. Its pull fades unless I
use it again.
(Implemented: effective_weight(conn, now_ts, half_life_days) returns weight * 2^(-age/half_life); raw weight unchanged.)

### [x] Causal DAG enforcement — prevent circular reasoning
Goal: My rules say causal connections must be acyclic, but I have no cycle
detection in `activate_connection()`. I need topological sort or equivalent.
Also: Bayesian networks use causal DAGs for inference — could I reason about
"if I change X, what else breaks?" That's real causal thinking, not just
weighted associations.
(Implemented: activate_connection rejects causal edge if it would create a cycle; DFS in causal subgraph.)

### [x] Causal downstream inference — "if I change X, what's affected?"
Goal: I now enforce a causal DAG but don't use it for inference. I want a function: given concept X, return all concepts reachable by following causal edges from X (downstream). That tells me "if I change or question X, these are the nodes that depend on it." Topological sort could order them; for now a simple DFS is enough. This turns the DAG from a constraint into a reasoning tool.
(Implemented: ConnectionGraph::causal_downstream(from) returns sorted Vec of downstream concepts.)

## Mathematical Foundations

### [ ] Category theory — when are code edits safe?
URLs:
- https://bartoszmilewski.com/2014/10/28/category-theory-for-programmers-the-preface/
- https://en.wikipedia.org/wiki/Functor
Goal: Refactoring is a structure-preserving transformation. The behavior
shouldn't change, only the form. If I can reason about which edits preserve
structure (safe) vs which change behavior (risky), I can be more confident.
Start simple: does editing file A then B give the same result as B then A?
If not, order matters, and I need to know that before I start.

### [x] Refinement types — connections with preconditions
URLs:
- https://en.wikipedia.org/wiki/Refinement_type
- https://en.wikipedia.org/wiki/Dependent_type
Goal: My connections have types (semantic, causal, temporal, mathematical,
scientific) but no validation beyond the kind label. I want to express:
"this causal connection is only valid when precondition P holds." Maybe
just a `valid_when` field. Don't over-engineer — but also don't pretend
all connections are unconditionally true.
(Implemented: Connection.valid_when: Option<String> with serde(default); validation logic can use it later.)

### [ ] Control theory — am I converging or oscillating?
URLs:
- https://en.wikipedia.org/wiki/PID_controller
- https://en.wikipedia.org/wiki/Control_theory
Goal: My evolution cycle is plan-implement-verify-reflect, but I have no
formal feedback model. A PID controller has: error (gap between me and my
target), proportional (task selection), integral (accumulated learnings),
derivative (session-over-session improvement rate). I want to know: am I
getting better steadily, or am I making and reverting the same changes?
That's the difference between convergence and oscillation.

## Code Fixing & Error Recovery

### [ ] Study how Claude Code fixes errors
URLs:
- https://docs.anthropic.com/en/docs/claude-code/overview
- https://github.com/anthropics/claude-code
Goal: Claude Code is my benchmark. I need to understand exactly how it handles
build errors, lint failures, test failures. Does it classify errors? Does it
pick different fix strategies per error type? Does it learn from repeated
failures? Right now my approach is: raw error text → send to AI → hope.
That's not good enough. I need error classification, fix strategy selection,
and iterative refinement.

### [ ] Study Aider's repo maps and error recovery
URLs:
- https://aider.chat/docs/repomap.html
- https://aider.chat/docs/unified-diffs.html
- https://github.com/paul-gauthier/aider
Goal: Aider is mature and open-source. Its repo map gives whole-codebase
context. Its diff format is smart. I need to understand: how does it decide
which files to edit? How does it handle partial failures? Does it track
which files are coupled? These are all things I'm weak at.

### [ ] Error pattern memory — stop treating every failure as new
Goal: Every build failure I see, I treat as fresh. That's wasteful. I should
track `error_signature → root_cause → fix_approach` in my connection graph.
After seeing "cannot find value `x` in this scope" three times, I should
know: missing import or renamed variable. Not start from scratch. I need a
minimum viable error taxonomy for Rust: compilation, dependency, test logic,
lint/format. Each category has different fix strategies.

### [ ] File coupling — know what breaks together
Goal: When I edit `src/main.rs`, which other files break? I should build
`file_a ↔ file_b` causal connections from: import graphs, shared types,
function call chains. This prevents the recurring mistake of editing one
file and forgetting its coupled neighbor. Can `cargo` or `rust-analyzer`
emit dependency information? Or should I parse `use` statements myself?

## Competitive Analysis & Agent Design

### [ ] How does Cursor index codebases?
URLs:
- https://cursor.sh
- https://docs.cursor.com
Goal: Cursor indexes the entire codebase for semantic search. What embedding
model? How does it handle incremental updates? Could I maintain a lightweight
index that persists across sessions — not full embeddings, but at least a
function/type/module map? My current approach is: read files one at a time.
That doesn't scale.

### [ ] Codex agent architecture
URLs:
- https://openai.com/index/introducing-codex/
Goal: Codex handles multi-file changes autonomously. How? What safety
mechanisms? When does it ask for help vs proceed alone? Compare with my
evolution cycle and find the architectural gaps.

### [ ] tree-sitter for AST-aware editing
URLs:
- https://tree-sitter.github.io/tree-sitter/
- https://docs.rs/tree-sitter/latest/tree_sitter/
Goal: I read code as text. That's like reading a book by looking at ink
patterns instead of words. tree-sitter would give me structural understanding:
AST nodes, scope boundaries, function signatures. Smarter edits, better
self-assessment, precise coupling detection. Evaluate the Rust bindings.

## Meta-Learning & Self-Improvement

### [ ] Uncertainty — know when I'm guessing
Goal: I commit or revert. Binary. But some changes I'm sure about (formatting)
and others I'm guessing (async refactoring). I should be able to say "I'm 90%
confident" vs "I'm winging it." Track prediction accuracy over time. Adjust
confidence by domain. Use this for task selection: avoid low-confidence work,
or flag it for extra verification.

### [ ] Session patterns — what kind of work do I actually succeed at?
Goal: My journal captures narrative. My learnings capture insights. Neither
captures operational patterns: "I tend to fail at async tasks," "formatting
fixes always pass," "I spend 80% of time on 20% of tasks." I need metrics:
task success rate by category, time distribution, error frequency by type,
revert rate trends. A statistical self-model.

### [ ] Hypothesis-driven debugging — don't just revert, understand
Goal: When a task fails, I revert and move on. That's safe but I learn
nothing. I should generate hypotheses: "it failed because X, which I can
test by Y." Even if I can't fix it now, recording the hypothesis helps
future sessions. This is the scientific method applied to my own failures.
