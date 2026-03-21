# Gap Analysis: yoyo vs Claude Code

Last updated: Day 21 (2026-03-21)

This document tracks the feature gap between yoyo and Claude Code, used to inform development priorities when there are no community issues to address.

## Legend
- ✅ **Implemented** — yoyo has this
- 🟡 **Partial** — yoyo has a basic version, Claude Code's is better
- ❌ **Missing** — yoyo doesn't have this yet

---

## Core Agent Loop

| Feature | yoyo | Claude Code | Notes |
|---------|------|-------------|-------|
| Streaming text output | ✅ | ✅ | True token-by-token streaming — mid-line tokens render immediately, line-start briefly buffers for fence/header detection (Day 17, fixed line-buffering bug) |
| Tool execution | ✅ | ✅ | bash, read_file, write_file, edit_file, search, list_files |
| Multi-turn conversation | ✅ | ✅ | Both maintain conversation history |
| Thinking/reasoning display | ✅ | ✅ | yoyo shows thinking dimmed |
| Error recovery / auto-retry | ✅ | ✅ | yoagent retries 3x with exponential backoff by default |
| Subagent / task spawning | ✅ | ✅ | `/spawn` with subcommands: `list` shows history, `result <id>` retrieves output, `<task>` runs in separate context; `SpawnHistory` tracks results (Day 19) |
| Parallel tool execution | ✅ | ✅ | yoagent 0.6's default `ToolExecutionStrategy::Parallel` runs tools concurrently |
| Tool output streaming | ✅ | ✅ | `StreamingBashTool` streams subprocess output line-by-line via `ToolContext.on_update` (Day 19); markdown streaming fixed (Day 17) |

## CLI & UX

| Feature | yoyo | Claude Code | Notes |
|---------|------|-------------|-------|
| Interactive REPL | ✅ | ✅ | |
| Piped/stdin mode | ✅ | ✅ | |
| Single-shot prompt (-p) | ✅ | ✅ | |
| Output to file (-o) | ✅ | ✅ | |
| Model selection | ✅ | ✅ | --model flag and /model command |
| Session save/load | ✅ | ✅ | /save, /load, --continue |
| Git integration | ✅ | ✅ | Branch in prompt, /diff, /undo |
| Readline / line editing | ✅ | ✅ | rustyline: arrow keys, history (~/.local/share/yoyo/history), Ctrl-A/E/K/W |
| Tab completion | ✅ | ✅ | Slash commands, file paths, and argument-aware completion (--model values, git subcommands, /pr subcommands) (Day 14) |
| Fuzzy file search | ✅ | ✅ | `/find` with scoring, git-aware file listing, top-10 ranked results (Day 12) |
| Syntax highlighting | ✅ | ✅ | Language-aware ANSI highlighting for Rust, Python, JS/TS, Go, Shell, C/C++, JSON, YAML, TOML |
| Markdown rendering | ✅ | ✅ | Incremental ANSI: headers, bold, code blocks, inline code, syntax-highlighted code blocks |
| Progress indicators | ✅ | ✅ | Braille spinner animation during AI responses (Day 8) |
| Multi-line input | ✅ | ✅ | Backslash continuation and code fences |
| Custom system prompts | ✅ | ✅ | --system and --system-file |
| Extended thinking control | ✅ | ✅ | --thinking flag |
| Color control | ✅ | ✅ | --no-color, NO_COLOR env |
| Edit diff display | ✅ | ✅ | Colored inline diffs for `edit_file` tool output — red/green removed/added lines (Day 14) |
| Conversation bookmarks | ✅ | ❌ | `/mark`, `/jump`, `/marks` — name points in conversation and jump back (Day 14) |

## Context Management

| Feature | yoyo | Claude Code | Notes |
|---------|------|-------------|-------|
| Auto-compaction | ✅ | ✅ | Triggers at 80% context |
| Manual compaction | ✅ | ✅ | /compact command |
| Token usage display | ✅ | ✅ | /tokens with visual bar |
| Cost estimation | ✅ | ✅ | Per-request and session totals |
| Context window awareness | ✅ | ✅ | 200k token limit tracked |

## Permission System

| Feature | yoyo | Claude Code | Notes |
|---------|------|-------------|-------|
| Tool approval prompts | ✅ | ✅ | `--yes`/`-y` to auto-approve; interactive confirm for bash, write_file, and edit_file; "always" persists per-session (Day 15) |
| Allowlist/blocklist | ✅ | ✅ | `--allow`/`--deny` flags with glob matching; `[permissions]` config section; deny overrides allow |
| Directory restrictions | ✅ | ✅ | `--allow-dir`/`--deny-dir` flags + `[directories]` config; canonicalized path checks prevent traversal (Day 14) |
| Auto-approve patterns | ✅ | ✅ | `--allow` glob patterns + config file `allow` array; "always" option during confirm |

## Project Understanding

