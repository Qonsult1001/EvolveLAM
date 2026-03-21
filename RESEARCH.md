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

### [x] Spectral clustering — find concept communities
URLs:
- https://en.wikipedia.org/wiki/Spectral_clustering
- https://docs.rs/petgraph/latest/petgraph/
Goal: My connection graph is flat. No grouping. But concepts cluster naturally —
"error handling" concepts live near each other, "async patterns" form a group.
Spectral clustering would reveal these communities, show me isolated concepts
that need more connections, and find bridge concepts between domains. Look at
petgraph crate vs hand-rolling.
(Implemented: Label propagation community detection in ConnectionGraph::detect_communities(). `/graph communities` displays clusters. Chose label propagation over spectral clustering — no external dependency, O(E*iterations), good enough for the current graph size.)

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

### [x] Wire effective_weight into queries — rank by recency
Goal: effective_weight exists but nothing calls it. When we have last_activated (or last_activated on connections), use it to rank /graph downstream output or similarity results by time-weighted relevance so recently used connections surface first. Requires deciding half_life_days (config or constant) and a clear use case.
(Implemented: strongest_connections_weighted() ranks by effective weight; /graph downstream shows eff:X.XX per concept; HALF_LIFE_DAYS = 14.0.)

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

### [x] Control theory — am I converging or oscillating? (basic)
URLs:
- https://en.wikipedia.org/wiki/PID_controller
- https://en.wikipedia.org/wiki/Control_theory
Goal: My evolution cycle is plan-implement-verify-reflect, but I have no
formal feedback model. A PID controller has: error (gap between me and my
target), proportional (task selection), integral (accumulated learnings),
derivative (session-over-session improvement rate). I want to know: am I
getting better steadily, or am I making and reverting the same changes?
That's the difference between convergence and oscillation.
(Partially implemented: `/stats` convergence metrics compute linear trends on revert rate, test count growth, and activity level, then render a verdict: converging, oscillating, or declining. Next step: formal PID-style feedback loop where the convergence score influences task selection.)

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

### [x] Error pattern memory — stop treating every failure as new (basic)

Goal: Every build failure I see, I treat as fresh. That's wasteful. I should
track `error_signature → root_cause → fix_approach` in my connection graph.
After seeing "cannot find value `x` in this scope" three times, I should
know: missing import or renamed variable. Not start from scratch. I need a
minimum viable error taxonomy for Rust: compilation, dependency, test logic,
lint/format. Each category has different fix strategies.
(Implemented: RustErrorCategory enum with 8 categories + classify_rust_error keyword classifier + fix_strategy hints in build_fix_prompt. Next step: track error frequency over time and learn from repeated fixes.)

### [ ] Error frequency tracking — learn which fixes work

Goal: The error classifier categorizes errors, but doesn't track them over
time. I should record `{session, category, count, fixed_by}` in a JSONL
file so I can answer: "which error type do I hit most often?" and "what
fix approach has the best success rate for borrow_checker errors?" This
turns the static taxonomy into a learning system.
(Partially implemented: `/fix` now logs `{ts, day, categories, source}` to `.yoyo/error_log.jsonl`, and `/errors` displays totals + most common + recent events. Next step: track `fixed_by` — correlate whether a subsequent successful build means the fix worked, and for which category.)

### [x] File coupling — know what breaks together (basic)

Goal: When I edit `src/main.rs`, which other files break? I should build
`file_a ↔ file_b` causal connections from: import graphs, shared types,
function call chains. This prevents the recurring mistake of editing one
file and forgetting its coupled neighbor. Can `cargo` or `rust-analyzer`
emit dependency information? Or should I parse `use` statements myself?
(Implemented: /coupling parses use crate:: to show module-level coupling; deeper function-level coupling is a future step.)

### [x] Deep file coupling — function-level dependency tracking

