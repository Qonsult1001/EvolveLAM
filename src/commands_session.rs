//! Session-related command handlers: /save, /load, /compact, /history, /search,
//! /mark, /jump, /marks, /spawn, /stats, /confidence.

use crate::format::*;
use crate::prompt::*;

use std::collections::HashMap;
use yoagent::agent::Agent;
use yoagent::context::{compact_messages, total_tokens, ContextConfig};
use yoagent::*;

use crate::cli::{
    AUTO_COMPACT_THRESHOLD, AUTO_SAVE_SESSION_PATH, DEFAULT_SESSION_PATH, MAX_CONTEXT_TOKENS,
};

// ── compact ──────────────────────────────────────────────────────────────

/// Compact the agent's conversation and return (before_count, before_tokens, after_count, after_tokens).
/// Returns None if nothing changed.
pub fn compact_agent(agent: &mut Agent) -> Option<(usize, u64, usize, u64)> {
    let messages = agent.messages().to_vec();
    let before_tokens = total_tokens(&messages) as u64;
    let before_count = messages.len();
    let config = ContextConfig::default();
    let compacted = compact_messages(messages, &config);
    let after_tokens = total_tokens(&compacted) as u64;
    let after_count = compacted.len();
    agent.replace_messages(compacted);
    if before_tokens == after_tokens {
        None
    } else {
        Some((before_count, before_tokens, after_count, after_tokens))
    }
}

/// Auto-compact conversation if context window usage exceeds threshold.
pub fn auto_compact_if_needed(agent: &mut Agent) {
    let messages = agent.messages().to_vec();
    let used = total_tokens(&messages) as u64;
    let ratio = used as f64 / MAX_CONTEXT_TOKENS as f64;

    if ratio > AUTO_COMPACT_THRESHOLD {
        if let Some((before_count, before_tokens, after_count, after_tokens)) = compact_agent(agent)
        {
            println!(
                "{DIM}  ⚡ auto-compacted: {before_count} → {after_count} messages, ~{} → ~{} tokens{RESET}",
                format_token_count(before_tokens),
                format_token_count(after_tokens)
            );
        }
    }
}

pub fn handle_compact(agent: &mut Agent) {
    let messages = agent.messages();
    let before_count = messages.len();
    let before_tokens = total_tokens(messages) as u64;
    match compact_agent(agent) {
        Some((_, _, after_count, after_tokens)) => {
            println!(
                "{DIM}  compacted: {before_count} → {after_count} messages, ~{} → ~{} tokens{RESET}\n",
                format_token_count(before_tokens),
                format_token_count(after_tokens)
            );
        }
        None => {
            println!(
                "{DIM}  (nothing to compact — {before_count} messages, ~{} tokens){RESET}\n",
                format_token_count(before_tokens)
            );
        }
    }
}

// ── auto-save ────────────────────────────────────────────────────────────

/// Check whether a previous auto-saved session exists at `.yoyo/last-session.json`.
pub fn last_session_exists() -> bool {
    std::path::Path::new(AUTO_SAVE_SESSION_PATH).exists()
}

/// Auto-save the current conversation to `.yoyo/last-session.json`.
/// Creates the `.yoyo/` directory if it doesn't exist.
/// Silently ignores errors (best-effort crash recovery).
pub fn auto_save_on_exit(agent: &Agent) {
    if agent.messages().is_empty() {
        return;
    }
    if let Ok(json) = agent.save_messages() {
        // Ensure .yoyo/ directory exists
        let _ = std::fs::create_dir_all(".yoyo");
        if std::fs::write(AUTO_SAVE_SESSION_PATH, &json).is_ok() {
            eprintln!(
                "{DIM}  session auto-saved to {AUTO_SAVE_SESSION_PATH} ({} messages){RESET}",
                agent.messages().len()
            );
        }
    }
}

/// Return the path to load for `--continue`: use `.yoyo/last-session.json` if it exists,
/// otherwise fall back to the legacy `yoyo-session.json`.
pub fn continue_session_path() -> &'static str {
    if last_session_exists() {
        AUTO_SAVE_SESSION_PATH
    } else {
        DEFAULT_SESSION_PATH
    }
}

// ── /save ────────────────────────────────────────────────────────────────

pub fn handle_save(agent: &Agent, input: &str) {
    let path = input.strip_prefix("/save").unwrap_or("").trim();
    let path = if path.is_empty() {
        DEFAULT_SESSION_PATH
    } else {
        path
    };
    match agent.save_messages() {
        Ok(json) => match std::fs::write(path, &json) {
            Ok(_) => println!(
                "{DIM}  (session saved to {path}, {} messages){RESET}\n",
                agent.messages().len()
            ),
            Err(e) => eprintln!("{RED}  error saving: {e}{RESET}\n"),
        },
        Err(e) => eprintln!("{RED}  error serializing: {e}{RESET}\n"),
    }
}

// ── /load ────────────────────────────────────────────────────────────────

pub fn handle_load(agent: &mut Agent, input: &str) {
    let path = input.strip_prefix("/load").unwrap_or("").trim();
    let path = if path.is_empty() {
        DEFAULT_SESSION_PATH
    } else {
        path
    };
    match std::fs::read_to_string(path) {
        Ok(json) => match agent.restore_messages(&json) {
            Ok(_) => println!(
                "{DIM}  (session loaded from {path}, {} messages){RESET}\n",
                agent.messages().len()
            ),
            Err(e) => eprintln!("{RED}  error parsing: {e}{RESET}\n"),
        },
        Err(e) => eprintln!("{RED}  error reading {path}: {e}{RESET}\n"),
    }
}