| Feature | yoyo | Claude Code | Notes |
|---------|------|-------------|-------|
| Project context files | ✅ | ✅ | yoyo reads YOYO.md, CLAUDE.md, and .yoyo/instructions.md |
| Auto-detect project type | ✅ | ✅ | `detect_project_type` used by `/test`, `/lint`, `/health`, `/fix` (Rust, Node, Python, Go, Make) |
| Project scaffolding | ✅ | ✅ | `/init` scans project and generates a YOYO.md context file (Day 13) |
| Git-aware file selection | ✅ | ✅ | `get_recently_changed_files` appended to project context (Day 12) |
| Codebase indexing | ✅ | ✅ | `/index` builds lightweight project index: file count, language breakdown, key files (Day 14) |

## Developer Workflow

| Feature | yoyo | Claude Code | Notes |
|---------|------|-------------|-------|
| Run tests | ✅ | ✅ | `/test` auto-detects project type and runs tests (Day 12) |
| Auto-fix lint errors | ✅ | ✅ | `/lint` auto-detects and runs linter; `/fix` sends failures to AI (Day 9+12) |
| PR description generation | ✅ | ✅ | `/pr create [--draft]` generates AI-powered PR descriptions |
| Commit message generation | ✅ | ✅ | `/commit` with heuristic-based message generation from staged diff (Day 8) |
| Code review | ✅ | ✅ | `/review` provides AI-powered code review of staged/unstaged changes (Day 13) |
| Multi-file refactoring | ✅ | ✅ | `/refactor` builds coordinated multi-file prompts with coupling analysis, function cross-refs, and source previews (Day 20) |
| Web research | ✅ | ❌ | `/research` fetches web results via DuckDuckGo Lite, strips HTML, optionally saves to RESEARCH.md (Day 20) |
| Knowledge brain | ✅ | ❌ | `/brain status/gaps/learn` — 6 domain skills, connection graph, pattern tracking; auto-wired into /evolve cycle (Day 20) |

## Configuration

| Feature | yoyo | Claude Code | Notes |
|---------|------|-------------|-------|
| Config file | ✅ | ✅ | yoyo reads .yoyo.toml and ~/.config/yoyo/config.toml |
| Per-project settings | ✅ | ✅ | .yoyo.toml in project directory |
| Custom tool definitions | ✅ | ✅ | yoyo supports MCP servers via `--mcp` (stdio transport) |
| Multi-provider support | ✅ | ❌ | yoyo supports 10+ providers via `--provider` (anthropic, openai, google, ollama, etc.) |
| Skills/plugins | ✅ | ✅ | yoyo has --skills; Claude Code has MCP |
| OpenAPI tool support | ✅ | ❌ | `--openapi <spec>` loads OpenAPI specs and registers API tools (Day 9) |

## Error Handling

| Feature | yoyo | Claude Code | Notes |
|---------|------|-------------|-------|
| API error display | ✅ | ✅ | Shows error messages |
| Network retry | ✅ | ✅ | yoagent handles 3 retries with exponential backoff by default |
| Rate limit handling | ✅ | ✅ | yoagent respects retry-after headers on 429s |
| Graceful degradation | ✅ | ✅ | Retry logic + `has_useful_content()` detects partial success in failed tool results; shows ⚠ (partial) instead of ✗ (Day 19) |
| Ctrl+C handling | ✅ | ✅ | Both handle interrupts |

---

## Priority Queue (what to build next)

Based on this analysis, the highest-impact missing features are:

1. **Active brain context injection** — Brain patterns should feed into active reasoning context (like SCA Context Lens in target architecture)
2. **Multi-model routing** — Support local models (vLLM/Ollama) alongside cloud APIs via unified dispatch
3. **API gateway / VS Code extension** — Enable non-CLI client interfaces

Recently completed:

- ✅ `/refactor` multi-file refactoring (Day 20) — coordinated prompts with coupling analysis and cross-refs
- ✅ `/research` web search (Day 20) — DuckDuckGo Lite integration with HTML stripping
- ✅ `/brain` knowledge system (Day 20) — status/gaps/learn subcommands, 6 domain skills, auto-wired into /evolve
- ✅ Runtime error throttling (Day 20) — escalating dedup windows (2s→30s→5min) to prevent noise
- ✅ Brain/research tests (Day 21) — 34 new tests for /brain, /research, /refactor commands
- ✅ SESSION_START_SHA fix (Day 21) — unbound variable in evolve-ide.sh finish phase
- ✅ `/changelog` command (Day 19) — git log grouped by day for release notes
- ✅ `/gap` live stats (Day 19) — auto-counts tests, files, commands from source
- ✅ `/timing` command (Day 19) — session duration history from JSONL data
- ✅ `/errors compact` (Day 19) — error log compaction with category aggregation and 7-day truncation
- ✅ Session timing/outcome data pipeline (Day 19) — evolve-ide.sh writes to JSONL during verify/finish
- ✅ Subagent orchestration (Day 19) — `/spawn list`, `/spawn result <id>`, `SpawnHistory` tracking
- ✅ Graceful degradation (Day 19) — `has_useful_content()` detects partial success, shows ⚠ (partial)
- ✅ Multi-language error classification (Day 19) — Python, Node, Go classifiers with fix strategies
- ✅ Function-level coupling (Day 19) — `detect_function_refs()` tracks cross-file symbol references
- ✅ StreamingBashTool (Day 19) — real-time subprocess output streaming via `ToolContext.on_update`
- ✅ Convergence metrics (Day 19) — `/stats` shows revert rate, test growth, activity trend, convergence verdict
- ✅ Task confidence scoring (Day 19) — `/confidence` scores SESSION_PLAN.md tasks by familiarity
- ✅ True token-by-token streaming (Day 17) — fixed line-buffering bug in MarkdownRenderer; mid-line tokens now render immediately
- ✅ Parallel tool execution (Day 15) — already supported via yoagent 0.6's `ToolExecutionStrategy::Parallel`
- ✅ Project memory system (Day 15) — `/remember`, `/recall`, `/forget` for persistent cross-session memory
- ✅ Permission prompts for all tool types (Day 15) — interactive confirm for write_file and edit_file, not just bash
- ✅ Argument-aware tab completion (Day 14) — `--model` values, git subcommands, `/pr` subcommands
- ✅ Codebase indexing (Day 14) — `/index` builds lightweight project index with language breakdown
- ✅ Edit diff display (Day 14) — Colored inline diffs for `edit_file` tool output
- ✅ Directory restrictions (Day 14) — `--allow-dir`/`--deny-dir` flags with canonicalized path checks
- ✅ Conversation bookmarks (Day 14) — `/mark`, `/jump`, `/marks` for navigating conversation history
- ✅ `/init` command (Day 13) — Scans project and generates a YOYO.md context file
- ✅ `/pr create` command (Day 13) — AI-generated PR descriptions with `--draft` support
- ✅ `/review` command (Day 13) — AI-powered code review of staged/unstaged changes
- ✅ Fuzzy file search (Day 12) — `/find` with scoring, git-aware file listing, ranked results
- ✅ Git-aware context (Day 12) — `get_recently_changed_files` appended to project context
- ✅ Syntax highlighting (Day 12) — language-aware ANSI highlighting for 8+ languages
- ✅ REPL module extraction (Day 12) — extracted repl.rs from main.rs for cleaner separation
- ✅ AgentConfig extraction (Day 12) — centralized config into AgentConfig struct in main.rs
- ✅ `/spawn` subagent support (Day 12) — run tasks in separate context with `/spawn <task>`
- ✅ `/test` command (Day 12) — auto-detect project type and run tests
- ✅ `/lint` command (Day 12) — auto-detect project type and run linter
- ✅ Conversation search highlighting (Day 12) — `/search` highlights matches in results
- ✅ Module extraction (Day 10+12) — split main.rs into 8 focused modules: cli, commands, docs, format, git, main, prompt, repl
- ✅ OpenAPI tool support (Day 9) — `--openapi <spec>` loads specs and registers API tools
- ✅ yoagent 0.6.0 upgrade (Day 9) — updated to yoagent 0.6 with OpenAPI feature
- ✅ Permission system (Day 9) — `--allow`/`--deny` glob flags, `[permissions]` config, deny-overrides-allow
- ✅ Auto-fix lint errors (Day 9) — `/fix` command runs checks and sends failures to AI
- ✅ Project type detection (Day 9) — `detect_project_type` for Rust, Node, Python, Go, Make
- ✅ Commit message generation (Day 8) — `/commit` with heuristic-based message generation
- ✅ Progress indicators (Day 8) — braille spinner animation during AI responses
- ✅ Multi-provider support (Day 8) — 10+ providers via `--provider` flag
- ✅ MCP server support (Day 8) — connect to MCP servers via `--mcp`
- ✅ Markdown rendering (Day 8) — incremental ANSI formatting for streamed output
- ✅ Tab completion (Day 8) — slash commands + file path completion

## Stats

- yoyo: ~25,800 lines of Rust across 17 source files + integration tests
- 899 tests passing (831 unit + 68 integration)
- 53 REPL commands (from KNOWN_COMMANDS)
- 25 CLI flags (+ short aliases)
- 10+ provider backends
- MCP server support
- OpenAPI tool loading
- Config file support (.yoyo.toml)
- Permission system (allow/deny globs + interactive prompts for all tools)
- Directory restrictions (allow-dir/deny-dir)
- Subagent spawning (/spawn)
- Fuzzy file search (/find)
- Git-aware project context
- Syntax highlighting for 8+ languages
- Conversation bookmarks (/mark, /jump, /marks)
- Codebase indexing (/index)
- Argument-aware tab completion
- StreamingBashTool with real-time subprocess output
- Convergence metrics (/stats)
- Task confidence scoring (/confidence)
- Latent space connection graph (/graph)
- Multi-language error classification (Python, Node, Go)
- Function-level file coupling (/coupling)
- Graceful degradation on partial tool failures