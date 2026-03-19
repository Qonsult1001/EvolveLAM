//! Core REPL command handlers: constants, completion, help, status, tokens, cost,
//! model, provider, think, config, retry, version.

use crate::cli::{default_model_for_provider, KNOWN_PROVIDERS};
use crate::cli::{is_verbose, AUTO_COMPACT_THRESHOLD, MAX_CONTEXT_TOKENS, VERSION};
use crate::commands_session::auto_compact_if_needed;
use crate::format::*;
use crate::git::*;
use crate::prompt::*;

use yoagent::agent::Agent;
use yoagent::context::total_tokens;
use yoagent::*;

/// Known REPL command prefixes. Used to detect unknown slash commands
/// and for tab-completion in the REPL.
pub const KNOWN_COMMANDS: &[&str] = &[
    "/help",
    "/quit",
    "/exit",
    "/clear",
    "/compact",
    "/commit",
    "/cost",
    "/docs",
    "/find",
    "/fix",
    "/forget",
    "/gap",
    "/index",
    "/status",
    "/tokens",
    "/save",
    "/load",
    "/diff",
    "/undo",
    "/health",
    "/retry",
    "/history",
    "/search",
    "/model",
    "/think",
    "/config",
    "/context",
    "/init",
    "/version",
    "/run",
    "/tree",
    "/pr",
    "/git",
    "/test",
    "/lint",
    "/spawn",
    "/review",
    "/mark",
    "/jump",
    "/marks",
    "/remember",
    "/memories",
    "/graph",
    "/provider",
    "/ast",
    "/coupling",
    "/errors",
    "/stats",
    "/confidence",
    "/hypotheses",
    "/timing",
    "/changelog",
    "/blame",
    "/runtime-errors",
];

/// Well-known model names for `/model <Tab>` completion.
pub const KNOWN_MODELS: &[&str] = &[
    // Claude (current short names + dated aliases)
    "claude-opus-4-6",
    "claude-sonnet-4-6",
    "claude-haiku-4-5-20251001",
    "claude-sonnet-4-20250514",
    "claude-opus-4-20250514",
    // OpenAI
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-4.1",
    "gpt-4.1-mini",
    "o3",
    "o3-mini",
    "o4-mini",
    // Google
    "gemini-2.5-pro",
    "gemini-2.5-flash",
    // DeepSeek
    "deepseek-chat",
    "deepseek-reasoner",
];

/// Thinking level names for `/think <Tab>` completion.
pub const THINKING_LEVELS: &[&str] = &["off", "minimal", "low", "medium", "high"];

/// Git subcommand names for `/git <Tab>` completion.
pub const GIT_SUBCOMMANDS: &[&str] = &["status", "log", "add", "diff", "branch", "stash"];

/// PR subcommand names for `/pr <Tab>` completion.
pub const PR_SUBCOMMANDS: &[&str] = &["list", "view", "diff", "comment", "create", "checkout"];

/// Graph subcommand names for `/graph <Tab>` completion.
/// Spawn subcommand names for `/spawn <Tab>` completion.
pub const SPAWN_SUBCOMMANDS: &[&str] = &["list", "result"];

pub const GRAPH_SUBCOMMANDS: &[&str] = &[
    "downstream",
    "neighbors",
    "info",
    "activate",
    "search",
    "path",
    "populate",
    "stats",
    "similar",
    "communities",
];

/// Return context-aware argument completions for a given command and partial argument.
///
/// `cmd` is the slash command (e.g. "/model"), `partial_arg` is what the user has typed
/// after the command + space so far. Returns a list of candidate completions.
pub fn command_arg_completions(cmd: &str, partial_arg: &str) -> Vec<String> {
    let partial_lower = partial_arg.to_lowercase();
    match cmd {
        "/model" => filter_candidates(KNOWN_MODELS, &partial_lower),
        "/think" => filter_candidates(THINKING_LEVELS, &partial_lower),
        "/git" => filter_candidates(GIT_SUBCOMMANDS, &partial_lower),
        "/pr" => filter_candidates(PR_SUBCOMMANDS, &partial_lower),
        "/provider" => filter_candidates(KNOWN_PROVIDERS, &partial_lower),
        "/graph" => filter_candidates(GRAPH_SUBCOMMANDS, &partial_lower),
        "/spawn" => filter_candidates(SPAWN_SUBCOMMANDS, &partial_lower),
        "/save" | "/load" => list_json_files(partial_arg),
        _ => Vec::new(),
    }
}

/// Filter a list of candidates by a lowercase prefix.
fn filter_candidates(candidates: &[&str], partial_lower: &str) -> Vec<String> {
    candidates
        .iter()
        .filter(|c| c.to_lowercase().starts_with(partial_lower))
        .map(|c| c.to_string())
        .collect()
}