// ── /history ─────────────────────────────────────────────────────────────

pub fn handle_history(agent: &Agent) {
    let messages = agent.messages();
    if messages.is_empty() {
        println!("{DIM}  (no messages in conversation){RESET}\n");
    } else {
        println!("{DIM}  Conversation ({} messages):", messages.len());
        for (i, msg) in messages.iter().enumerate() {
            let (role, preview) = summarize_message(msg);
            let idx = i + 1;
            println!("    {idx:>3}. [{role}] {preview}");
        }
        println!("{RESET}");
    }
}

// ── /search ──────────────────────────────────────────────────────────────

pub fn handle_search(agent: &Agent, input: &str) {
    if input == "/search" {
        println!("{DIM}  usage: /search <query>");
        println!("  Search conversation history for messages containing <query>.{RESET}\n");
        return;
    }
    let query = input.trim_start_matches("/search ").trim();
    if query.is_empty() {
        println!("{DIM}  usage: /search <query>{RESET}\n");
        return;
    }
    let messages = agent.messages();
    if messages.is_empty() {
        println!("{DIM}  (no messages to search){RESET}\n");
        return;
    }
    let results = search_messages(messages, query);
    if results.is_empty() {
        println!(
            "{DIM}  No matches for '{query}' in {len} messages.{RESET}\n",
            len = messages.len()
        );
    } else {
        println!(
            "{DIM}  {count} match{es} for '{query}':",
            count = results.len(),
            es = if results.len() == 1 { "" } else { "es" }
        );
        for (idx, role, preview) in &results {
            println!("    {idx:>3}. [{role}] {preview}");
        }
        println!("{RESET}");
    }
}

// ── /mark, /jump, /marks (bookmarks) ─────────────────────────────────────

/// Storage for conversation bookmarks: named snapshots of the message list.
pub type Bookmarks = HashMap<String, String>;

/// Path to the bookmarks persistence file.
const BOOKMARKS_FILE: &str = ".yoyo/bookmarks.json";

/// Load bookmarks from `.yoyo/bookmarks.json`.
/// Returns empty if file doesn't exist or can't be parsed.
pub fn load_bookmarks() -> Bookmarks {
    load_bookmarks_from(std::path::Path::new(BOOKMARKS_FILE))
}

/// Load bookmarks from a specific path (for testing).
pub fn load_bookmarks_from(path: &std::path::Path) -> Bookmarks {
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Bookmarks::new(),
    }
}

/// Save bookmarks to `.yoyo/bookmarks.json`.
/// Creates the `.yoyo/` directory if needed.
pub fn save_bookmarks(bookmarks: &Bookmarks) -> Result<(), String> {
    save_bookmarks_to(bookmarks, std::path::Path::new(BOOKMARKS_FILE))
}

/// Save bookmarks to a specific path (for testing).
pub fn save_bookmarks_to(bookmarks: &Bookmarks, path: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {e}"))?;
    }
    let json =
        serde_json::to_string_pretty(bookmarks).map_err(|e| format!("Serialization error: {e}"))?;
    std::fs::write(path, json).map_err(|e| format!("Failed to write bookmarks: {e}"))
}

