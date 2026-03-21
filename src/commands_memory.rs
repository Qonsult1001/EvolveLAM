//! Memory and connection graph command handlers for yoyo.
//!
//! Handles `/remember`, `/memories`, `/forget`, and `/graph` subcommands.

use crate::format::*;
use crate::memory::{add_memory, load_memories, remove_memory, save_memories};

// ── /remember ────────────────────────────────────────────────────────────

pub fn handle_remember(input: &str) {
    let note = input
        .strip_prefix("/remember")
        .unwrap_or("")
        .trim()
        .to_string();
    if note.is_empty() {
        println!("{DIM}  usage: /remember <note>");
        println!("  Save a project-specific memory that persists across sessions.");
        println!("  Examples:");
        println!("    /remember this project uses sqlx for database access");
        println!("    /remember tests require docker running");
        println!("    /remember always run cargo fmt before committing{RESET}\n");
        return;
    }
    let mut memory = load_memories();
    if !add_memory(&mut memory, &note) {
        println!("{DIM}  ⚡ Similar memory already exists, skipping duplicate.{RESET}\n");
        return;
    }
    match save_memories(&memory) {
        Ok(_) => {
            println!(
                "{GREEN}  ✓ Remembered: \"{note}\" ({} total memories){RESET}\n",
                memory.entries.len()
            );
        }
        Err(e) => {
            eprintln!("{RED}  error saving memory: {e}{RESET}\n");
        }
    }
}

// ── /memories ────────────────────────────────────────────────────────────

pub fn handle_memories() {
    let memory = load_memories();
    if memory.entries.is_empty() {
        println!("{DIM}  No project memories yet.");
        println!("  Use /remember <note> to add one.{RESET}\n");
        return;
    }
    println!("{DIM}  Project memories ({}):", memory.entries.len());
    for (i, entry) in memory.entries.iter().enumerate() {
        println!("    [{i}] {} ({})", entry.note, entry.timestamp);
    }
    println!("  Use /forget <n> to remove a memory.{RESET}\n");
}

// ── /forget ──────────────────────────────────────────────────────────────

pub fn handle_forget(input: &str) {
    let arg = input.strip_prefix("/forget").unwrap_or("").trim();
    if arg.is_empty() {
        println!("{DIM}  usage: /forget <n>");
        println!("  Remove a project memory by index. Use /memories to see indexes.{RESET}\n");
        return;
    }
    let index = match arg.parse::<usize>() {
        Ok(i) => i,
        Err(_) => {
            eprintln!("{RED}  error: '{arg}' is not a valid index. Use /memories to see indexes.{RESET}\n");
            return;
        }
    };
    let mut memory = load_memories();
    match remove_memory(&mut memory, index) {
        Some(removed) => match save_memories(&memory) {
            Ok(_) => {
                println!(
                    "{GREEN}  ✓ Forgot: \"{}\" ({} memories remaining){RESET}\n",
                    removed.note,
                    memory.entries.len()
                );
            }
            Err(e) => {
                eprintln!("{RED}  error saving memory: {e}{RESET}\n");
            }
        },
        None => {
            eprintln!(
                "{RED}  error: index {index} out of range (have {} memories). Use /memories to see indexes.{RESET}\n",
                memory.entries.len()
            );
        }
    }
}

// ── /graph ───────────────────────────────────────────────────────────────

/// Handle /graph subcommands — explore the latent space connection graph.
pub fn handle_graph(input: &str) {
    let rest = input.strip_prefix("/graph").unwrap_or("").trim();
    if rest.is_empty() {
        print_graph_help();
        return;
    }

    if let Some(args) = rest.strip_prefix("downstream") {
        handle_graph_downstream(args.trim());
    } else if let Some(args) = rest.strip_prefix("neighbors") {
        handle_graph_neighbors(args.trim());
    } else if rest == "info" || rest.starts_with("info ") {
        handle_graph_info();
    } else if let Some(args) = rest.strip_prefix("activate") {
        handle_graph_activate(args.trim());
    } else if let Some(args) = rest.strip_prefix("similar") {
        handle_graph_similar(args.trim());
    } else if let Some(args) = rest.strip_prefix("search") {
        handle_graph_search(args.trim());
    } else if let Some(args) = rest.strip_prefix("path") {
        handle_graph_path(args.trim());
    } else if rest == "populate" || rest.starts_with("populate ") {
        handle_graph_populate();
    } else if rest == "stats" || rest.starts_with("stats ") {
        handle_graph_stats();
    } else if rest == "communities" || rest.starts_with("communities ") {
        handle_graph_communities();
    } else {
        print_graph_help();
    }
}