/// List .json files in the current directory matching a partial prefix.
fn list_json_files(partial: &str) -> Vec<String> {
    let entries = match std::fs::read_dir(".") {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    let mut matches: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".json") && name.starts_with(partial) {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    matches.sort();
    matches
}

/// Check if a slash-prefixed input is an unknown command.
/// Extracts the first word and checks against known commands.
pub fn is_unknown_command(input: &str) -> bool {
    let cmd = input.split_whitespace().next().unwrap_or(input);
    !KNOWN_COMMANDS.contains(&cmd)
}

/// Format a ThinkingLevel as a display string.
pub fn thinking_level_name(level: ThinkingLevel) -> &'static str {
    match level {
        ThinkingLevel::Off => "off",
        ThinkingLevel::Minimal => "minimal",
        ThinkingLevel::Low => "low",
        ThinkingLevel::Medium => "medium",
        ThinkingLevel::High => "high",
    }
}

// ── /help ────────────────────────────────────────────────────────────────

/// Build help text as a String so it's testable.
pub fn help_text() -> String {
    let mut out = String::new();

    // ── Session ──
    out.push_str("  ── Session ──\n");
    out.push_str("  /help              Show this help\n");
    out.push_str("  /quit, /exit       Exit yoyo\n");
    out.push_str("  /clear             Clear conversation history\n");
    out.push_str("  /compact           Compact conversation to save context space\n");
    out.push_str("  /save [path]       Save session to file (default: yoyo-session.json)\n");
    out.push_str("  /load [path]       Load session from file\n");
    out.push_str("  /retry             Re-send the last user input\n");
    out.push_str("  /status            Show session info\n");
    out.push_str("  /tokens            Show token usage and context window\n");
    out.push_str("  /cost              Show estimated session cost\n");
    out.push_str("  /config            Show all current settings\n");
    out.push_str("  /version           Show yoyo version\n");
    out.push_str("  /history           Show summary of conversation messages\n");
    out.push_str("  /search <query>    Search conversation history for matching messages\n");
    out.push_str("  /mark <name>       Bookmark current conversation state\n");
    out.push_str(
        "  /jump <name>       Restore conversation to a bookmark (discards messages after it)\n",
    );
    out.push_str("  /marks             List all saved bookmarks\n");
    out.push('\n');

    // ── Git ──
    out.push_str("  ── Git ──\n");
    out.push_str("  /git <subcmd>      Quick git: status, log, add, diff, branch, stash\n");
    out.push_str("  /diff              Show file summary, change stats, and full diff\n");
    out.push_str("  /undo              Revert all uncommitted changes (git checkout)\n");
    out.push_str("  /commit [msg]      Commit staged changes (AI-generates message if no msg)\n");
    out.push_str("  /pr [number]       List open PRs, view, diff, comment, or checkout a PR\n");
    out.push_str(
        "                     /pr create [--draft] | /pr <n> diff | /pr <n> comment <text>\n",
    );
    out.push_str(
        "  /review [path]     AI code review: staged changes (default) or a specific file\n",
    );
    out.push('\n');

    // ── Project ──
    out.push_str("  ── Project ──\n");
    out.push_str("  /context           Show loaded project context files\n");
    out.push_str("  /init              Scan project and generate a YOYO.md context file\n");
    out.push_str("  /health            Run project health checks (auto-detects project type)\n");
    out.push_str(
        "  /fix               Auto-fix build/lint errors (runs checks, sends failures to AI)\n",
    );
    out.push_str(
        "  /test              Auto-detect and run project tests (cargo test, npm test, etc.)\n",
    );
    out.push_str(
        "  /lint              Auto-detect and run project linter (clippy, eslint, ruff, etc.)\n",
    );
    out.push_str("  /run <cmd>         Run a shell command directly (no AI, no tokens)\n");
    out.push_str("  !<cmd>             Shortcut for /run\n");
    out.push_str("  /docs <crate> [item] Look up docs.rs documentation for a Rust crate\n");
    out.push_str("  /find <pattern>    Fuzzy-search project files by name\n");
    out.push_str("  /index             Build a lightweight index of project source files\n");
    out.push_str("  /tree [depth]      Show project directory tree (default depth: 3)\n");
    out.push_str("  /ast <pattern>     Search code symbols (fn, struct, trait, class, etc.)\n");
    out.push_str("  /coupling          Show file coupling map (use crate:: imports)\n");
    out.push_str("  /gap               Show gap analysis stats and update CLAUDE_CODE_GAP.md\n");
    out.push_str("  /timing            Show session duration history\n");
    out.push_str("  /changelog [N]     Git changelog grouped by day (default: 7 days)\n");
    out.push_str("  /blame <file> [L]  Git blame for a file (optionally centered on line L)\n");
    out.push_str("  /runtime-errors    Show runtime error log (tool failures, API errors)\n");
    out.push('\n');

    // ── AI ──
    out.push_str("  ── AI ──\n");
    out.push_str("  /model <name>      Switch model (preserves conversation)\n");
    out.push_str("  /provider <name>   Switch provider (resets model to provider default)\n");
    out.push_str("  /think [level]     Show or change thinking level (off/low/medium/high)\n");
    out.push_str("  /spawn <task>      Spawn a subagent to handle a task (separate context)\n");
    out.push_str(
        "  /remember <note>   Save a project-specific memory (persists across sessions)\n",
    );
    out.push_str("  /memories          List project-specific memories for this directory\n");
    out.push_str(
        "  /graph <sub> [args]  Connection graph (populate, stats, downstream, neighbors, info)\n",
    );
    out.push_str("  /forget <n>        Remove a project memory by index\n");
    out.push('\n');

    // ── Input ──
    out.push_str("  ── Input ──\n");
    out.push_str("  End a line with \\ to continue on the next line\n");
    out.push_str("  Start with ``` to enter a fenced code block\n");

    out
}