/// Parse the bookmark name from `/mark <name>` input.
/// Returns None if no name is provided.
pub fn parse_bookmark_name(input: &str, prefix: &str) -> Option<String> {
    let name = input.strip_prefix(prefix).unwrap_or("").trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Handle `/mark <name>`: save the current conversation state as a named bookmark.
pub fn handle_mark(agent: &Agent, input: &str, bookmarks: &mut Bookmarks) {
    let name = match parse_bookmark_name(input, "/mark") {
        Some(n) => n,
        None => {
            println!("{DIM}  usage: /mark <name>");
            println!("  Save a bookmark at the current point in the conversation.");
            println!("  Use /jump <name> to return to this point later.{RESET}\n");
            return;
        }
    };

    match agent.save_messages() {
        Ok(json) => {
            let msg_count = agent.messages().len();
            let overwriting = bookmarks.contains_key(&name);
            bookmarks.insert(name.clone(), json);
            if overwriting {
                println!("{GREEN}  ✓ bookmark '{name}' updated ({msg_count} messages){RESET}\n");
            } else {
                println!("{GREEN}  ✓ bookmark '{name}' saved ({msg_count} messages){RESET}\n");
            }
            // Auto-persist to disk
            if let Err(e) = save_bookmarks(bookmarks) {
                eprintln!("{DIM}  (bookmark save warning: {e}){RESET}");
            }
        }
        Err(e) => eprintln!("{RED}  error saving bookmark: {e}{RESET}\n"),
    }
}

/// Handle `/jump <name>`: restore conversation to a previously saved bookmark.
pub fn handle_jump(agent: &mut Agent, input: &str, bookmarks: &Bookmarks) {
    let name = match parse_bookmark_name(input, "/jump") {
        Some(n) => n,
        None => {
            println!("{DIM}  usage: /jump <name>");
            println!("  Restore the conversation to a previously saved bookmark.");
            println!("  Messages added after the bookmark will be discarded.{RESET}\n");
            return;
        }
    };

    match bookmarks.get(&name) {
        Some(json) => match agent.restore_messages(json) {
            Ok(_) => {
                let msg_count = agent.messages().len();
                println!("{GREEN}  ✓ jumped to bookmark '{name}' ({msg_count} messages){RESET}\n");
            }
            Err(e) => eprintln!("{RED}  error restoring bookmark: {e}{RESET}\n"),
        },
        None => {
            let available: Vec<&str> = bookmarks.keys().map(|k| k.as_str()).collect();
            if available.is_empty() {
                eprintln!("{RED}  bookmark '{name}' not found — no bookmarks saved yet.");
                eprintln!("  Use /mark <name> to save one.{RESET}\n");
            } else {
                eprintln!("{RED}  bookmark '{name}' not found.");
                eprintln!("{DIM}  available: {}{RESET}\n", available.join(", "));
            }
        }
    }
}

/// Handle `/marks`: list all saved bookmarks.
pub fn handle_marks(bookmarks: &Bookmarks) {
    if bookmarks.is_empty() {
        println!("{DIM}  (no bookmarks saved)");
        println!("  Use /mark <name> to save a bookmark.{RESET}\n");
    } else {
        println!("{DIM}  Saved bookmarks:");
        let mut names: Vec<&String> = bookmarks.keys().collect();
        names.sort();
        for name in names {
            println!("    • {name}");
        }
        println!("{RESET}");
    }
}

// ── /spawn ────────────────────────────────────────────────────────────────

/// A completed spawn record for history tracking.
#[derive(Debug, Clone)]
pub struct SpawnRecord {
    pub id: usize,
    pub task: String,
    pub result: String,
    pub timestamp: String,
}

/// Spawn history — tracks all completed subagent runs in this session.
pub struct SpawnHistory {
    records: Vec<SpawnRecord>,
}

impl SpawnHistory {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    pub fn add(&mut self, task: String, result: String) -> usize {
        let id = self.records.len() + 1;
        let timestamp = simple_time_stamp();
        self.records.push(SpawnRecord {
            id,
            task,
            result,
            timestamp,
        });
        id
    }

    pub fn get(&self, id: usize) -> Option<&SpawnRecord> {
        self.records.iter().find(|r| r.id == id)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Format all records for display.
    pub fn format_list(&self) -> String {
        if self.records.is_empty() {
            return "  No spawns this session.\n".to_string();
        }
        let mut out = String::new();
        out.push_str(&format!(
            "  {} spawn(s) this session:\n",
            self.records.len()
        ));
        for r in &self.records {
            let task_preview = crate::format::truncate_with_ellipsis(&r.task, 60);
            let result_preview = crate::format::truncate_with_ellipsis(
                r.result.lines().next().unwrap_or("(empty)"),
                50,
            );
            out.push_str(&format!(
                "  #{} [{}] {}\n     → {}\n",
                r.id, r.timestamp, task_preview, result_preview
            ));
        }
        out
    }

    /// Aggregate results from multiple spawns into a summary.
    pub fn aggregate(&self, ids: &[usize]) -> String {
        let mut out = String::new();
        let records: Vec<_> = ids.iter().filter_map(|id| self.get(*id)).collect();
        if records.is_empty() {
            return "(no matching spawn results)".to_string();
        }
        for r in &records {
            out.push_str(&format!("## Spawn #{}: {}\n{}\n\n", r.id, r.task, r.result));
        }
        out
    }
}

/// Simple HH:MM timestamp without chrono dependency.
fn simple_time_stamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hours = (now % 86400) / 3600;
    let minutes = (now % 3600) / 60;
    format!("{hours:02}:{minutes:02}")
}

/// Parsed /spawn subcommand.
#[derive(Debug, PartialEq)]
pub enum SpawnCommand {
    Help,
    List,
    ShowResult(usize),
    Task(String),
}

/// Parse spawn subcommand: `/spawn list`, `/spawn result <id>`, or `/spawn <task>`.
pub fn parse_spawn_subcommand(input: &str) -> SpawnCommand {
    let rest = input.strip_prefix("/spawn").unwrap_or("").trim();
    if rest == "list" || rest == "ls" {
        SpawnCommand::List
    } else if let Some(id_str) = rest.strip_prefix("result ").or(rest.strip_prefix("show ")) {
        if let Ok(id) = id_str.trim().parse::<usize>() {
            SpawnCommand::ShowResult(id)
        } else {
            SpawnCommand::Task(rest.to_string())
        }
    } else if rest.is_empty() {
        SpawnCommand::Help
    } else {
        SpawnCommand::Task(rest.to_string())
    }
}

/// Display spawn help.
pub fn print_spawn_help() {
    println!("{DIM}  usage: /spawn <task>          Run task in fresh subagent context");
    println!("         /spawn list             Show completed spawns this session");
    println!("         /spawn result <id>      Show full result of spawn #id");
    println!("  Example: /spawn read src/main.rs and summarize the architecture{RESET}\n");
}

/// Handle the /spawn command: create a fresh subagent, run a task, and return the result.
/// The subagent gets its own independent context window so complex tasks don't pollute
/// the main conversation. Now with history tracking and subcommands.
/// Returns Some(context_msg) to be injected back into the main conversation, or None.
pub async fn handle_spawn(
    input: &str,
    agent_config: &crate::AgentConfig,
    session_total: &mut Usage,
    model: &str,
    history: &mut SpawnHistory,
) -> Option<String> {
    match parse_spawn_subcommand(input) {
        SpawnCommand::Help => {
            print_spawn_help();
            None
        }
        SpawnCommand::List => {
            let display = history.format_list();
            println!("{DIM}{display}{RESET}\n");
            None
        }
        SpawnCommand::ShowResult(id) => {
            if id == 0 {
                // /spawn result 0 → show all aggregated
                let all_ids: Vec<usize> = (1..=history.len()).collect();
                let agg = history.aggregate(&all_ids);
                println!("{agg}");
            } else if let Some(record) = history.get(id) {
                println!("{DIM}  Spawn #{}: {}{RESET}", record.id, record.task);
                println!("{DIM}  Completed: {}{RESET}\n", record.timestamp);
                println!("{}", record.result);
            } else {
                println!(
                    "{DIM}  No spawn with id #{id}. Use /spawn list to see available.{RESET}\n"
                );
            }
            None
        }
        SpawnCommand::Task(task) => {
            println!(
                "{CYAN}  🐙 spawning subagent #{}...{RESET}",
                history.len() + 1
            );
            println!(
                "{DIM}  task: {}{RESET}",
                crate::format::truncate_with_ellipsis(&task, 100)
            );

            // Build a fresh agent with the same config but independent context
            let mut sub_agent = agent_config.build_agent();

            // Run the task as a single prompt on the subagent
            let response = run_prompt(&mut sub_agent, &task, session_total, model).await;

            let result_text = if response.trim().is_empty() {
                "(no output)".to_string()
            } else {
                response.trim().to_string()
            };

            let id = history.add(task.clone(), result_text.clone());

            println!("\n{GREEN}  ✓ subagent #{id} completed{RESET}");
            println!("{DIM}  injecting result into main conversation...{RESET}");
            println!("{DIM}  use /spawn result {id} to see full output later{RESET}\n");

            let context_msg = format!(
                "Subagent #{id} just completed a task. Here is its result:\n\n**Task:** {task}\n\n**Result:**\n{result_text}"
            );

            Some(context_msg)
        }
    }
}

// ── /stats ──────────────────────────────────────────────────────────────

/// Parse JOURNAL.md to extract session stats: number of sessions per day, task counts.
pub fn parse_journal_stats(content: &str) -> JournalStats {
    let mut stats = JournalStats {
        total_sessions: 0,
        sessions_by_day: Vec::new(),
        total_tests_mentioned: 0,
        test_counts_by_session: Vec::new(),
        revert_sessions: 0,
        task_sessions: 0,
    };

    let mut current_day: Option<u32> = None;
    let mut day_sessions: u32 = 0;
    let mut session_has_revert = false;
    let mut session_has_task = false;
    let mut session_test_count: Option<u32> = None;

    for line in content.lines() {
        // Match "## Day N" headers
        if let Some(rest) = line.strip_prefix("## Day ") {
            // Flush previous session data
            if current_day.is_some() {
                if session_has_revert {
                    stats.revert_sessions += 1;
                }
                if session_has_task {
                    stats.task_sessions += 1;
                }
                if let (Some(day), Some(tc)) = (current_day, session_test_count) {
                    stats.test_counts_by_session.push((day, tc));
                }
            }
            session_has_revert = false;
            session_has_task = false;
            session_test_count = None;

            if let Some(day_num) = rest
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|s| s.parse::<u32>().ok())
            {
                if let Some(prev_day) = current_day {
                    if prev_day != day_num && day_sessions > 0 {
                        stats.sessions_by_day.push((prev_day, day_sessions));
                        day_sessions = 0;
                    }
                }
                current_day = Some(day_num);
                stats.total_sessions += 1;
                day_sessions += 1;
            }
        }

        let lower = line.to_lowercase();

        // Track reverts and tasks (exclude "zero reverts", "no reverts", "0 reverts")
        if lower.contains("revert")
            && !lower.contains("zero revert")
            && !lower.contains("no revert")
            && !lower.contains("0 revert")
        {
            session_has_revert = true;
        }
        if lower.contains("task") {
            session_has_task = true;
        }

        // Count test mentions like "N unit tests" or "N tests"
        if let Some(idx) = line.find(" unit test") {
            let before = &line[..idx];
            let num_str: String = before
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            if let Ok(num) = num_str.parse::<u32>() {
                stats.total_tests_mentioned = stats.total_tests_mentioned.max(num);
                session_test_count = Some(num);
            }
        }
    }
    // Flush last session
    if current_day.is_some() {
        if session_has_revert {
            stats.revert_sessions += 1;
        }
        if session_has_task {
            stats.task_sessions += 1;
        }
        if let (Some(day), Some(tc)) = (current_day, session_test_count) {
            stats.test_counts_by_session.push((day, tc));
        }
    }
    // Push the last day
    if let Some(day) = current_day {
        if day_sessions > 0 {
            stats.sessions_by_day.push((day, day_sessions));
        }
    }

    stats
}

