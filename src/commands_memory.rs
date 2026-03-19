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