fn print_graph_help() {
    println!("{DIM}  usage: /graph downstream <concept>   Show causal downstream");
    println!("         /graph neighbors <concept>   Show all connections for a concept");
    println!("         /graph similar <a> <b>       Compare two concepts (similarity + MI)");
    println!("         /graph info                  Show graph statistics");
    println!("         /graph activate <from> <to> <kind>  Activate a connection");
    println!("         /graph search <query>        Search concepts by substring");
    println!("         /graph path <from> <to>      Shortest path between concepts");
    println!("         /graph populate              Populate graph from learnings.jsonl");
    println!(
        "         /graph stats                 Show detailed graph health metrics
         /graph communities           Detect concept clusters via label propagation{RESET}\n"
    );
}

fn handle_graph_downstream(concept: &str) {
    if concept.is_empty() {
        println!("{DIM}  usage: /graph downstream <concept>{RESET}\n");
        return;
    }
    let graph = crate::memory::ConnectionGraph::load();
    let down = graph.causal_downstream(concept);
    if down.is_empty() {
        println!("{DIM}  No causal downstream for \"{concept}\".{RESET}\n");
    } else {
        let now_ts = crate::memory::current_timestamp();
        println!("  Causal downstream of \"{concept}\":");
        for c in &down {
            let eff = graph
                .edges
                .values()
                .flat_map(|v| v.iter())
                .find(|e| e.to == *c && e.kind == crate::memory::ConnectionKind::Causal)
                .map(|e| {
                    crate::memory::effective_weight(e, &now_ts, crate::memory::HALF_LIFE_DAYS)
                });
            match eff {
                Some(w) => println!("    {c} (eff:{w:.2})"),
                None => println!("    {c}"),
            }
        }
        println!();
    }
}

fn handle_graph_neighbors(concept: &str) {
    if concept.is_empty() {
        println!("{DIM}  usage: /graph neighbors <concept>{RESET}\n");
        return;
    }
    let graph = crate::memory::ConnectionGraph::load();
    let (outgoing, incoming) = graph.neighbors_detailed(concept);
    if outgoing.is_empty() && incoming.is_empty() {
        println!("{DIM}  No connections for \"{concept}\".{RESET}\n");
        return;
    }
    let now_ts = crate::memory::current_timestamp();
    println!("  Connections for \"{concept}\":");
    if !outgoing.is_empty() {
        println!("{DIM}  Outgoing:{RESET}");
        for conn in &outgoing {
            let eff = crate::memory::effective_weight(conn, &now_ts, crate::memory::HALF_LIFE_DAYS);
            println!(
                "    → {} ({}, w:{:.2}, eff:{:.2}, {}x)",
                conn.to, conn.kind, conn.weight, eff, conn.activations
            );
        }
    }
    if !incoming.is_empty() {
        println!("{DIM}  Incoming:{RESET}");
        for conn in &incoming {
            let eff = crate::memory::effective_weight(conn, &now_ts, crate::memory::HALF_LIFE_DAYS);
            println!(
                "    ← {} ({}, w:{:.2}, eff:{:.2}, {}x)",
                conn.from, conn.kind, conn.weight, eff, conn.activations
            );
        }
    }
    println!();
}