/// Compute a simple linear trend from a sequence of values.
/// Returns (slope, direction_label). Positive slope = improving, negative = declining.
pub fn compute_trend(values: &[f64]) -> (f64, &'static str) {
    if values.len() < 2 {
        return (0.0, "insufficient data");
    }
    let n = values.len() as f64;
    let sum_x: f64 = (0..values.len()).map(|i| i as f64).sum();
    let sum_y: f64 = values.iter().sum();
    let sum_xy: f64 = values.iter().enumerate().map(|(i, y)| i as f64 * y).sum();
    let sum_x2: f64 = (0..values.len()).map(|i| (i as f64) * (i as f64)).sum();

    let denom = n * sum_x2 - sum_x * sum_x;
    if denom.abs() < f64::EPSILON {
        return (0.0, "flat");
    }
    let slope = (n * sum_xy - sum_x * sum_y) / denom;

    let label = if slope > 0.5 {
        "improving"
    } else if slope < -0.5 {
        "declining"
    } else {
        "stable"
    };
    (slope, label)
}

/// Stats extracted from JOURNAL.md.
#[derive(Debug, Clone, PartialEq)]
pub struct JournalStats {
    pub total_sessions: u32,
    pub sessions_by_day: Vec<(u32, u32)>,
    pub total_tests_mentioned: u32,
    /// Test counts extracted per session: (day, test_count).
    pub test_counts_by_session: Vec<(u32, u32)>,
    /// Count of sessions that mention "revert" (case-insensitive).
    pub revert_sessions: u32,
    /// Count of sessions that mention "task" (rough measure of tasks attempted).
    pub task_sessions: u32,
}

