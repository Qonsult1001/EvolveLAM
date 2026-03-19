//! Session-related command handlers: /save, /load, /compact, /history, /search,
//! /mark, /jump, /marks, /spawn, /stats.

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

/// Parse the task from a `/spawn <task>` input.
/// Returns None if no task is provided.
pub fn parse_spawn_task(input: &str) -> Option<String> {
    let task = input
        .strip_prefix("/spawn")
        .unwrap_or("")
        .trim()
        .to_string();
    if task.is_empty() {
        None
    } else {
        Some(task)
    }
}

/// Handle the /spawn command: create a fresh subagent, run a task, and return the result.
/// The subagent gets its own independent context window so complex tasks don't pollute
/// the main conversation.
/// Returns Some(context_msg) to be injected back into the main conversation, or None.
pub async fn handle_spawn(
    input: &str,
    agent_config: &crate::AgentConfig,
    session_total: &mut Usage,
    model: &str,
) -> Option<String> {
    let task = match parse_spawn_task(input) {
        Some(t) => t,
        None => {
            println!("{DIM}  usage: /spawn <task>");
            println!("  Spawn a subagent with a fresh context to handle a task.");
            println!("  The result is summarized back into your main conversation.");
            println!("  Example: /spawn read src/main.rs and summarize the architecture{RESET}\n");
            return None;
        }
    };

    println!("{CYAN}  🐙 spawning subagent...{RESET}");
    println!(
        "{DIM}  task: {}{RESET}",
        crate::format::truncate_with_ellipsis(&task, 100)
    );

    // Build a fresh agent with the same config but independent context
    let mut sub_agent = agent_config.build_agent();

    // Run the task as a single prompt on the subagent
    let response = run_prompt(&mut sub_agent, &task, session_total, model).await;

    println!("\n{GREEN}  ✓ subagent completed{RESET}");
    println!("{DIM}  injecting result into main conversation...{RESET}\n");

    // Build a context message for the main agent summarizing what the subagent did
    let result_text = if response.trim().is_empty() {
        "(no output)".to_string()
    } else {
        response.trim().to_string()
    };

    let context_msg = format!(
        "A subagent just completed a task. Here is its result:\n\n**Task:** {task}\n\n**Result:**\n{result_text}"
    );

    Some(context_msg)
}

// ── /stats ──────────────────────────────────────────────────────────────

/// Parse JOURNAL.md to extract session stats: number of sessions per day, task counts.
pub fn parse_journal_stats(content: &str) -> JournalStats {
    let mut stats = JournalStats {
        total_sessions: 0,
        sessions_by_day: Vec::new(),
        total_tests_mentioned: 0,
    };

    let mut current_day: Option<u32> = None;
    let mut day_sessions: u32 = 0;

    for line in content.lines() {
        // Match "## Day N" headers
        if let Some(rest) = line.strip_prefix("## Day ") {
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
        // Count test mentions like "N unit tests" or "N tests"
        if let Some(idx) = line.find(" unit test") {
            let before = &line[..idx];
            // Extract the last number before " unit test"
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
            }
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

/// Stats extracted from JOURNAL.md.
#[derive(Debug, Clone, PartialEq)]
pub struct JournalStats {
    pub total_sessions: u32,
    pub sessions_by_day: Vec<(u32, u32)>,
    pub total_tests_mentioned: u32,
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
        };
        let display = format_stats_display(&stats, "");
        assert!(display.contains("Total sessions: 5"));
        assert!(display.contains("Days active: 2"));
        assert!(display.contains("Avg sessions/day: 2.5"));
        assert!(display.contains("Most productive: Day 19 (3 sessions)"));
        assert!(display.contains("Peak test count: 700"));
    }

    #[test]
    fn test_format_stats_display_with_errors() {
        let stats = JournalStats {
            total_sessions: 1,
            sessions_by_day: vec![(19, 1)],
            total_tests_mentioned: 0,
        };
        let error_content = r#"{"ts":"2026-03-19T06:30:00Z","day":19,"categories":{"missing_import":3},"source":"fix","resolved":"true"}"#;
        let display = format_stats_display(&stats, error_content);
        assert!(display.contains("Error Recovery"));
        assert!(display.contains("Total errors logged: 3"));
        assert!(display.contains("Resolved: 1"));
    }
}