fn handle_graph_info() {
    let graph = crate::memory::ConnectionGraph::load();
    if graph.edges.is_empty() {
        println!("{DIM}  Connection graph is empty.{RESET}\n");
        return;
    }
    let by_kind = graph.connections_by_kind();
    let total_activations: u64 = graph.node_activations.values().sum();
    println!("  Connection graph:");
    println!(
        "    Nodes: {}  |  Connections: {}  |  Activations: {}",
        graph.node_count(),
        graph.connection_count(),
        total_activations
    );
    if !by_kind.is_empty() {
        let mut kinds: Vec<_> = by_kind.iter().collect();
        kinds.sort_by(|a, b| b.1.cmp(a.1));
        let kind_strs: Vec<String> = kinds.iter().map(|(k, v)| format!("{k}: {v}")).collect();
        println!("    By kind: {}", kind_strs.join(", "));
    }
    // Show top 5 strongest connections
    let mut all_conns: Vec<&crate::memory::Connection> =
        graph.edges.values().flat_map(|v| v.iter()).collect();
    all_conns.sort_by(|a, b| {
        b.weight
            .partial_cmp(&a.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if !all_conns.is_empty() {
        println!("{DIM}    Strongest connections:{RESET}");
        for conn in all_conns.iter().take(5) {
            println!(
                "      {} → {} (w:{:.2}, {}, {}x)",
                conn.from, conn.to, conn.weight, conn.kind, conn.activations
            );
        }
    }
    println!();
}

fn handle_graph_activate(args: &str) {
    if args.is_empty() {
        println!("{DIM}  usage: /graph activate <from> <to> <kind>");
        println!("  kinds: semantic, causal, temporal, mathematical, scientific{RESET}\n");
        return;
    }
    let parts: Vec<&str> = args.splitn(3, char::is_whitespace).collect();
    if parts.len() < 3 {
        println!("{DIM}  usage: /graph activate <from> <to> <kind>");
        println!("  kinds: semantic, causal, temporal, mathematical, scientific{RESET}\n");
        return;
    }
    let from = parts[0];
    let to = parts[1];
    let kind_str = parts[2].trim();
    let kind = match crate::memory::parse_connection_kind(kind_str) {
        Some(k) => k,
        None => {
            println!(
                "{RED}  Unknown kind: \"{kind_str}\". Use: semantic, causal, temporal, mathematical, scientific{RESET}\n"
            );
            return;
        }
    };
    let mut graph = crate::memory::ConnectionGraph::load();
    let new_weight = graph.activate_connection(from, to, kind.clone());
    match graph.save() {
        Ok(_) => {
            println!("{GREEN}  ✓ activated {from} → {to} ({kind}, w:{new_weight:.2}){RESET}\n");
        }
        Err(e) => {
            eprintln!("{RED}  error saving graph: {e}{RESET}\n");
        }
    }
}

fn handle_graph_path(args: &str) {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        println!("{DIM}  usage: /graph path <from> <to>{RESET}\n");
        return;
    }
    let from = parts[0].trim();
    let to = parts[1].trim();
    let graph = crate::memory::ConnectionGraph::load();
    match graph.shortest_path(from, to) {
        Some(path) => {
            println!(
                "  Path from \"{from}\" to \"{to}\" ({} hops):",
                path.len() - 1
            );
            for (i, node) in path.iter().enumerate() {
                if i == 0 {
                    println!("    {node}");
                } else {
                    // Find the edge between path[i-1] and path[i] to show the kind
                    let prev = &path[i - 1];
                    let edge_kind = graph
                        .edges
                        .get(prev.as_str())
                        .and_then(|edges| edges.iter().find(|e| e.to == *node))
                        .map(|e| format!("{}", e.kind))
                        .or_else(|| {
                            // Check reverse direction
                            graph
                                .edges
                                .get(node.as_str())
                                .and_then(|edges| edges.iter().find(|e| e.to == *prev))
                                .map(|e| format!("{} (rev)", e.kind))
                        })
                        .unwrap_or_else(|| "?".to_string());
                    println!("    → {node}  [{edge_kind}]");
                }
            }
            println!();
        }
        None => {
            println!("{DIM}  No path found between \"{from}\" and \"{to}\".{RESET}\n");
        }
    }
}

fn handle_graph_populate() {
    println!("  Loading learnings from memory/learnings.jsonl...");
    let learnings = crate::memory::load_learnings();
    if learnings.is_empty() {
        println!("{DIM}  No learnings found in memory/learnings.jsonl.{RESET}\n");
        return;
    }
    println!(
        "  Found {} learnings. Extracting concepts...",
        learnings.len()
    );

    let mut graph = crate::memory::ConnectionGraph::load();
    let before_nodes = graph.node_count();
    let before_edges = graph.connection_count();

    let (processed, connections) = graph.populate_from_learnings(&learnings);

    match graph.save() {
        Ok(()) => {
            println!(
                "  Populated graph from {} learnings ({} concept-pair activations).",
                processed, connections
            );
            println!(
                "  Graph: {} nodes (+{}), {} connections (+{})",
                graph.node_count(),
                graph.node_count().saturating_sub(before_nodes),
                graph.connection_count(),
                graph.connection_count().saturating_sub(before_edges),
            );
            println!("  Saved to memory/connections.jsonl\n");
        }
        Err(e) => {
            println!("{DIM}  Error saving graph: {e}{RESET}\n");
        }
    }
}

fn handle_graph_stats() {
    let graph = crate::memory::ConnectionGraph::load();
    if graph.edges.is_empty() {
        println!("{DIM}  Connection graph is empty. Run /graph populate to seed it.{RESET}\n");
        return;
    }
    let stats = graph.compute_stats();

    println!("  Connection Graph Health:");
    println!("    Nodes:       {}", stats.node_count);
    println!("    Edges:       {}", stats.edge_count);
    println!("    Avg weight:  {:.3}", stats.avg_weight);

    if !stats.by_kind.is_empty() {
        let mut kinds: Vec<_> = stats.by_kind.iter().collect();
        kinds.sort_by(|a, b| b.1.cmp(a.1));
        println!("    By kind:");
        for (kind, count) in &kinds {
            println!("      {kind}: {count}");
        }
    }

    if let Some(ref s) = stats.strongest {
        println!("    Strongest:   {s}");
    }
    if let Some(ref m) = stats.most_connected {
        println!("    Hub node:    {m}");
    }
    if stats.isolated_count > 0 {
        println!(
            "    Isolated:    {} nodes with zero edges",
            stats.isolated_count
        );
    }
    println!();
}

fn handle_graph_communities() {
    let graph = crate::memory::ConnectionGraph::load();
    if graph.edges.is_empty() {
        println!("{DIM}  Connection graph is empty. Run /graph populate to seed it.{RESET}\n");
        return;
    }
    let communities = graph.detect_communities(20);
    if communities.is_empty() {
        println!("{DIM}  No communities detected.{RESET}\n");
        return;
    }
    println!("  Concept Communities ({} clusters):\n", communities.len());
    for (i, (label, members)) in communities.iter().enumerate() {
        if members.len() == 1 {
            continue; // Skip singletons
        }
        println!("  {}. {} ({} members)", i + 1, label, members.len());
        for member in members.iter().take(15) {
            let marker = if member == label { " ◆" } else { "" };
            println!("     - {member}{marker}");
        }
        if members.len() > 15 {
            println!("     ... and {} more", members.len() - 15);
        }
        println!();
    }
}

fn handle_graph_similar(args: &str) {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        println!("{DIM}  usage: /graph similar <concept_a> <concept_b>{RESET}\n");
        return;
    }
    let a = parts[0].trim();
    let b = parts[1].trim();
    let graph = crate::memory::ConnectionGraph::load();

    let jaccard = graph.concept_similarity(a, b);
    let mi = graph.mutual_information(a, b);
    let shared: Vec<String> = {
        let na: std::collections::HashSet<String> = graph.neighbors(a).into_iter().collect();
        let nb: std::collections::HashSet<String> = graph.neighbors(b).into_iter().collect();
        na.intersection(&nb).cloned().collect()
    };

    println!("  Comparing \"{a}\" and \"{b}\":");
    println!("    Jaccard similarity: {jaccard:.3}");
    println!("    Mutual information: {mi:.3} bits");
    if shared.is_empty() {
        println!("    Shared neighbors:   (none)");
    } else {
        let mut sorted = shared;
        sorted.sort();
        println!("    Shared neighbors:   {}", sorted.join(", "));
    }
    println!();
}

fn handle_graph_search(query: &str) {
    if query.is_empty() {
        println!("{DIM}  usage: /graph search <query>{RESET}\n");
        return;
    }
    let graph = crate::memory::ConnectionGraph::load();
    let results = graph.search_concepts(query);
    if results.is_empty() {
        println!("{DIM}  No concepts matching \"{query}\".{RESET}\n");
    } else {
        println!("  Concepts matching \"{query}\":");
        for (name, conn_count) in &results {
            println!("    {name} ({conn_count} connections)");
        }
        println!();
    }
}

// ── /brain ────────────────────────────────────────────────────────────────

/// Handle /brain subcommands — query and grow the knowledge system.
pub fn handle_brain(input: &str) {
    let rest = input.strip_prefix("/brain").unwrap_or("").trim();
    if rest.is_empty() || rest == "status" {
        handle_brain_status();
    } else if rest == "gaps" {
        handle_brain_gaps();
    } else if let Some(args) = rest.strip_prefix("learn") {
        handle_brain_learn(args.trim());
    } else {
        println!("{DIM}  usage: /brain status    Show knowledge system health");
        println!("         /brain gaps      Identify domains with fewest learned patterns");
        println!(
            "         /brain learn <domain> <pattern>   Add a pattern to a domain skill{RESET}\n"
        );
    }
}

fn handle_brain_status() {
    println!("\n{BOLD}  Brain Status{RESET}\n");

    // Check domain skills and count patterns in each
    let skills_dir = std::path::Path::new("skills");
    let domains = [
        "code-rust",
        "code-web",
        "code-systems",
        "code-data",
        "code-devops",
        "code-testing",
    ];

    let mut total_patterns = 0usize;
    for domain in &domains {
        let path = skills_dir.join(domain).join("SKILL.md");
        let pattern_count = if let Ok(content) = std::fs::read_to_string(&path) {
            // Count lines after "## Patterns Learned" that start with "- "
            let mut counting = false;
            let mut count = 0usize;
            for line in content.lines() {
                if line.contains("Patterns Learned") {
                    counting = true;
                    continue;
                }
                if counting && line.starts_with("## ") {
                    break;
                }
                if counting && line.starts_with("- ") {
                    count += 1;
                }
            }
            count
        } else {
            0
        };
        total_patterns += pattern_count;
        let status = if pattern_count == 0 {
            format!("{DIM}empty{RESET}")
        } else {
            format!("{GREEN}{pattern_count} patterns{RESET}")
        };
        println!("  {domain}: {status}");
    }

    // Connection graph health
    let conn_path = std::path::Path::new("memory/connections.jsonl");
    let conn_count = if let Ok(content) = std::fs::read_to_string(conn_path) {
        content.lines().filter(|l| !l.trim().is_empty()).count()
    } else {
        0
    };

    // Learnings count
    let learn_path = std::path::Path::new("memory/learnings.jsonl");
    let learn_count = if let Ok(content) = std::fs::read_to_string(learn_path) {
        content.lines().filter(|l| !l.trim().is_empty()).count()
    } else {
        0
    };

    // Brain skill exists?
    let brain_exists = skills_dir.join("brain").join("SKILL.md").exists();

    println!();
    println!("  Connection graph: {conn_count} edges");
    println!("  Learnings archive: {learn_count} entries");
    println!("  Domain patterns: {total_patterns} total");
    println!(
        "  Brain skill: {}",
        if brain_exists {
            format!("{GREEN}present{RESET}")
        } else {
            format!("{RED}missing{RESET}")
        }
    );
    println!();
}

fn handle_brain_gaps() {
    println!("\n{BOLD}  Knowledge Gaps{RESET}\n");

    let skills_dir = std::path::Path::new("skills");
    let domains = [
        ("code-rust", "Rust patterns"),
        ("code-web", "Web development"),
        ("code-systems", "Systems programming"),
        ("code-data", "Data & databases"),
        ("code-devops", "DevOps & deployment"),
        ("code-testing", "Testing strategies"),
    ];

    let mut gaps: Vec<(&str, &str, usize)> = Vec::new();
    for (domain, desc) in &domains {
        let path = skills_dir.join(domain).join("SKILL.md");
        let pattern_count = if let Ok(content) = std::fs::read_to_string(&path) {
            let mut counting = false;
            let mut count = 0usize;
            for line in content.lines() {
                if line.contains("Patterns Learned") {
                    counting = true;
                    continue;
                }
                if counting && line.starts_with("## ") {
                    break;
                }
                if counting && line.starts_with("- ") {
                    count += 1;
                }
            }
            count
        } else {
            0
        };
        gaps.push((domain, desc, pattern_count));
    }

    // Sort by fewest patterns first
    gaps.sort_by_key(|(_, _, count)| *count);

    for (domain, desc, count) in &gaps {
        let indicator = if *count == 0 {
            format!("{RED}⚠ EMPTY{RESET}")
        } else if *count < 3 {
            format!("{YELLOW}sparse ({count}){RESET}")
        } else {
            format!("{GREEN}growing ({count}){RESET}")
        };
        println!("  {domain} ({desc}): {indicator}");
    }

    println!("\n{DIM}  Use /brain learn <domain> <pattern> to add patterns.");
    println!("  Use /research <topic> to study a domain before adding patterns.{RESET}\n");
}

fn handle_brain_learn(args: &str) {
    if args.is_empty() {
        println!(
            "{DIM}  usage: /brain learn <domain> <pattern description>\n\n\
             Domains: code-rust, code-web, code-systems, code-data, code-devops, code-testing\n\n\
             Example: /brain learn code-rust use Cow<str> for functions that sometimes need to allocate{RESET}\n"
        );
        return;
    }

    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() < 2 {
        println!("{DIM}  Need both domain and pattern. Example:");
        println!(
            "  /brain learn code-rust prefer &str over String in function parameters{RESET}\n"
        );
        return;
    }

    let domain = parts[0];
    let pattern = parts[1].trim();
    let valid_domains = [
        "code-rust",
        "code-web",
        "code-systems",
        "code-data",
        "code-devops",
        "code-testing",
    ];

    if !valid_domains.contains(&domain) {
        println!("{DIM}  Unknown domain: {domain}");
        println!("  Valid domains: {}", valid_domains.join(", "));
        println!("{RESET}");
        return;
    }

    let skill_path = format!("skills/{domain}/SKILL.md");
    let path = std::path::Path::new(&skill_path);

    if !path.exists() {
        println!("{DIM}  Skill file not found: {skill_path}{RESET}\n");
        return;
    }

    // Read the file and append the pattern after "## Patterns Learned"
    if let Ok(content) = std::fs::read_to_string(path) {
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        let mut inserted = false;

        // Find "Patterns Learned" section and insert before the next section or at end
        for i in 0..lines.len() {
            if lines[i].contains("Patterns Learned") {
                // Skip the placeholder text if present
                let mut insert_at = i + 1;
                while insert_at < lines.len() {
                    let trimmed = lines[insert_at].trim();
                    if trimmed.starts_with("## ") {
                        break;
                    }
                    if trimmed.starts_with("*(") {
                        // Skip placeholder like "*(This section grows...)*"
                        insert_at += 1;
                        continue;
                    }
                    insert_at += 1;
                }
                // Insert before the next section header or at insert_at
                lines.insert(insert_at, format!("- {pattern}"));
                inserted = true;
                break;
            }
        }

        if !inserted {
            // No "Patterns Learned" section found — append at end
            lines.push(String::new());
            lines.push("## Patterns Learned".to_string());
            lines.push(String::new());
            lines.push(format!("- {pattern}"));
        }

        let new_content = lines.join("\n");
        if std::fs::write(path, &new_content).is_ok() {
            println!("{GREEN}  Added pattern to {domain}: {pattern}{RESET}\n");
        } else {
            println!("{RED}  Failed to write to {skill_path}{RESET}\n");
        }
    } else {
        println!("{DIM}  Could not read {skill_path}{RESET}\n");
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- /remember argument parsing --

    #[test]
    fn test_remember_empty_input_does_not_panic() {
        // Should print usage, not panic
        handle_remember("/remember");
        handle_remember("/remember ");
        handle_remember("/remember  ");
    }

    #[test]
    fn test_remember_strips_prefix() {
        // Verify the strip_prefix logic doesn't panic on edge cases
        let input = "/remember this is a test note";
        let note = input.strip_prefix("/remember").unwrap_or("").trim();
        assert_eq!(note, "this is a test note");
    }

    #[test]
    fn test_remember_whitespace_only() {
        let input = "/remember   ";
        let note = input.strip_prefix("/remember").unwrap_or("").trim();
        assert!(note.is_empty());
    }

    // -- /forget argument parsing --

    #[test]
    fn test_forget_empty_input_does_not_panic() {
        handle_forget("/forget");
        handle_forget("/forget ");
    }

    #[test]
    fn test_forget_invalid_index_does_not_panic() {
        handle_forget("/forget abc");
        handle_forget("/forget -1");
        handle_forget("/forget 999999");
    }

    #[test]
    fn test_forget_parses_valid_index() {
        let input = "/forget 3";
        let arg = input.strip_prefix("/forget").unwrap_or("").trim();
        let index = arg.parse::<usize>();
        assert_eq!(index.unwrap(), 3);
    }

    // -- /memories does not panic --

    #[test]
    fn test_memories_does_not_panic() {
        handle_memories();
    }

    // -- /graph dispatch --

    #[test]
    fn test_graph_empty_does_not_panic() {
        handle_graph("/graph");
        handle_graph("/graph ");
    }

    #[test]
    fn test_graph_unknown_subcommand_shows_help() {
        // Should show help, not panic
        handle_graph("/graph nonexistent");
        handle_graph("/graph foo bar");
    }

    #[test]
    fn test_graph_downstream_empty_does_not_panic() {
        handle_graph("/graph downstream");
        handle_graph("/graph downstream ");
    }

    #[test]
    fn test_graph_downstream_nonexistent_concept() {
        handle_graph("/graph downstream nonexistent_concept_xyz");
    }

    #[test]
    fn test_graph_neighbors_empty_does_not_panic() {
        handle_graph("/graph neighbors");
        handle_graph("/graph neighbors ");
    }

    #[test]
    fn test_graph_neighbors_nonexistent_concept() {
        handle_graph("/graph neighbors nonexistent_concept_xyz");
    }

    #[test]
    fn test_graph_info_does_not_panic() {
        handle_graph("/graph info");
    }

    #[test]
    fn test_graph_stats_does_not_panic() {
        handle_graph("/graph stats");
    }

    #[test]
    fn test_graph_communities_does_not_panic() {
        handle_graph("/graph communities");
    }

    #[test]
    fn test_graph_similar_empty_does_not_panic() {
        handle_graph("/graph similar");
        handle_graph("/graph similar onlyone");
    }

    #[test]
    fn test_graph_similar_two_concepts() {
        handle_graph("/graph similar concept_a concept_b");
    }

    #[test]
    fn test_graph_search_empty_does_not_panic() {
        handle_graph("/graph search");
        handle_graph("/graph search ");
    }

    #[test]
    fn test_graph_search_nonexistent() {
        handle_graph("/graph search zzz_nonexistent_zzz");
    }

    #[test]
    fn test_graph_path_empty_does_not_panic() {
        handle_graph("/graph path");
        handle_graph("/graph path onlyone");
    }

    #[test]
    fn test_graph_path_two_concepts() {
        handle_graph("/graph path concept_a concept_b");
    }

    // Note: /graph populate and /graph activate tests are omitted because
    // they read/write real files (memory/learnings.jsonl, memory/connections.jsonl)
    // and can be slow or cause test interference. They are tested manually.

    #[test]
    fn test_graph_activate_arg_parsing() {
        // Test the argument parsing logic without actually writing to disk
        let args = "concept_a concept_b semantic";
        let parts: Vec<&str> = args.splitn(3, char::is_whitespace).collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "concept_a");
        assert_eq!(parts[1], "concept_b");
        assert_eq!(parts[2], "semantic");

        // Verify kind parsing
        assert!(crate::memory::parse_connection_kind("semantic").is_some());
        assert!(crate::memory::parse_connection_kind("causal").is_some());
        assert!(crate::memory::parse_connection_kind("temporal").is_some());
        assert!(crate::memory::parse_connection_kind("mathematical").is_some());
        assert!(crate::memory::parse_connection_kind("scientific").is_some());
        assert!(crate::memory::parse_connection_kind("invalid").is_none());
    }

    // -- /brain dispatch --

    #[test]
    fn test_brain_empty_shows_status() {
        // Empty input defaults to status — should not panic
        handle_brain("/brain");
        handle_brain("/brain ");
    }

    #[test]
    fn test_brain_status_does_not_panic() {
        handle_brain("/brain status");
    }

    #[test]
    fn test_brain_gaps_does_not_panic() {
        handle_brain("/brain gaps");
    }

    #[test]
    fn test_brain_unknown_subcommand_shows_help() {
        // Should print usage, not panic
        handle_brain("/brain unknown");
        handle_brain("/brain foo bar");
    }

    #[test]
    fn test_brain_learn_empty_shows_usage() {
        // No args → usage
        handle_brain("/brain learn");
        handle_brain("/brain learn ");
    }

    #[test]
    fn test_brain_learn_single_arg_shows_usage() {
        // Domain only, no pattern → usage
        handle_brain("/brain learn code-rust");
        handle_brain("/brain learn code-rust ");
    }

    // -- /brain learn argument parsing --

    #[test]
    fn test_brain_learn_arg_parsing_domain_and_pattern() {
        let args = "code-rust use Cow<str> for conditional ownership";
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "code-rust");
        assert_eq!(parts[1], "use Cow<str> for conditional ownership");
    }

    #[test]
    fn test_brain_learn_domain_validation() {
        let valid_domains = [
            "code-rust",
            "code-web",
            "code-systems",
            "code-data",
            "code-devops",
            "code-testing",
        ];
        assert!(valid_domains.contains(&"code-rust"));
        assert!(valid_domains.contains(&"code-testing"));
        assert!(!valid_domains.contains(&"code-python"));
        assert!(!valid_domains.contains(&"invalid"));
    }

    #[test]
    fn test_brain_learn_invalid_domain_does_not_panic() {
        // Invalid domain — prints error, doesn't crash
        handle_brain("/brain learn invalid-domain some pattern here");
    }

    // -- /brain gaps ordering --

    #[test]
    fn test_brain_gaps_sorting_logic() {
        // Test the sorting logic used by handle_brain_gaps
        let mut gaps: Vec<(&str, &str, usize)> = vec![
            ("code-rust", "Rust", 5),
            ("code-web", "Web", 0),
            ("code-systems", "Systems", 2),
        ];
        gaps.sort_by_key(|(_, _, count)| *count);
        assert_eq!(gaps[0].0, "code-web"); // 0 patterns first
        assert_eq!(gaps[1].0, "code-systems"); // 2 patterns
        assert_eq!(gaps[2].0, "code-rust"); // 5 patterns last
    }

    // -- /brain status pattern counting --

    #[test]
    fn test_brain_status_pattern_counting_logic() {
        // Verify the pattern-counting logic used in handle_brain_status/gaps
        let content = "\
## Patterns Learned

*(This section grows as the agent learns)*
- pattern one
- pattern two

## Next Section
";
        let mut counting = false;
        let mut count = 0usize;
        for line in content.lines() {
            if line.contains("Patterns Learned") {
                counting = true;
                continue;
            }
            if counting && line.starts_with("## ") {
                break;
            }
            if counting && line.starts_with("- ") {
                count += 1;
            }
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn test_brain_status_pattern_counting_empty() {
        let content = "\
## Patterns Learned

*(This section grows as the agent learns)*

## Next Section
";
        let mut counting = false;
        let mut count = 0usize;
        for line in content.lines() {
            if line.contains("Patterns Learned") {
                counting = true;
                continue;
            }
            if counting && line.starts_with("## ") {
                break;
            }
            if counting && line.starts_with("- ") {
                count += 1;
            }
        }
        assert_eq!(count, 0);
    }

    #[test]
    fn test_brain_status_pattern_counting_no_section() {
        // File with no "Patterns Learned" section
        let content = "# Skill\nSome description\n";
        let mut counting = false;
        let mut count = 0usize;
        for line in content.lines() {
            if line.contains("Patterns Learned") {
                counting = true;
                continue;
            }
            if counting && line.starts_with("## ") {
                break;
            }
            if counting && line.starts_with("- ") {
                count += 1;
            }
        }
        assert_eq!(count, 0);
    }

    // -- /brain learn insertion logic --

    #[test]
    fn test_brain_learn_insertion_logic() {
        // Test the insertion logic used by handle_brain_learn
        let content = "\
## Patterns Learned

*(This section grows as the agent learns)*

## Next Section
";
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        let pattern = "test pattern here";
        let mut inserted = false;

        for i in 0..lines.len() {
            if lines[i].contains("Patterns Learned") {
                let mut insert_at = i + 1;
                while insert_at < lines.len() {
                    let trimmed = lines[insert_at].trim();
                    if trimmed.starts_with("## ") {
                        break;
                    }
                    if trimmed.starts_with("*(") {
                        insert_at += 1;
                        continue;
                    }
                    insert_at += 1;
                }
                lines.insert(insert_at, format!("- {pattern}"));
                inserted = true;
                break;
            }
        }

        assert!(inserted);
        let result = lines.join("\n");
        assert!(result.contains("- test pattern here"));
        // Pattern should be before "## Next Section"
        let pattern_pos = result.find("- test pattern here").unwrap();
        let next_section_pos = result.find("## Next Section").unwrap();
        assert!(pattern_pos < next_section_pos);
    }

    #[test]
    fn test_brain_learn_insertion_no_section() {
        // When "Patterns Learned" section is missing, it should be created at end
        let content = "# Skill\nSome description\n";
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        let pattern = "new pattern";
        let mut inserted = false;

        for line in &lines {
            if line.contains("Patterns Learned") {
                inserted = true;
                break;
            }
        }

        if !inserted {
            lines.push(String::new());
            lines.push("## Patterns Learned".to_string());
            lines.push(String::new());
            lines.push(format!("- {pattern}"));
        }

        let result = lines.join("\n");
        assert!(result.contains("## Patterns Learned"));
        assert!(result.contains("- new pattern"));
    }
}