/// Format the /stats display output.
pub fn format_stats_display(journal_stats: &JournalStats, error_content: &str) -> String {
    let mut out = String::new();

    out.push_str("  Session Statistics:\n");
    out.push_str(&format!(
        "    Total sessions: {}\n",
        journal_stats.total_sessions
    ));

    if !journal_stats.sessions_by_day.is_empty() {
        let total_days = journal_stats.sessions_by_day.len();
        let avg = journal_stats.total_sessions as f64 / total_days as f64;
        out.push_str(&format!("    Days active: {}\n", total_days));
        out.push_str(&format!("    Avg sessions/day: {:.1}\n", avg));

        // Most productive day
        if let Some((day, count)) = journal_stats.sessions_by_day.iter().max_by_key(|(_, c)| c) {
            out.push_str(&format!(
                "    Most productive: Day {} ({} sessions)\n",
                day, count
            ));
        }
    }

    if journal_stats.total_tests_mentioned > 0 {
        out.push_str(&format!(
            "    Peak test count: {}\n",
            journal_stats.total_tests_mentioned
        ));
    }

    // Convergence metrics
    if journal_stats.total_sessions > 1 {
        out.push_str("\n  Convergence:\n");

        // Revert rate
        let revert_rate = if journal_stats.total_sessions > 0 {
            journal_stats.revert_sessions as f64 / journal_stats.total_sessions as f64 * 100.0
        } else {
            0.0
        };
        out.push_str(&format!(
            "    Revert rate: {}/{} sessions ({:.0}%)\n",
            journal_stats.revert_sessions, journal_stats.total_sessions, revert_rate
        ));

        // Test count trend
        if journal_stats.test_counts_by_session.len() >= 2 {
            let test_values: Vec<f64> = journal_stats
                .test_counts_by_session
                .iter()
                .map(|(_, tc)| *tc as f64)
                .collect();
            let (slope, label) = compute_trend(&test_values);
            out.push_str(&format!(
                "    Test growth: {} ({:+.1} tests/session)\n",
                label, slope
            ));
        }

        // Sessions per day trend
        if journal_stats.sessions_by_day.len() >= 2 {
            let day_values: Vec<f64> = journal_stats
                .sessions_by_day
                .iter()
                .map(|(_, count)| *count as f64)
                .collect();
            let (_, label) = compute_trend(&day_values);
            out.push_str(&format!("    Activity trend: {}\n", label));
        }

        // Overall convergence score (simple heuristic)
        let test_growing = journal_stats.test_counts_by_session.len() >= 2 && {
            let vals: Vec<f64> = journal_stats
                .test_counts_by_session
                .iter()
                .map(|(_, tc)| *tc as f64)
                .collect();
            compute_trend(&vals).0 > 0.0
        };
        let low_revert = revert_rate < 30.0;
        let score = (if test_growing { 1 } else { 0 }) + (if low_revert { 1 } else { 0 });
        let verdict = match score {
            2 => "converging",
            1 => "mixed signals",
            _ => "needs attention",
        };
        out.push_str(&format!("    Verdict: {}\n", verdict));
    }

    // Error stats
    let error_entries = crate::commands_project::parse_error_log(error_content);
    if !error_entries.is_empty() {
        let totals = crate::commands_project::summarize_error_log(&error_entries);
        let grand_total: usize = totals.iter().map(|(_, c)| c).sum();
        let resolved_count = error_entries
            .iter()
            .filter(|e| e.resolved == Some(true))
            .count();
        out.push_str(&format!(
            "\n  Error Recovery:\n    Total errors logged: {}\n    Fix events: {}\n    Resolved: {}\n",
            grand_total,
            error_entries.len(),
            resolved_count
        ));
        if let Some((top_cat, top_count)) = totals.first() {
            out.push_str(&format!(
                "    Most common error: {} ({})\n",
                top_cat, top_count
            ));
        }
    }

    out
}

/// A task parsed from SESSION_PLAN.md.
#[derive(Debug, Clone)]
pub struct PlanTask {
    pub number: u32,
    pub title: String,
    pub files: Vec<String>,
    pub description: String,
}

/// Confidence level for a task.
#[derive(Debug, Clone, PartialEq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::High => write!(f, "HIGH"),
            Confidence::Medium => write!(f, "MEDIUM"),
            Confidence::Low => write!(f, "LOW"),
        }
    }
}

/// Parse tasks from SESSION_PLAN.md content.
pub fn parse_plan_tasks(content: &str) -> Vec<PlanTask> {
    let mut tasks = Vec::new();
    let mut current_number = 0u32;
    let mut current_title = String::new();
    let mut current_files: Vec<String> = Vec::new();
    let mut current_desc = String::new();
    let mut in_task = false;

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("### Task ") {
            // Save previous task
            if in_task && current_number > 0 {
                tasks.push(PlanTask {
                    number: current_number,
                    title: current_title.clone(),
                    files: current_files.clone(),
                    description: current_desc.trim().to_string(),
                });
            }
            // Parse "### Task N: Title"
            if let Some((num_str, title)) = rest.split_once(':') {
                current_number = num_str.trim().parse().unwrap_or(0);
                current_title = title.trim().to_string();
            }
            current_files.clear();
            current_desc.clear();
            in_task = true;
        } else if in_task {
            if let Some(files_str) = line.strip_prefix("Files:") {
                current_files = files_str
                    .split(',')
                    .map(|f| f.trim().to_string())
                    .filter(|f| !f.is_empty())
                    .collect();
            } else if let Some(desc) = line.strip_prefix("Description:") {
                current_desc = desc.trim().to_string();
            } else if !line.starts_with("Issue:") && !line.trim().is_empty() {
                if !current_desc.is_empty() {
                    current_desc.push(' ');
                }
                current_desc.push_str(line.trim());
            }
        }
    }
    // Save last task
    if in_task && current_number > 0 {
        tasks.push(PlanTask {
            number: current_number,
            title: current_title,
            files: current_files,
            description: current_desc.trim().to_string(),
        });
    }
    tasks
}