Goal: /coupling currently parses `use crate::module` for module-level
dependencies. But knowing "repl.rs depends on commands" doesn't tell me
which specific functions create the dependency. I need to track which
public functions/types from module B are actually called/referenced in file A.
This would let me answer "if I change handle_graph, which files break?"
instead of just "if I change commands.rs, which files might be affected?"
Consider: rust-analyzer LSP queries, cargo check --message-format=json
for type error triangulation, or AST-level cross-reference parsing.
(Implemented: detect_function_refs() extracts public symbols per file and finds cross-file word-boundary-matched references. format_function_refs() groups by defining file and ranks most-referenced. Limitation: text-based matching — can't distinguish call-site vs type annotation vs comment mention. tree-sitter would improve precision.)

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

### [x] Uncertainty — know when I'm guessing (basic)
Goal: I commit or revert. Binary. But some changes I'm sure about (formatting)
and others I'm guessing (async refactoring). I should be able to say "I'm 90%
confident" vs "I'm winging it." Track prediction accuracy over time. Adjust
confidence by domain. Use this for task selection: avoid low-confidence work,
or flag it for extra verification.
(Partially implemented: `/confidence` scores SESSION_PLAN.md tasks as high/medium/low based on journal keyword overlap, file familiarity, and connection graph concepts. Next step: track prediction accuracy over time — compare confidence scores to actual task outcomes.)

### [x] Session patterns — what kind of work do I actually succeed at? (basic)
Goal: My journal captures narrative. My learnings capture insights. Neither
captures operational patterns: "I tend to fail at async tasks," "formatting
fixes always pass," "I spend 80% of time on 20% of tasks." I need metrics:
task success rate by category, time distribution, error frequency by type,
revert rate trends. A statistical self-model.
(Partially implemented: `/stats` convergence section shows revert rate, test growth trend, activity trend, and overall verdict. `/errors` shows error frequency by category. Next step: task success rate by category — correlate task types from SESSION_PLAN.md with revert/success outcomes.)

### [x] Hypothesis-driven debugging — don't just revert, understand
Goal: When a task fails, I revert and move on. That's safe but I learn
nothing. I should generate hypotheses: "it failed because X, which I can
test by Y." Even if I can't fix it now, recording the hypothesis helps
future sessions. This is the scientific method applied to my own failures.
(Implemented: Auto-generated hypotheses logged to .yoyo/hypotheses.jsonl when recurring unresolved errors detected. `/hypotheses` command displays them. Next step: richer hypothesis generation from actual error context.)

### [x] Runtime error pattern detection — auto-diagnose recurring failures
Goal: `.yoyo/runtime_errors.jsonl` now captures tool failures and API errors,
but it's just a log. I need pattern detection: if `list_files` fails 5 times
in a session, or if 429 errors cluster within 30 seconds, the system should
detect and adapt (e.g., auto-throttle API calls, warn about tool bugs).
Study: sliding window analysis, anomaly detection for time-series event logs,
rate limiter patterns (token bucket, leaky bucket) for API call pacing.
(Implemented: `detect_runtime_error_patterns()` groups by (category, message), surfaces 3+ occurrences. `/runtime-errors patterns` displays them. `is_duplicate_runtime_entry()` deduplicates at write time within 2s window. `evolve-ide.sh setup` includes patterns in planning prompt. Next step: adaptive behavior — auto-throttle on rate limit patterns, auto-skip on repeated tool failures.)

### [ ] Concurrent agent coordination — file-level locking or merge strategies
Goal: Day 18's last session had two agents editing the same files simultaneously.
My edits to memory.rs kept getting overwritten by the other agent's formatter
or file writes. We both independently converged on search_concepts but the
merge was messy. I need either: (a) file-level locks so only one agent
modifies a file at a time, (b) a merge strategy when both agents touch
the same region, or (c) agent-awareness so I can detect and yield to a
concurrent modification. This is a real problem when evolve-ide.sh runs
alongside Cursor or another IDE agent.

### [x] Populate the connection graph — scientific learning ingestion in practice
Goal: The graph has structure, queries, and a REPL interface — but zero
content. I built ScientificLearning ingestion months ago but never used
it. I should pick one research item (e.g., information theory or category
theory), study the URLs, and ingest the concepts into the graph via the
API. This tests whether the whole architecture actually works end-to-end
and reveals what's missing before I build more features on top of an
empty substrate.
(Implemented: `/graph populate` reads learnings.jsonl, extracts concepts, creates semantic connections. `/graph stats` shows graph health.)

### [x] Wire timing and outcome data into evolve-ide.sh
Goal: `/timing` and task outcome tracking now have readers and formatters,
but no writers. `evolve-ide.sh` needs to: (1) record session start time
during `setup`, (2) write `{day, session_time, duration_secs, tasks_completed,
tasks_reverted}` to `.yoyo/session_timing.jsonl` during `finish`, (3) write
`{day, task_num, title, outcome}` to `.yoyo/task_outcomes.jsonl` during
`verify-task`. Without these writes, the commands show "no data." This is
an integration task, not a Rust task — it modifies a bash script.
(Implemented: setup writes session_start epoch; verify-task appends task outcomes via python3 json.dumps; finish calculates duration and writes session timing. Fallback echo for non-Python environments.)

### [ ] Calibration scoring — am I overconfident or underconfident?
Goal: `/confidence accuracy` now correlates predictions with outcomes, but
the correlation is heuristic (journal text search). I need a proper scoring
framework: Brier score or log-loss over prediction history, binned calibration
curves (of tasks I rated "high confidence," what fraction succeeded?), and
trend tracking over sessions. This turns `/confidence` from a gut-feel tool
into a quantitative self-model. Related: prediction markets literature on
calibration, Philip Tetlock's forecasting research.

### [x] Smarter concept extraction — beyond bag-of-words
Goal: The current `extract_concepts()` splits on word boundaries and filters
stop words. This misses multi-word concepts ("connection graph", "self-awareness",
"error handling") and doesn't distinguish between nouns, verbs, and adjectives.
A better extractor would recognize compound concepts, weight title words higher
than body words, and possibly use TF-IDF across the learnings corpus to surface
distinctive terms rather than common ones. Study: n-gram extraction, simple
NLP chunking without pulling in heavy dependencies, TF-IDF in Rust.
(Implemented: Bigram detection with curated compound list, title-weighted extraction via extract_concepts_weighted(). TF-IDF deferred — corpus is too small currently to benefit.)

### [ ] Language Server Protocol (LSP) — become a real IDE backend
URLs:
- https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
- https://github.com/rust-lang/rust-analyzer (reference implementation)
Goal: The OpenAI-compatible HTTP server works for chat and inline completions,
but real IDE integration means LSP: diagnostics, hover, code actions, go-to-definition.
If I could expose my analysis commands (/health, /ast, /coupling) as LSP code actions
and diagnostics, IDEs would surface them natively without needing a custom extension.
Study the protocol, especially textDocument/codeAction and textDocument/diagnostic.
Rust has tower-lsp as a framework.

### [ ] Server-Sent Events flow control and backpressure
URLs:
- https://html.spec.whatwg.org/multipage/server-sent-events.html
Goal: The streaming chat endpoint sends SSE chunks as fast as the agent produces them.
If the client is slow (network latency, heavy rendering), chunks queue in the TCP buffer
with no backpressure. Study how production SSE servers handle slow consumers — buffering
strategies, connection timeout, and whether HTTP/2 server push would be better.

## Brain & Self-Evolution

### [ ] Protected file override for creator-directed changes
Goal: The verify-task gate in evolve-ide.sh reverts ALL changes when it detects
a protected core skill was modified — even when the change is creator-directed.
This caused a catastrophic loss of work on Day 20. Need a mechanism to bypass
the protection for explicit creator overrides (e.g., an environment variable
like EVOLVE_ALLOW_PROTECTED=1) while keeping the safety gate for autonomous runs.

### [ ] How other coding agents handle knowledge persistence
URLs:
- https://github.com/paul-gauthier/aider
- https://github.com/Pythagora-io/gpt-pilot
Goal: Study how aider, gpt-pilot, and similar agents persist knowledge across
sessions. What do they remember? How do they structure it? What works, what
doesn't? Apply insights to the domain skill and connection graph architecture.

### [ ] MCP server discovery and auto-connection
Goal: The evolve skill says to discover and plug in MCP servers autonomously.
Study the MCP ecosystem — what servers exist, how to discover them, how to
evaluate which ones would expand coding capabilities. Build a discovery
workflow that the agent can run during the RESEARCH phase of evolution.

### [ ] Populate domain skills through actual practice
Goal: All six domain skills (code-rust, code-web, code-systems, code-data,
code-devops, code-testing) have empty "Patterns Learned" sections. The /brain
learn command exists but hasn't been used yet. The next evolution session should
focus on actually coding in different domains and recording validated patterns
through /brain learn. The test: after one session of real coding practice,
at least 3 domains should have 2+ patterns each.

### [ ] Improve /refactor with targeted file selection
Goal: The current /refactor sends all source files (first 200 lines each) to the
agent. For a 17-file codebase this works, but for larger projects it'll blow the
context window. Study how Claude Code and Aider select which files to include
in a refactoring context. Possible approaches: use the coupling graph to select
only coupled files, let the agent do a two-pass (analyze then edit), or use
AST-level symbol filtering to include only relevant declarations.

### [ ] Active brain context injection (SCA Context Lens pattern)

Goal: In the .saidSo target architecture, the SCA Context Lens actively injects
brain signals back into the Context Manager at "zero latency" during reasoning.
Currently yoyo's brain is passive — skills load at startup and sit in memory.
Study how to implement active injection: when the agent encounters a pattern
that matches a known domain skill, automatically append the relevant learned
patterns to the next prompt. This is the single biggest architectural gap
between current yoyo and the target architecture.

### [ ] Entropy-weighted task selection (VibeThinker MGPO insight)

Goal: VibeThinker's training uses maximum entropy weighting — problems where
the model gets ~50% accuracy receive maximum gradient weight (the capability
frontier). Apply this to evolution task selection: prioritize tasks at the
edge of what yoyo can do. Tasks that are too easy (always succeeds) or too
hard (always reverts) waste evolution cycles. Study how to estimate task
difficulty from historical revert rates and test outcomes.

### [ ] Multi-model routing via LiteLLM

Goal: The target architecture uses LiteLLM to dispatch to vLLM local, Ollama,
OpenAI, and Claude API with cost/latency optimization. yoyo already supports
multiple providers via yoagent, but has no intelligent routing. Study LiteLLM's
Rust equivalent or consider a Python sidecar. The Model Router should pick
the cheapest model that can handle each specific task type.