pub fn handle_help() {
    println!("{DIM}{}{RESET}", help_text());
}

// ── /version ─────────────────────────────────────────────────────────────

pub fn handle_version() {
    println!("{DIM}  yoyo v{VERSION}{RESET}\n");
}

// ── /status ──────────────────────────────────────────────────────────────

pub fn handle_status(model: &str, cwd: &str, session_total: &Usage) {
    println!("{DIM}  model:   {model}");
    if let Some(branch) = git_branch() {
        println!("  git:     {branch}");
    }
    println!("  cwd:     {cwd}");
    println!(
        "  tokens:  {} in / {} out (session total){RESET}\n",
        session_total.input, session_total.output
    );
}

// ── /tokens ──────────────────────────────────────────────────────────────

pub fn handle_tokens(agent: &Agent, session_total: &Usage, model: &str) {
    let max_context = MAX_CONTEXT_TOKENS;
    let messages = agent.messages().to_vec();
    let context_used = total_tokens(&messages) as u64;
    let bar = context_bar(context_used, max_context);

    println!("{DIM}  Context window:");
    println!("    messages:    {}", messages.len());
    println!(
        "    context:     {} / {} tokens",
        format_token_count(context_used),
        format_token_count(max_context)
    );
    println!("    {bar}");
    if context_used as f64 / max_context as f64 > 0.75 {
        println!("    {YELLOW}⚠ Context is getting full. Consider /clear or /compact.{RESET}");
    }
    println!();
    println!("  Session totals:");
    println!(
        "    input:       {} tokens",
        format_token_count(session_total.input)
    );
    println!(
        "    output:      {} tokens",
        format_token_count(session_total.output)
    );
    println!(
        "    cache read:  {} tokens",
        format_token_count(session_total.cache_read)
    );
    println!(
        "    cache write: {} tokens",
        format_token_count(session_total.cache_write)
    );
    if let Some(cost) = estimate_cost(session_total, model) {
        println!("    est. cost:   {}", format_cost(cost));
    }
    println!("{RESET}");
}

// ── /cost ────────────────────────────────────────────────────────────────

pub fn handle_cost(session_total: &Usage, model: &str) {
    if let Some(cost) = estimate_cost(session_total, model) {
        println!("{DIM}  Session cost: {}", format_cost(cost));
        println!(
            "    {} in / {} out",
            format_token_count(session_total.input),
            format_token_count(session_total.output)
        );
        if session_total.cache_read > 0 || session_total.cache_write > 0 {
            println!(
                "    cache: {} read / {} write",
                format_token_count(session_total.cache_read),
                format_token_count(session_total.cache_write)
            );
        }
        if let Some((input_cost, cw_cost, cr_cost, output_cost)) =
            cost_breakdown(session_total, model)
        {
            println!();
            println!("    Breakdown:");
            println!("      input:       {}", format_cost(input_cost));
            println!("      output:      {}", format_cost(output_cost));
            if cw_cost > 0.0 {
                println!("      cache write: {}", format_cost(cw_cost));
            }
            if cr_cost > 0.0 {
                println!("      cache read:  {}", format_cost(cr_cost));
            }
        }
        println!("{RESET}");
    } else {
        println!("{DIM}  Cost estimation not available for model '{model}'.{RESET}\n");
    }
}