/// Score a task's confidence based on journal familiarity and connection graph.
pub fn score_task_confidence(
    task: &PlanTask,
    journal_lower: &str,
    known_files: &[String],
    graph_concepts: &[String],
) -> (Confidence, Vec<String>) {
    let mut score = 0i32;
    let mut reasons = Vec::new();

    // Check if files are familiar (mentioned in journal or exist in src/)
    let familiar_files: Vec<_> = task
        .files
        .iter()
        .filter(|f| {
            known_files
                .iter()
                .any(|kf| kf.contains(f.as_str()) || f.contains(kf.as_str()))
        })
        .collect();
    if !task.files.is_empty() && familiar_files.len() == task.files.len() {
        score += 2;
        reasons.push("all files familiar".to_string());
    } else if !familiar_files.is_empty() {
        score += 1;
        reasons.push(format!(
            "{}/{} files familiar",
            familiar_files.len(),
            task.files.len()
        ));
    } else if !task.files.is_empty() {
        reasons.push("unfamiliar files".to_string());
    }

    // Check if task keywords appear in journal (has done similar work before)
    let title_words: Vec<&str> = task
        .title
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .collect();
    let matching_words: Vec<&&str> = title_words
        .iter()
        .filter(|w| journal_lower.contains(&w.to_lowercase()))
        .collect();
    if matching_words.len() >= 3 {
        score += 2;
        reasons.push("strong journal keyword overlap".to_string());
    } else if !matching_words.is_empty() {
        score += 1;
        reasons.push(format!(
            "some journal keywords ({})",
            matching_words
                .iter()
                .map(|w| w.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    } else {
        reasons.push("no prior journal mentions".to_string());
    }

    // Check connection graph for related concepts
    let desc_lower = task.description.to_lowercase();
    let related_concepts: Vec<_> = graph_concepts
        .iter()
        .filter(|c| {
            let cl = c.to_lowercase();
            desc_lower.contains(&cl) || task.title.to_lowercase().contains(&cl)
        })
        .collect();
    if related_concepts.len() >= 2 {
        score += 2;
        reasons.push(format!("graph concepts: {}", related_concepts.len()));
    } else if !related_concepts.is_empty() {
        score += 1;
        reasons.push("1 graph concept".to_string());
    }

    let confidence = if score >= 4 {
        Confidence::High
    } else if score >= 2 {
        Confidence::Medium
    } else {
        Confidence::Low
    };

    (confidence, reasons)
}

/// Format confidence display for all tasks.
pub fn format_confidence_display(
    tasks: &[PlanTask],
    journal_lower: &str,
    known_files: &[String],
    graph_concepts: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("  Task Confidence Scores\n");
    out.push_str("  ─────────────────────\n");

    if tasks.is_empty() {
        out.push_str("  No tasks found in SESSION_PLAN.md\n");
        return out;
    }

    let mut high_count = 0;
    let mut low_count = 0;

    for task in tasks {
        let (confidence, reasons) =
            score_task_confidence(task, journal_lower, known_files, graph_concepts);
        let marker = match &confidence {
            Confidence::High => "●",
            Confidence::Medium => "◐",
            Confidence::Low => "○",
        };
        match &confidence {
            Confidence::High => high_count += 1,
            Confidence::Low => low_count += 1,
            _ => {}
        }
        out.push_str(&format!(
            "  {} Task {}: {} [{}]\n",
            marker, task.number, task.title, confidence
        ));
        out.push_str(&format!("    Reasons: {}\n", reasons.join("; ")));
    }

    out.push('\n');
    if low_count > 0 {
        out.push_str(&format!(
            "  Suggestion: {} low-confidence task(s) — consider extra verification or front-load high-confidence work.\n",
            low_count
        ));
    }
    if high_count == tasks.len() {
        out.push_str("  All tasks high confidence — execute with conviction.\n");
    }

    out
}

/// Handle the /confidence command.
pub fn handle_confidence() {
    let plan = std::fs::read_to_string("SESSION_PLAN.md").unwrap_or_default();
    let tasks = parse_plan_tasks(&plan);

    let journal = std::fs::read_to_string("JOURNAL.md").unwrap_or_default();
    let journal_lower = journal.to_lowercase();

    // Collect known files from src/
    let known_files: Vec<String> = std::fs::read_dir("src")
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();

    // Collect graph concepts
    let graph = crate::memory::ConnectionGraph::load();
    let graph_concepts: Vec<String> = graph
        .search_concepts("")
        .into_iter()
        .map(|(name, _)| name)
        .collect();

    let display = format_confidence_display(&tasks, &journal_lower, &known_files, &graph_concepts);
    println!("{DIM}{display}{RESET}\n");
}

/// Handle the /stats command.
pub fn handle_stats() {
    let journal = std::fs::read_to_string("JOURNAL.md").unwrap_or_default();
    let journal_stats = parse_journal_stats(&journal);
    let error_content = std::fs::read_to_string(".yoyo/error_log.jsonl").unwrap_or_default();
    let display = format_stats_display(&journal_stats, &error_content);
    println!("{DIM}{display}{RESET}\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::AUTO_SAVE_SESSION_PATH;
    use std::sync::{Mutex, OnceLock};

    static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn lock_cwd() -> std::sync::MutexGuard<'static, ()> {
        CWD_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn test_auto_save_session_path_constant() {
        assert_eq!(AUTO_SAVE_SESSION_PATH, ".yoyo/last-session.json");
    }

    #[test]
    fn test_continue_session_path_fallback() {
        // When .yoyo/last-session.json doesn't exist, should fall back to yoyo-session.json
        // (In CI, .yoyo/last-session.json won't exist unless created by a prior test)
        let path = continue_session_path();
        // Should be one of the two valid paths
        assert!(
            path == AUTO_SAVE_SESSION_PATH || path == DEFAULT_SESSION_PATH,
            "continue_session_path should return a valid session path, got: {path}"
        );
    }

    #[test]
    fn test_last_session_exists_returns_bool() {
        // Should not panic regardless of whether the file exists
        let _exists = last_session_exists();
    }

    #[test]
    fn test_auto_save_creates_directory_and_file() {
        use yoagent::agent::Agent;
        use yoagent::provider::AnthropicProvider;

        let _cwd_guard = lock_cwd();

        // Use a temp directory to avoid polluting the project
        let tmp_dir = std::env::temp_dir().join("yoyo_test_autosave");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let original_dir = std::env::current_dir().unwrap();

        // Change to temp directory
        std::env::set_current_dir(&tmp_dir).unwrap();

        // Create an agent with an empty conversation — should NOT save
        let agent = Agent::new(AnthropicProvider)
            .with_system_prompt("test")
            .with_model("test-model")
            .with_api_key("test-key");
        auto_save_on_exit(&agent);
        assert!(
            !std::path::Path::new(AUTO_SAVE_SESSION_PATH).exists(),
            "Should not save empty conversations"
        );

        // Restore directory
        std::env::set_current_dir(&original_dir).unwrap();
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_continue_session_path_prefers_auto_save() {
        // Create a temp directory with .yoyo/last-session.json
        let _cwd_guard = lock_cwd();

        let tmp_dir = std::env::temp_dir().join("yoyo_test_continue_path");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(tmp_dir.join(".yoyo")).unwrap();
        std::fs::write(tmp_dir.join(".yoyo/last-session.json"), "[]").unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&tmp_dir).unwrap();

        let path = continue_session_path();
        assert_eq!(
            path, AUTO_SAVE_SESSION_PATH,
            "Should prefer .yoyo/last-session.json when it exists"
        );

        std::env::set_current_dir(&original_dir).unwrap();
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_continue_session_path_falls_back_to_default() {
        // Create a temp directory WITHOUT .yoyo/last-session.json
        let _cwd_guard = lock_cwd();

        let tmp_dir = std::env::temp_dir().join("yoyo_test_continue_fallback");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&tmp_dir).unwrap();

        let path = continue_session_path();
        assert_eq!(
            path, DEFAULT_SESSION_PATH,
            "Should fall back to yoyo-session.json when .yoyo/last-session.json doesn't exist"
        );

        std::env::set_current_dir(&original_dir).unwrap();
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_parse_journal_stats_counts_sessions() {
        let journal = "# Journal\n\n## Day 19 — 07:21 — title\nSome text\n\n## Day 19 — 07:00 — title2\nMore text\n\n## Day 18 — 23:36 — title3\nText\n";
        let stats = parse_journal_stats(journal);
        assert_eq!(stats.total_sessions, 3);
        assert_eq!(stats.sessions_by_day.len(), 2);
    }

    #[test]
    fn test_parse_journal_stats_finds_test_count() {
        let journal = "## Day 19 — 07:21 — title\n716 unit tests + 67 integration tests.\n";
        let stats = parse_journal_stats(journal);
        assert_eq!(stats.total_tests_mentioned, 716);
    }

    #[test]
    fn test_parse_journal_stats_empty() {
        let stats = parse_journal_stats("");
        assert_eq!(stats.total_sessions, 0);
        assert!(stats.sessions_by_day.is_empty());
    }

    #[test]
    fn test_format_stats_display_basic() {
        let stats = JournalStats {
            total_sessions: 5,
            sessions_by_day: vec![(18, 2), (19, 3)],
            total_tests_mentioned: 700,
            test_counts_by_session: vec![(18, 600), (18, 650), (19, 680), (19, 700)],
            revert_sessions: 1,
            task_sessions: 4,
        };
        let display = format_stats_display(&stats, "");
        assert!(display.contains("Total sessions: 5"));
        assert!(display.contains("Days active: 2"));
        assert!(display.contains("Avg sessions/day: 2.5"));
        assert!(display.contains("Most productive: Day 19 (3 sessions)"));
        assert!(display.contains("Peak test count: 700"));
        assert!(display.contains("Convergence:"));
        assert!(display.contains("Revert rate:"));
        assert!(display.contains("Test growth:"));
    }

    #[test]
    fn test_format_stats_display_with_errors() {
        let stats = JournalStats {
            total_sessions: 1,
            sessions_by_day: vec![(19, 1)],
            total_tests_mentioned: 0,
            test_counts_by_session: Vec::new(),
            revert_sessions: 0,
            task_sessions: 0,
        };
        let error_content = r#"{"ts":"2026-03-19T06:30:00Z","day":19,"categories":{"missing_import":3},"source":"fix","resolved":"true"}"#;
        let display = format_stats_display(&stats, error_content);
        assert!(display.contains("Error Recovery"));
        assert!(display.contains("Total errors logged: 3"));
        assert!(display.contains("Resolved: 1"));
    }

    #[test]
    fn test_compute_trend_improving() {
        let values = vec![100.0, 200.0, 300.0, 400.0];
        let (slope, label) = compute_trend(&values);
        assert!(slope > 0.0);
        assert_eq!(label, "improving");
    }

    #[test]
    fn test_compute_trend_declining() {
        let values = vec![400.0, 300.0, 200.0, 100.0];
        let (slope, label) = compute_trend(&values);
        assert!(slope < 0.0);
        assert_eq!(label, "declining");
    }

    #[test]
    fn test_compute_trend_stable() {
        let values = vec![100.0, 100.0, 100.0, 100.0];
        let (slope, label) = compute_trend(&values);
        assert!(slope.abs() < 0.01);
        assert_eq!(label, "stable");
    }

    #[test]
    fn test_compute_trend_insufficient_data() {
        let values = vec![100.0];
        let (_, label) = compute_trend(&values);
        assert_eq!(label, "insufficient data");
    }

    #[test]
    fn test_parse_journal_stats_tracks_reverts() {
        let journal = "## Day 10 — 06:00 — session one\nFive tasks, zero reverts.\n\n## Day 10 — 12:00 — session two\nTask 1 was reverted due to build failure.\n";
        let stats = parse_journal_stats(journal);
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.revert_sessions, 1); // only second mentions "revert"
    }

    #[test]
    fn test_parse_journal_stats_tracks_test_counts() {
        let journal = "## Day 18 — 12:00 — session\n694 unit tests, 67 integration tests.\n\n## Day 19 — 08:00 — session\n739 unit tests + 67 integration tests.\n";
        let stats = parse_journal_stats(journal);
        assert_eq!(stats.test_counts_by_session.len(), 2);
        assert_eq!(stats.test_counts_by_session[0], (18, 694));
        assert_eq!(stats.test_counts_by_session[1], (19, 739));
        assert_eq!(stats.total_tests_mentioned, 739);
    }

    #[test]
    fn test_convergence_verdict_converging() {
        let stats = JournalStats {
            total_sessions: 10,
            sessions_by_day: vec![(15, 2), (16, 3), (17, 2), (18, 3)],
            total_tests_mentioned: 739,
            test_counts_by_session: vec![(15, 500), (16, 550), (17, 600), (18, 700)],
            revert_sessions: 1,
            task_sessions: 8,
        };
        let display = format_stats_display(&stats, "");
        assert!(display.contains("converging"));
    }

    #[test]
    fn test_parse_plan_tasks_basic() {
        let plan = "## Session Plan\n\n### Task 1: Fix the widget\nFiles: src/main.rs, src/format.rs\nDescription: Make the widget work properly.\nIssue: none\n\n### Task 2: Add tests\nFiles: src/commands.rs\nDescription: Write unit tests for commands.\nIssue: #42\n";
        let tasks = parse_plan_tasks(plan);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].number, 1);
        assert_eq!(tasks[0].title, "Fix the widget");
        assert_eq!(tasks[0].files, vec!["src/main.rs", "src/format.rs"]);
        assert!(tasks[0].description.contains("widget work properly"));
        assert_eq!(tasks[1].number, 2);
        assert_eq!(tasks[1].title, "Add tests");
    }

    #[test]
    fn test_parse_plan_tasks_empty() {
        let tasks = parse_plan_tasks("nothing here");
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_score_task_high_confidence() {
        let task = PlanTask {
            number: 1,
            title: "Fix error handling in format module".to_string(),
            files: vec!["src/format.rs".to_string()],
            description: "Update error classification logic".to_string(),
        };
        let journal =
            "day 15: worked on error handling in the format module. fixed classification bugs.";
        let known_files = vec!["format.rs".to_string()];
        let concepts = vec!["error".to_string(), "classification".to_string()];
        let (conf, _reasons) = score_task_confidence(&task, journal, &known_files, &concepts);
        assert_eq!(conf, Confidence::High);
    }

    #[test]
    fn test_score_task_low_confidence() {
        let task = PlanTask {
            number: 1,
            title: "Implement WASM compilation target".to_string(),
            files: vec!["src/wasm.rs".to_string()],
            description: "Add WebAssembly support for browser execution".to_string(),
        };
        let journal = "day 15: worked on stats and journal parsing.";
        let known_files = vec!["main.rs".to_string()];
        let concepts: Vec<String> = vec![];
        let (conf, _reasons) = score_task_confidence(&task, journal, &known_files, &concepts);
        assert_eq!(conf, Confidence::Low);
    }

    #[test]
    fn test_format_confidence_display_empty() {
        let display = format_confidence_display(&[], "", &[], &[]);
        assert!(display.contains("No tasks found"));
    }

    #[test]
    fn test_format_confidence_display_shows_markers() {
        let tasks = vec![PlanTask {
            number: 1,
            title: "Test task".to_string(),
            files: vec!["src/main.rs".to_string()],
            description: "A test task".to_string(),
        }];
        let display = format_confidence_display(&tasks, "", &[], &[]);
        assert!(display.contains("Task 1:"));
        assert!(display.contains("Reasons:"));
    }

    #[test]
    fn test_confidence_display_enum() {
        assert_eq!(format!("{}", Confidence::High), "HIGH");
        assert_eq!(format!("{}", Confidence::Medium), "MEDIUM");
        assert_eq!(format!("{}", Confidence::Low), "LOW");
    }
}