// ── /retry ───────────────────────────────────────────────────────────────

pub async fn handle_retry(
    agent: &mut Agent,
    last_input: &Option<String>,
    session_total: &mut Usage,
    model: &str,
) {
    match last_input {
        Some(prev) => {
            println!("{DIM}  (retrying last input){RESET}");
            let retry_input = prev.clone();
            run_prompt(agent, &retry_input, session_total, model).await;
            auto_compact_if_needed(agent);
        }
        None => {
            eprintln!("{DIM}  (nothing to retry — no previous input){RESET}\n");
        }
    }
}

// ── /model ───────────────────────────────────────────────────────────────

pub fn handle_model_show(model: &str) {
    println!("{DIM}  current model: {model}");
    println!("  usage: /model <name>{RESET}\n");
}

// ── /provider ────────────────────────────────────────────────────────────

pub fn handle_provider_show(provider: &str) {
    println!("{DIM}  current provider: {provider}");
    println!("  usage: /provider <name>");
    println!("  available: {}{RESET}\n", KNOWN_PROVIDERS.join(", "));
}

pub fn handle_provider_switch(
    new_provider: &str,
    agent_config: &mut crate::AgentConfig,
    agent: &mut Agent,
) {
    if !KNOWN_PROVIDERS.contains(&new_provider) {
        eprintln!("{RED}  unknown provider: '{new_provider}'{RESET}");
        eprintln!("{DIM}  available: {}{RESET}\n", KNOWN_PROVIDERS.join(", "));
        return;
    }
    agent_config.provider = new_provider.to_string();
    agent_config.model = default_model_for_provider(new_provider);
    let saved = agent.save_messages().ok();
    *agent = agent_config.build_agent();
    if let Some(json) = saved {
        let _ = agent.restore_messages(&json);
    }
    println!(
        "{DIM}  (switched to provider '{}', model '{}', conversation preserved){RESET}\n",
        agent_config.provider, agent_config.model
    );
}

// ── /think ───────────────────────────────────────────────────────────────

pub fn handle_think_show(thinking: ThinkingLevel) {
    let level_str = thinking_level_name(thinking);
    println!("{DIM}  thinking: {level_str}");
    println!("  usage: /think <off|minimal|low|medium|high>{RESET}\n");
}

// ── /config ──────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn handle_config(
    provider: &str,
    model: &str,
    base_url: &Option<String>,
    thinking: ThinkingLevel,
    max_tokens: Option<u32>,
    max_turns: Option<usize>,
    temperature: Option<f32>,
    skills: &yoagent::skills::SkillSet,
    system_prompt: &str,
    mcp_count: u32,
    openapi_count: u32,
    agent: &Agent,
    cwd: &str,
) {
    println!("{DIM}  Configuration:");
    println!("    provider:   {provider}");
    println!("    model:      {model}");
    if let Some(ref url) = base_url {
        println!("    base_url:   {url}");
    }
    println!("    thinking:   {}", thinking_level_name(thinking));
    println!(
        "    max_tokens: {}",
        max_tokens
            .map(|m| m.to_string())
            .unwrap_or_else(|| "default (8192)".to_string())
    );
    println!(
        "    max_turns:  {}",
        max_turns
            .map(|m| m.to_string())
            .unwrap_or_else(|| "default (50)".to_string())
    );
    println!(
        "    temperature: {}",
        temperature
            .map(|t| format!("{t:.1}"))
            .unwrap_or_else(|| "default".to_string())
    );
    println!(
        "    skills:     {}",
        if skills.is_empty() {
            "none".to_string()
        } else {
            format!("{} loaded", skills.len())
        }
    );
    let system_preview =
        truncate_with_ellipsis(system_prompt.lines().next().unwrap_or("(empty)"), 60);
    println!("    system:     {system_preview}");
    if mcp_count > 0 {
        println!("    mcp:        {mcp_count} server(s)");
    }
    if openapi_count > 0 {
        println!("    openapi:    {openapi_count} spec(s)");
    }
    println!(
        "    verbose:    {}",
        if is_verbose() { "on" } else { "off" }
    );
    if let Some(branch) = git_branch() {
        println!("    git:        {branch}");
    }
    println!("    cwd:        {cwd}");
    println!(
        "    context:    {} max tokens",
        format_token_count(MAX_CONTEXT_TOKENS)
    );
    println!(
        "    auto-compact: at {:.0}%",
        AUTO_COMPACT_THRESHOLD * 100.0
    );
    println!("    messages:   {}", agent.messages().len());
    println!(
        "    session:    auto-save on exit ({})",
        crate::cli::AUTO_SAVE_SESSION_PATH
    );
    println!("{RESET}");
}
