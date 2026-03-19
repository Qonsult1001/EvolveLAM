//! Project memory system for yoyo.
//!
//! Persists project-specific notes across sessions in `.yoyo/memory.json`.
//! Each memory is a `{note, timestamp}` pair stored as a JSON array.
//! Users can add memories with `/remember`, list with `/memories`, remove with `/forget`.
//!
//! ## Latent Space Connection Layer
//!
//! Beyond flat memory storage, this module implements a latent-space-inspired
//! connection graph (`memory/connections.jsonl`). Every learning forms nodes;
//! connections between them carry weights that strengthen each time both concepts
//! co-activate (are referenced or relevant in the same context). This mimics how
//! neural latent spaces form dense representation clusters — connections happen
//! millions of times, building compressed mathematical understanding that can
//! generate novel combinations.
//!
//! The connection graph enables:
//! - **Associative recall**: finding related learnings by traversal, not just text search
//! - **Concept clustering**: emergent topic groups from connection density
//! - **Scientific learning ingestion**: new external knowledge auto-connects to existing nodes
//! - **Cognitive growth**: connection weights strengthen over time, forming "intuition"

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A single project memory entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryEntry {
    pub note: String,
    pub timestamp: String,
}

/// The in-memory store of project memories.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectMemory {
    pub entries: Vec<MemoryEntry>,
}

/// The directory name for yoyo project data.
const YOYO_DIR: &str = ".yoyo";

/// The filename for the memory store within `.yoyo/`.
const MEMORY_FILE: &str = "memory.json";

/// Get the path to the memory file for the current project.
pub fn memory_file_path() -> PathBuf {
    Path::new(YOYO_DIR).join(MEMORY_FILE)
}

/// Load project memories from `.yoyo/memory.json`.
/// Returns an empty `ProjectMemory` if the file doesn't exist or can't be parsed.
pub fn load_memories() -> ProjectMemory {
    load_memories_from(&memory_file_path())
}

/// Load project memories from a specific path (for testing).
pub fn load_memories_from(path: &Path) -> ProjectMemory {
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => ProjectMemory::default(),
    }
}

/// Save project memories to `.yoyo/memory.json`.
/// Creates the `.yoyo/` directory if it doesn't exist.
pub fn save_memories(memory: &ProjectMemory) -> Result<(), String> {
    save_memories_to(memory, &memory_file_path())
}

/// Save project memories to a specific path (for testing).
pub fn save_memories_to(memory: &ProjectMemory, path: &Path) -> Result<(), String> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }
    let json =
        serde_json::to_string_pretty(memory).map_err(|e| format!("Serialization error: {e}"))?;
    std::fs::write(path, json).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

/// Compute similarity between two strings using bigram overlap (Dice coefficient).
/// Returns a value between 0.0 (completely different) and 1.0 (identical).
pub fn text_similarity(a: &str, b: &str) -> f64 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    let bigrams_a = bigrams(&a_lower);
    let bigrams_b = bigrams(&b_lower);

    if bigrams_a.is_empty() && bigrams_b.is_empty() {
        return 1.0; // both empty = identical
    }
    if bigrams_a.is_empty() || bigrams_b.is_empty() {
        return 0.0;
    }

    let intersection = bigrams_a.iter().filter(|bg| bigrams_b.contains(bg)).count();
    (2.0 * intersection as f64) / (bigrams_a.len() + bigrams_b.len()) as f64
}

/// Extract character bigrams from a string.
fn bigrams(s: &str) -> Vec<(char, char)> {
    let chars: Vec<char> = s
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect();
    chars.windows(2).map(|w| (w[0], w[1])).collect()
}

/// Threshold for considering two memories as duplicates.
const SIMILARITY_THRESHOLD: f64 = 0.75;

/// Add a new memory entry with the current timestamp.
/// Returns true if the memory was added, false if it was too similar to an existing one.
pub fn add_memory(memory: &mut ProjectMemory, note: &str) -> bool {
    // Check for near-duplicates
    if is_duplicate(memory, note) {
        return false;
    }

    let timestamp = current_timestamp();
    memory.entries.push(MemoryEntry {
        note: note.to_string(),
        timestamp,
    });
    true
}

/// Add a memory entry without dedup check (used in tests).
#[allow(dead_code)]
pub fn add_memory_force(memory: &mut ProjectMemory, note: &str) {
    let timestamp = current_timestamp();
    memory.entries.push(MemoryEntry {
        note: note.to_string(),
        timestamp,
    });
}

/// Check if a note is too similar to any existing memory.
pub fn is_duplicate(memory: &ProjectMemory, note: &str) -> bool {
    memory
        .entries
        .iter()
        .any(|e| text_similarity(&e.note, note) >= SIMILARITY_THRESHOLD)
}

/// Remove a memory entry by index (0-based).
/// Returns the removed entry, or None if the index is out of bounds.
pub fn remove_memory(memory: &mut ProjectMemory, index: usize) -> Option<MemoryEntry> {
    if index < memory.entries.len() {
        Some(memory.entries.remove(index))
    } else {
        None
    }
}

/// Format memories for display in the system prompt.
/// Returns None if there are no memories.
pub fn format_memories_for_prompt(memory: &ProjectMemory) -> Option<String> {
    if memory.entries.is_empty() {
        return None;
    }
    let mut lines = Vec::new();
    lines.push("## Project Memories".to_string());
    lines.push(String::new());
    for entry in &memory.entries {
        lines.push(format!("- {} ({})", entry.note, entry.timestamp));
    }
    Some(lines.join("\n"))
}

// ============================================================================
// Latent Space Connection Layer
// ============================================================================

/// Path to the connections archive (append-only JSONL).
const CONNECTIONS_FILE: &str = "memory/connections.jsonl";

/// A connection between two learning concepts in the latent space.
/// Weights strengthen each time both concepts co-activate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Connection {
    /// Source node identifier (learning title or concept tag).
    pub from: String,
    /// Target node identifier.
    pub to: String,
    /// Connection weight — strengthens with co-activation (0.0 to 1.0 scale, can exceed 1.0).
    pub weight: f64,
    /// How many times this connection has been activated.
    pub activations: u64,
    /// Timestamp of last activation.
    pub last_activated: String,
    /// The type of connection (semantic, causal, temporal, mathematical).
    pub kind: ConnectionKind,
    /// Optional precondition: connection holds only when this condition is met (e.g. "when project has feature X").
    #[serde(default)]
    pub valid_when: Option<String>,
}

/// Types of connections in the latent space graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionKind {
    /// Concepts share semantic meaning (similar topics).
    Semantic,
    /// One concept causes or enables another.
    Causal,
    /// Concepts occurred near each other in time.
    Temporal,
    /// Mathematical or logical relationship between concepts.
    Mathematical,
    /// Connection formed from external scientific knowledge.
    Scientific,
}

impl std::fmt::Display for ConnectionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionKind::Semantic => write!(f, "semantic"),
            ConnectionKind::Causal => write!(f, "causal"),
            ConnectionKind::Temporal => write!(f, "temporal"),
            ConnectionKind::Mathematical => write!(f, "mathematical"),
            ConnectionKind::Scientific => write!(f, "scientific"),
        }
    }
}

/// Parse a string into a ConnectionKind, case-insensitive.
pub fn parse_connection_kind(s: &str) -> Option<ConnectionKind> {
    match s.to_lowercase().as_str() {
        "semantic" => Some(ConnectionKind::Semantic),
        "causal" => Some(ConnectionKind::Causal),
        "temporal" => Some(ConnectionKind::Temporal),
        "mathematical" | "math" => Some(ConnectionKind::Mathematical),
        "scientific" => Some(ConnectionKind::Scientific),
        _ => None,
    }
}

/// A scientific learning entry that can be ingested into the connection graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ScientificLearning {
    /// The domain (e.g., "latent_space", "information_theory", "category_theory").
    pub domain: String,
    /// The concept or principle.
    pub concept: String,
    /// How it relates to code generation or reasoning.
    pub application: String,
    /// Mathematical formulation if applicable (e.g., "KL_divergence(P||Q)").
    pub formula: Option<String>,
    /// Tags for connecting to existing learnings.
    pub tags: Vec<String>,
}

/// The in-memory connection graph.
#[derive(Debug, Clone, Default)]
pub struct ConnectionGraph {
    /// All connections indexed by source node.
    pub edges: HashMap<String, Vec<Connection>>,
    /// Node activation counts — how often each concept is referenced.
    pub node_activations: HashMap<String, u64>,
}

impl ConnectionGraph {
    /// Load connections from the JSONL archive.
    pub fn load() -> Self {
        Self::load_from(Path::new(CONNECTIONS_FILE))
    }

    /// Load connections from a specific path (for testing).
    pub fn load_from(path: &Path) -> Self {
        let mut graph = ConnectionGraph::default();
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return graph,
        };
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(conn) = serde_json::from_str::<Connection>(line) {
                let key = conn.from.clone();
                graph.edges.entry(key).or_default().push(conn);
            }
        }
        graph
    }

    /// Returns true if adding a causal edge from→to would create a cycle in the causal subgraph.
    /// Causal connections must remain acyclic (DAG).
    #[allow(dead_code)] // Called from activate_connection (test-only path)
    fn would_causal_cycle(&self, from: &str, to: &str) -> bool {
        // Build causal out-edges: node -> list of targets (causal only)
        let mut out: HashMap<String, Vec<String>> = HashMap::new();
        for (source, conns) in &self.edges {
            for c in conns {
                if c.kind == ConnectionKind::Causal {
                    out.entry(source.clone()).or_default().push(c.to.clone());
                }
            }
        }
        // Adding from→to would create a cycle iff there is already a path from to → from
        self.has_path_causal(&out, to, from)
    }

    /// DFS in the causal subgraph: is there a path from `start` to `target`?
    #[allow(dead_code)] // Called from would_causal_cycle
    fn has_path_causal(
        &self,
        out: &HashMap<String, Vec<String>>,
        start: &str,
        target: &str,
    ) -> bool {
        let mut stack = vec![start.to_string()];
        let mut seen = std::collections::HashSet::new();
        while let Some(node) = stack.pop() {
            if node == target {
                return true;
            }
            if !seen.insert(node.clone()) {
                continue;
            }
            if let Some(neighbors) = out.get(&node) {
                for n in neighbors {
                    stack.push(n.clone());
                }
            }
        }
        false
    }

    /// Strengthen a connection between two concepts (or create it).
    /// Returns the new weight after strengthening.
    /// Causal edges are rejected if they would create a cycle (DAG enforcement).
    #[allow(dead_code)] // Used by evolution scripts; not called from REPL binary path yet
    pub fn activate_connection(&mut self, from: &str, to: &str, kind: ConnectionKind) -> f64 {
        let timestamp = current_timestamp();

        // Causal DAG: reject edge if it would create a cycle
        if kind == ConnectionKind::Causal && self.would_causal_cycle(from, to) {
            return self
                .edges
                .get(from)
                .and_then(|v| v.iter().find(|c| c.to == to))
                .map(|c| c.weight)
                .unwrap_or(0.1);
        }

        // Increment node activations
        *self.node_activations.entry(from.to_string()).or_insert(0) += 1;
        *self.node_activations.entry(to.to_string()).or_insert(0) += 1;

        let edges = self.edges.entry(from.to_string()).or_default();

        // Find existing connection or create new one
        if let Some(conn) = edges.iter_mut().find(|c| c.to == to) {
            conn.activations += 1;
            // Weight grows logarithmically — rapid early growth, slower later
            // Mimics how neural connections stabilize
            conn.weight = (conn.activations as f64).ln() * 0.2 + 0.1;
            conn.last_activated = timestamp;
            conn.weight
        } else {
            let conn = Connection {
                from: from.to_string(),
                to: to.to_string(),
                weight: 0.1, // Initial connection strength
                activations: 1,
                last_activated: timestamp,
                kind,
                valid_when: None,
            };
            let w = conn.weight;
            edges.push(conn);
            w
        }
    }

    /// Return all concepts reachable by following causal edges from the given concept (downstream).
    /// Answers "if I change X, what's affected?"
    pub fn causal_downstream(&self, from: &str) -> Vec<String> {
        let mut out: HashMap<String, Vec<String>> = HashMap::new();
        for (source, conns) in &self.edges {
            for c in conns {
                if c.kind == ConnectionKind::Causal {
                    out.entry(source.clone()).or_default().push(c.to.clone());
                }
            }
        }
        let mut result = Vec::new();
        let mut stack = vec![from.to_string()];
        let mut seen = std::collections::HashSet::new();
        seen.insert(from.to_string());
        while let Some(node) = stack.pop() {
            if let Some(neighbors) = out.get(&node) {
                for n in neighbors {
                    if seen.insert(n.clone()) {
                        result.push(n.clone());
                        stack.push(n.clone());
                    }
                }
            }
        }
        result.sort();
        result
    }

    /// Find the strongest connections from a given concept.
    /// Returns connections sorted by weight (strongest first).
    #[allow(dead_code)] // API for future REPL commands
    pub fn strongest_connections(&self, from: &str, limit: usize) -> Vec<&Connection> {
        let mut conns: Vec<&Connection> = self
            .edges
            .get(from)
            .map(|v| v.iter().collect())
            .unwrap_or_default();
        conns.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        conns.truncate(limit);
        conns
    }

    /// Find the strongest connections from a given concept, ranked by effective weight
    /// (recency-weighted). Recently activated connections surface first.
    #[allow(dead_code)] // API for future REPL commands
    pub fn strongest_connections_weighted(
        &self,
        from: &str,
        limit: usize,
        now_ts: &str,
    ) -> Vec<(&Connection, f64)> {
        let mut conns: Vec<(&Connection, f64)> = self
            .edges
            .get(from)
            .map(|v| {
                v.iter()
                    .map(|c| (c, effective_weight(c, now_ts, HALF_LIFE_DAYS)))
                    .collect()
            })
            .unwrap_or_default();
        conns.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        conns.truncate(limit);
        conns
    }

    /// Shortest path between two concepts (BFS across all edges). Returns the sequence of concepts or None.
    #[allow(dead_code)] // used by handle_graph_path in commands.rs
    pub fn shortest_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        if from == to {
            return Some(vec![from.to_string()]);
        }
        use std::collections::VecDeque;
        let mut queue = VecDeque::new();
        let mut parent: std::collections::HashMap<String, Option<String>> =
            std::collections::HashMap::new();
        queue.push_back(from.to_string());
        parent.insert(from.to_string(), None);
        while let Some(node) = queue.pop_front() {
            if node == to {
                let mut path = vec![to.to_string()];
                let mut cur = &node;
                while let Some(Some(p)) = parent.get(cur) {
                    path.push(p.clone());
                    cur = p;
                }
                path.reverse();
                return Some(path);
            }
            // Out-edges
            if let Some(edges) = self.edges.get(&node) {
                for c in edges {
                    if !parent.contains_key(&c.to) {
                        parent.insert(c.to.clone(), Some(node.clone()));
                        queue.push_back(c.to.clone());
                    }
                }
            }
            // Reverse edges (graph is traversed bidirectionally for path finding)
            for (source, edges) in &self.edges {
                for c in edges {
                    if c.to == node && !parent.contains_key(source) {
                        parent.insert(source.clone(), Some(node.clone()));
                        queue.push_back(source.clone());
                    }
                }
            }
        }
        None
    }

    /// Find all concepts connected to a given concept (neighbors in the graph).
    #[allow(dead_code)] // API for future REPL commands
    pub fn neighbors(&self, concept: &str) -> Vec<String> {
        let mut result = Vec::new();
        if let Some(edges) = self.edges.get(concept) {
            for conn in edges {
                result.push(conn.to.clone());
            }
        }
        // Also find reverse connections
        for (source, edges) in &self.edges {
            for conn in edges {
                if conn.to == concept && !result.contains(source) {
                    result.push(source.clone());
                }
            }
        }
        result
    }

    /// Compute concept similarity using shared connections (Jaccard index).
    /// Two concepts are similar if they connect to the same things.
    #[allow(dead_code)] // API for future REPL commands
    pub fn concept_similarity(&self, a: &str, b: &str) -> f64 {
        let neighbors_a: std::collections::HashSet<String> =
            self.neighbors(a).into_iter().collect();
        let neighbors_b: std::collections::HashSet<String> =
            self.neighbors(b).into_iter().collect();

        if neighbors_a.is_empty() && neighbors_b.is_empty() {
            return 0.0;
        }

        let intersection = neighbors_a.intersection(&neighbors_b).count();
        let union = neighbors_a.union(&neighbors_b).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// Compute mutual information between two concepts.
    /// Uses pointwise mutual information: log2(P(a,b) / (P(a) * P(b))).
    /// P(a) = degree(a) / total_edges, P(a,b) = shared_neighbors / total_edges.
    /// Returns a value where positive means more related than expected by chance,
    /// zero means independent, negative means less related than expected.
    pub fn mutual_information(&self, a: &str, b: &str) -> f64 {
        let total_edges = self.connection_count();
        if total_edges == 0 {
            return 0.0;
        }

        let neighbors_a: std::collections::HashSet<String> =
            self.neighbors(a).into_iter().collect();
        let neighbors_b: std::collections::HashSet<String> =
            self.neighbors(b).into_iter().collect();

        let deg_a = neighbors_a.len();
        let deg_b = neighbors_b.len();
        if deg_a == 0 || deg_b == 0 {
            return 0.0;
        }

        let shared = neighbors_a.intersection(&neighbors_b).count();
        if shared == 0 {
            return 0.0; // No shared neighbors — independent
        }

        let total = total_edges as f64;
        let p_a = deg_a as f64 / total;
        let p_b = deg_b as f64 / total;
        let p_ab = shared as f64 / total;

        (p_ab / (p_a * p_b)).log2()
    }

    /// Save the entire graph to the JSONL archive.
    #[allow(dead_code)] // Used by evolution scripts; not called from REPL binary path yet
    pub fn save(&self) -> Result<(), String> {
        self.save_to(Path::new(CONNECTIONS_FILE))
    }

    /// Save the entire graph to a specific path (for testing).
    #[allow(dead_code)] // Used in tests and by save()
    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {e}"))?;
        }
        let mut lines = Vec::new();
        for edges in self.edges.values() {
            for conn in edges {
                let json =
                    serde_json::to_string(conn).map_err(|e| format!("Serialization error: {e}"))?;
                lines.push(json);
            }
        }
        std::fs::write(path, lines.join("\n") + "\n")
            .map_err(|e| format!("Failed to write {}: {e}", path.display()))
    }

    /// Ingest a scientific learning: create a node and connect it to related concepts.
    /// Returns the number of new connections formed.
    #[allow(dead_code)] // API for evolution scripts
    pub fn ingest_scientific_learning(&mut self, learning: &ScientificLearning) -> usize {
        let node_name = format!("{}:{}", learning.domain, learning.concept);
        let mut connections_formed = 0;

        // Connect to all tagged concepts
        for tag in &learning.tags {
            self.activate_connection(&node_name, tag, ConnectionKind::Scientific);
            connections_formed += 1;
        }

        // If there's a mathematical formula, connect with Mathematical kind
        if let Some(ref formula) = learning.formula {
            let formula_node = format!("formula:{}", formula);
            self.activate_connection(&node_name, &formula_node, ConnectionKind::Mathematical);
            connections_formed += 1;
        }

        connections_formed
    }

    /// Find all connections involving a given concept (both directions), with full details.
    /// Returns (outgoing, incoming) connection lists.
    pub fn neighbors_detailed(&self, concept: &str) -> (Vec<&Connection>, Vec<&Connection>) {
        let outgoing: Vec<&Connection> = self
            .edges
            .get(concept)
            .map(|v| v.iter().collect())
            .unwrap_or_default();
        let mut incoming: Vec<&Connection> = Vec::new();
        for edges in self.edges.values() {
            for conn in edges {
                if conn.to == concept && conn.from != concept {
                    incoming.push(conn);
                }
            }
        }
        (outgoing, incoming)
    }

    /// Count connections grouped by kind across the whole graph.
    pub fn connections_by_kind(&self) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for edges in self.edges.values() {
            for conn in edges {
                *counts.entry(conn.kind.to_string()).or_insert(0) += 1;
            }
        }
        counts
    }

    /// Search for concepts by case-insensitive substring match.
    /// Returns matching concept names with their total connection count (outgoing + incoming).
    pub fn search_concepts(&self, query: &str) -> Vec<(String, usize)> {
        if query.is_empty() {
            return Vec::new();
        }
        let query_lower = query.to_lowercase();

        // Collect all unique concept names
        let mut all_nodes: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for (source, edges) in &self.edges {
            all_nodes.insert(source);
            for conn in edges {
                all_nodes.insert(&conn.to);
            }
        }

        // Filter by substring match and count connections
        let mut results: Vec<(String, usize)> = all_nodes
            .into_iter()
            .filter(|name| name.to_lowercase().contains(&query_lower))
            .map(|name| {
                let (out, inc) = self.neighbors_detailed(name);
                (name.to_string(), out.len() + inc.len())
            })
            .collect();
        results.sort_by(|a, b| a.0.cmp(&b.0));
        results
    }

    /// Get the total number of connections in the graph.
    pub fn connection_count(&self) -> usize {
        self.edges.values().map(|v| v.len()).sum()
    }

    /// Get the total number of unique concepts (nodes) in the graph.
    pub fn node_count(&self) -> usize {
        let mut nodes: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for (source, edges) in &self.edges {
            nodes.insert(source);
            for conn in edges {
                nodes.insert(&conn.to);
            }
        }
        nodes.len()
    }

    /// Format the connection graph summary for inclusion in prompts.
    pub fn format_for_prompt(&self) -> Option<String> {
        if self.edges.is_empty() {
            return None;
        }
        let mut lines = Vec::new();
        lines.push("## Latent Space Connections".to_string());
        lines.push(format!(
            "Nodes: {} | Connections: {} | Total activations: {}",
            self.node_count(),
            self.connection_count(),
            self.node_activations.values().sum::<u64>()
        ));
        lines.push(String::new());

        // Show strongest connections across the whole graph
        let mut all_conns: Vec<&Connection> = self.edges.values().flat_map(|v| v.iter()).collect();
        all_conns.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for conn in all_conns.iter().take(10) {
            lines.push(format!(
                "- {} → {} (w:{:.2}, activated:{}x, {:?})",
                conn.from, conn.to, conn.weight, conn.activations, conn.kind
            ));
        }
        Some(lines.join("\n"))
    }
}

// ============================================================================
// Learning Ingestion — Populate graph from learnings.jsonl
// ============================================================================

/// A learning entry from `memory/learnings.jsonl`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEntry {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub takeaway: String,
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub day: Option<u32>,
    #[serde(default)]
    pub ts: String,
    #[serde(rename = "type", default)]
    pub entry_type: String,
}

/// Default path to the learnings archive.
const LEARNINGS_FILE: &str = "memory/learnings.jsonl";

/// Stop words to filter out during concept extraction.
const STOP_WORDS: &[&str] = &[
    "a",
    "an",
    "the",
    "and",
    "or",
    "but",
    "in",
    "on",
    "at",
    "to",
    "for",
    "of",
    "with",
    "by",
    "from",
    "is",
    "it",
    "its",
    "that",
    "this",
    "was",
    "are",
    "be",
    "been",
    "being",
    "have",
    "has",
    "had",
    "do",
    "does",
    "did",
    "will",
    "would",
    "could",
    "should",
    "may",
    "might",
    "can",
    "not",
    "no",
    "nor",
    "so",
    "if",
    "then",
    "than",
    "too",
    "very",
    "just",
    "about",
    "also",
    "more",
    "most",
    "much",
    "many",
    "some",
    "any",
    "each",
    "every",
    "all",
    "both",
    "few",
    "other",
    "own",
    "same",
    "such",
    "only",
    "even",
    "still",
    "into",
    "over",
    "after",
    "before",
    "between",
    "through",
    "during",
    "up",
    "out",
    "down",
    "off",
    "when",
    "where",
    "how",
    "what",
    "which",
    "who",
    "whom",
    "why",
    "my",
    "your",
    "his",
    "her",
    "our",
    "their",
    "you",
    "i",
    "we",
    "they",
    "me",
    "him",
    "us",
    "them",
    "itself",
    "myself",
    "yourself",
    "don",
    "doesn",
    "didn",
    "won",
    "wouldn",
    "couldn",
    "shouldn",
    "isn",
    "aren",
    "wasn",
    "weren",
    "because",
    "while",
    "until",
    "although",
    "though",
    "whether",
    "instead",
    "already",
    "actually",
    "rather",
    "without",
    "something",
    "anything",
    "everything",
    "nothing",
    "things",
    "thing",
    "way",
    "ways",
    "time",
    "times",
    "one",
    "two",
    "three",
    "four",
    "five",
    "first",
    "second",
    "last",
    "next",
    "new",
    "old",
    "good",
    "better",
    "best",
    "real",
    "really",
    "right",
    "back",
    "going",
    "went",
    "done",
    "made",
    "make",
    "got",
    "get",
    "say",
    "said",
    "like",
    "just",
];

/// Extract concept keywords from a text string.
///
/// Splits on non-alphanumeric boundaries, lowercases, filters stop words,
/// discards short tokens (<3 chars) and pure numbers, then deduplicates.
/// Also detects common bigrams (two-word compound concepts).
#[cfg(test)]
pub fn extract_concepts(text: &str) -> Vec<String> {
    extract_concepts_inner(text, text)
}

/// Extract concepts with title weighting — title concepts are included first
/// and body concepts are appended. Bigrams from both are detected.
pub fn extract_concepts_weighted(title: &str, body: &str) -> Vec<String> {
    let combined = format!("{} {}", title, body);
    let mut result = extract_concepts_inner(title, &combined);
    let body_concepts = extract_concepts_inner(body, &combined);
    let seen: std::collections::HashSet<String> = result.iter().cloned().collect();
    for c in body_concepts {
        if !seen.contains(&c) {
            result.push(c);
        }
    }
    result
}

/// Filter a word: returns Some(lowercased) if it's a valid concept word.
fn filter_word(word: &str) -> Option<String> {
    let w = word.to_lowercase();
    if w.len() < 3 {
        return None;
    }
    if w.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if STOP_WORDS.contains(&w.as_str()) {
        return None;
    }
    Some(w)
}

/// Internal: extract concepts with bigram detection.
/// `text` is the text to extract from, `corpus` is used for bigram frequency.
fn extract_concepts_inner(text: &str, corpus: &str) -> Vec<String> {
    // First pass: extract all valid words in order
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter_map(filter_word)
        .collect();

    // Build bigram candidates from the corpus
    let corpus_words: Vec<String> = corpus
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter_map(filter_word)
        .collect();

    let mut bigram_set = std::collections::HashSet::new();
    for pair in corpus_words.windows(2) {
        let bigram = format!("{}_{}", pair[0], pair[1]);
        // Only keep bigrams where both words are meaningful (not numbers, not stop words)
        // and the bigram itself is a recognizable compound concept
        if is_compound_concept(&pair[0], &pair[1]) {
            bigram_set.insert(bigram);
        }
    }

    // Second pass: greedily match bigrams, falling back to unigrams
    let mut seen = std::collections::HashSet::new();
    let mut concepts = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if i + 1 < words.len() {
            let bigram = format!("{}_{}", words[i], words[i + 1]);
            if bigram_set.contains(&bigram) {
                if seen.insert(bigram.clone()) {
                    concepts.push(bigram);
                }
                i += 2;
                continue;
            }
        }
        let w = &words[i];
        if seen.insert(w.clone()) {
            concepts.push(w.clone());
        }
        i += 1;
    }
    concepts
}

/// Known compound concept patterns — bigrams that should be kept together.
/// These are domain-specific terms common in coding agent / PL contexts.
fn is_compound_concept(a: &str, b: &str) -> bool {
    // Known compound patterns
    const KNOWN_COMPOUNDS: &[(&str, &str)] = &[
        ("connection", "graph"),
        ("error", "handling"),
        ("error", "recovery"),
        ("self", "awareness"),
        ("self", "improvement"),
        ("self", "modification"),
        ("self", "assessment"),
        ("latent", "space"),
        ("dead", "code"),
        ("tab", "completion"),
        ("file", "coupling"),
        ("type", "mismatch"),
        ("borrow", "checker"),
        ("missing", "import"),
        ("test", "failure"),
        ("git", "workflow"),
        ("code", "review"),
        ("pull", "request"),
        ("build", "failure"),
        ("fix", "strategy"),
        ("session", "plan"),
        ("research", "backlog"),
        ("memory", "system"),
        ("concept", "extraction"),
        ("label", "propagation"),
        ("graph", "search"),
        ("causal", "inference"),
        ("temporal", "decay"),
        ("knowledge", "gap"),
        ("permission", "prompts"),
        ("meta", "work"),
        ("commit", "message"),
        ("unit", "test"),
        ("integration", "test"),
    ];
    KNOWN_COMPOUNDS.contains(&(a, b))
}

/// Load learnings from the JSONL file.
pub fn load_learnings() -> Vec<LearningEntry> {
    load_learnings_from(Path::new(LEARNINGS_FILE))
}

/// Load learnings from a specific path (for testing).
pub fn load_learnings_from(path: &Path) -> Vec<LearningEntry> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<LearningEntry>(l).ok())
        .collect()
}

impl ConnectionGraph {
    /// Populate the graph from learnings.jsonl by extracting concepts from
    /// each learning's title and takeaway, then creating semantic connections
    /// between co-occurring concepts within the same learning.
    ///
    /// Returns (learnings_processed, connections_created).
    pub fn populate_from_learnings(&mut self, learnings: &[LearningEntry]) -> (usize, usize) {
        let mut connections_created = 0;

        for learning in learnings {
            let all_concepts = extract_concepts_weighted(&learning.title, &learning.takeaway);

            // Normalize direction alphabetically so repeated co-occurrences
            // always strengthen the same edge.
            for i in 0..all_concepts.len() {
                for j in (i + 1)..all_concepts.len() {
                    let (from, to) = if all_concepts[i] <= all_concepts[j] {
                        (&all_concepts[i], &all_concepts[j])
                    } else {
                        (&all_concepts[j], &all_concepts[i])
                    };
                    self.activate_connection(from, to, ConnectionKind::Semantic);
                    connections_created += 1;
                }
            }
        }

        (learnings.len(), connections_created)
    }

    /// Compute graph statistics.
    pub fn compute_stats(&self) -> GraphStats {
        let all_conns: Vec<&Connection> = self.edges.values().flat_map(|v| v.iter()).collect();
        let total_edges = all_conns.len();

        let avg_weight = if total_edges > 0 {
            all_conns.iter().map(|c| c.weight).sum::<f64>() / total_edges as f64
        } else {
            0.0
        };

        let strongest = all_conns
            .iter()
            .max_by(|a, b| {
                a.weight
                    .partial_cmp(&b.weight)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|c| format!("{} → {} (w:{:.2}, {})", c.from, c.to, c.weight, c.kind));

        let mut degree: HashMap<String, usize> = HashMap::new();
        for edges in self.edges.values() {
            for conn in edges {
                *degree.entry(conn.from.clone()).or_insert(0) += 1;
                *degree.entry(conn.to.clone()).or_insert(0) += 1;
            }
        }

        let most_connected = degree
            .iter()
            .max_by_key(|(_, d)| *d)
            .map(|(name, d)| format!("{name} (degree: {d})"));

        let all_nodes: std::collections::HashSet<String> = {
            let mut s = std::collections::HashSet::new();
            for (src, edges) in &self.edges {
                s.insert(src.clone());
                for c in edges {
                    s.insert(c.to.clone());
                }
            }
            s
        };

        let isolated_count = self
            .node_activations
            .keys()
            .filter(|k| !all_nodes.contains(*k))
            .count();

        GraphStats {
            node_count: self.node_count(),
            edge_count: total_edges,
            by_kind: self.connections_by_kind(),
            avg_weight,
            strongest,
            most_connected,
            isolated_count,
        }
    }

    /// Detect communities using label propagation.
    ///
    /// Each node starts with its own label. Iteratively, each node adopts the
    /// label most common among its neighbors (weighted by edge weight).
    /// Converges when no node changes label, or after max_iterations.
    ///
    /// Returns a map from community label to list of member concepts.
    pub fn detect_communities(&self, max_iterations: usize) -> Vec<(String, Vec<String>)> {
        // Collect all nodes
        let mut all_nodes: Vec<String> = Vec::new();
        let mut node_set: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (src, edges) in &self.edges {
            if node_set.insert(src.clone()) {
                all_nodes.push(src.clone());
            }
            for conn in edges {
                if node_set.insert(conn.to.clone()) {
                    all_nodes.push(conn.to.clone());
                }
            }
        }

        if all_nodes.is_empty() {
            return Vec::new();
        }

        // Initialize: each node is its own label
        let mut labels: HashMap<String, String> =
            all_nodes.iter().map(|n| (n.clone(), n.clone())).collect();

        // Build adjacency list with weights
        let mut adjacency: HashMap<String, Vec<(String, f64)>> = HashMap::new();
        for edges in self.edges.values() {
            for conn in edges {
                adjacency
                    .entry(conn.from.clone())
                    .or_default()
                    .push((conn.to.clone(), conn.weight));
                adjacency
                    .entry(conn.to.clone())
                    .or_default()
                    .push((conn.from.clone(), conn.weight));
            }
        }

        // Iterate
        for _ in 0..max_iterations {
            let mut changed = false;
            for node in &all_nodes {
                let neighbors = match adjacency.get(node) {
                    Some(n) => n,
                    None => continue,
                };
                if neighbors.is_empty() {
                    continue;
                }
                // Count weighted votes per label
                let mut label_weights: HashMap<String, f64> = HashMap::new();
                for (neighbor, weight) in neighbors {
                    let neighbor_label = labels.get(neighbor).cloned().unwrap_or_default();
                    *label_weights.entry(neighbor_label).or_insert(0.0) += weight;
                }
                // Pick the label with highest total weight
                if let Some((best_label, _)) = label_weights
                    .iter()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                {
                    let current = labels.get(node).cloned().unwrap_or_default();
                    if *best_label != current {
                        labels.insert(node.clone(), best_label.clone());
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }

        // Group nodes by label
        let mut communities: HashMap<String, Vec<String>> = HashMap::new();
        for (node, label) in &labels {
            communities
                .entry(label.clone())
                .or_default()
                .push(node.clone());
        }

        // Sort communities by size (largest first), sort members within each
        let mut result: Vec<(String, Vec<String>)> = communities.into_iter().collect();
        for (_, members) in &mut result {
            members.sort();
        }
        result.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        result
    }
}

/// Summary statistics for the connection graph.
#[derive(Debug)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub by_kind: HashMap<String, usize>,
    pub avg_weight: f64,
    pub strongest: Option<String>,
    pub most_connected: Option<String>,
    pub isolated_count: usize,
}

/// Parse a timestamp string (YYYY-MM-DD or YYYY-MM-DD HH:MM) to days since 2000-01-01 (approximate).
pub fn timestamp_to_days(ts: &str) -> Option<f64> {
    let s = ts.trim().get(0..10)?;
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: f64 = parts[0].parse().ok()?;
    let m: f64 = parts[1].parse().ok()?;
    let d: f64 = parts[2].parse().ok()?;
    Some((y - 2000.0) * 365.25 + (m - 1.0) * 30.44 + d)
}

/// Default half-life for temporal decay (days). Connections lose half their
/// effective weight every 14 days without reactivation.
pub const HALF_LIFE_DAYS: f64 = 14.0;

/// Time-weighted relevance: effective weight = raw weight * 2^(-age_days / half_life_days).
/// Raw weight is unchanged; use this when recency should influence influence.
pub fn effective_weight(conn: &Connection, now_ts: &str, half_life_days: f64) -> f64 {
    if half_life_days <= 0.0 {
        return conn.weight;
    }
    let (now_days, last_days) = match (
        timestamp_to_days(now_ts),
        timestamp_to_days(&conn.last_activated),
    ) {
        (Some(a), Some(b)) => (a, b),
        _ => return conn.weight,
    };
    let age_days = now_days - last_days;
    if age_days <= 0.0 {
        return conn.weight;
    }
    let decay = 2.0_f64.powf(-age_days / half_life_days);
    conn.weight * decay
}

/// Get the current timestamp in a human-readable format.
pub fn current_timestamp() -> String {
    // Use a simple approach: shell out to date command for portability
    std::process::Command::new("date")
        .arg("+%Y-%m-%d %H:%M")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_memory_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yoyo_test_memory_{}", name));
        let _ = fs::create_dir_all(&dir);
        dir.join(MEMORY_FILE)
    }

    fn cleanup(path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn test_memory_entry_serialize_deserialize() {
        let entry = MemoryEntry {
            note: "uses sqlx for database access".to_string(),
            timestamp: "2026-03-15 08:32".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: MemoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, entry);
    }

    #[test]
    fn test_project_memory_serialize_deserialize() {
        let memory = ProjectMemory {
            entries: vec![
                MemoryEntry {
                    note: "tests require docker running".to_string(),
                    timestamp: "2026-03-15 08:00".to_string(),
                },
                MemoryEntry {
                    note: "use pnpm not npm".to_string(),
                    timestamp: "2026-03-15 09:00".to_string(),
                },
            ],
        };
        let json = serde_json::to_string_pretty(&memory).unwrap();
        let parsed: ProjectMemory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].note, "tests require docker running");
        assert_eq!(parsed.entries[1].note, "use pnpm not npm");
    }

    #[test]
    fn test_add_memory() {
        let mut memory = ProjectMemory::default();
        assert!(memory.entries.is_empty());

        let added = add_memory(&mut memory, "this project uses sqlx");
        assert!(added);
        assert_eq!(memory.entries.len(), 1);
        assert_eq!(memory.entries[0].note, "this project uses sqlx");
        assert!(!memory.entries[0].timestamp.is_empty());

        let added = add_memory(&mut memory, "tests need docker");
        assert!(added);
        assert_eq!(memory.entries.len(), 2);
        assert_eq!(memory.entries[1].note, "tests need docker");
    }

    #[test]
    fn test_add_memory_dedup_rejects_similar() {
        let mut memory = ProjectMemory::default();
        add_memory(&mut memory, "this project uses sqlx for database access");
        assert_eq!(memory.entries.len(), 1);

        // Very similar note should be rejected
        let added = add_memory(&mut memory, "this project uses sqlx for database");
        assert!(!added);
        assert_eq!(memory.entries.len(), 1);
    }

    #[test]
    fn test_add_memory_dedup_allows_different() {
        let mut memory = ProjectMemory::default();
        add_memory(&mut memory, "this project uses sqlx for database access");
        assert_eq!(memory.entries.len(), 1);

        // Different note should be accepted
        let added = add_memory(&mut memory, "tests require docker running");
        assert!(added);
        assert_eq!(memory.entries.len(), 2);
    }

    #[test]
    fn test_text_similarity() {
        // Identical strings
        assert!((text_similarity("hello world", "hello world") - 1.0).abs() < 0.01);

        // Very similar
        let sim = text_similarity(
            "this project uses sqlx for database",
            "this project uses sqlx for database access",
        );
        assert!(
            sim > 0.7,
            "Similar strings should have high similarity: {sim}"
        );

        // Very different
        let sim = text_similarity("uses sqlx", "requires docker");
        assert!(
            sim < 0.3,
            "Different strings should have low similarity: {sim}"
        );

        // Empty strings
        assert!((text_similarity("", "") - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_remove_memory_valid_index() {
        let mut memory = ProjectMemory {
            entries: vec![
                MemoryEntry {
                    note: "note 0".to_string(),
                    timestamp: "t0".to_string(),
                },
                MemoryEntry {
                    note: "note 1".to_string(),
                    timestamp: "t1".to_string(),
                },
                MemoryEntry {
                    note: "note 2".to_string(),
                    timestamp: "t2".to_string(),
                },
            ],
        };

        let removed = remove_memory(&mut memory, 1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().note, "note 1");
        assert_eq!(memory.entries.len(), 2);
        assert_eq!(memory.entries[0].note, "note 0");
        assert_eq!(memory.entries[1].note, "note 2");
    }

    #[test]
    fn test_remove_memory_invalid_index() {
        let mut memory = ProjectMemory {
            entries: vec![MemoryEntry {
                note: "only one".to_string(),
                timestamp: "t0".to_string(),
            }],
        };

        let removed = remove_memory(&mut memory, 5);
        assert!(removed.is_none());
        assert_eq!(memory.entries.len(), 1);
    }

    #[test]
    fn test_remove_memory_empty() {
        let mut memory = ProjectMemory::default();
        let removed = remove_memory(&mut memory, 0);
        assert!(removed.is_none());
    }

    #[test]
    fn test_save_and_load_memories() {
        let path = temp_memory_path("save_load");
        let memory = ProjectMemory {
            entries: vec![
                MemoryEntry {
                    note: "first note".to_string(),
                    timestamp: "2026-03-15 08:00".to_string(),
                },
                MemoryEntry {
                    note: "second note".to_string(),
                    timestamp: "2026-03-15 09:00".to_string(),
                },
            ],
        };

        let result = save_memories_to(&memory, &path);
        assert!(result.is_ok(), "Save should succeed: {:?}", result);

        let loaded = load_memories_from(&path);
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(loaded.entries[0].note, "first note");
        assert_eq!(loaded.entries[1].note, "second note");

        cleanup(&path);
    }

    #[test]
    fn test_load_memories_nonexistent_file() {
        let path = Path::new("/tmp/yoyo_test_nonexistent_12345/memory.json");
        let memory = load_memories_from(path);
        assert!(memory.entries.is_empty());
    }

    #[test]
    fn test_load_memories_invalid_json() {
        let path = temp_memory_path("invalid_json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not valid json at all {{{").unwrap();

        let memory = load_memories_from(&path);
        assert!(
            memory.entries.is_empty(),
            "Invalid JSON should return empty memory"
        );

        cleanup(&path);
    }

    #[test]
    fn test_save_creates_directory() {
        let dir = std::env::temp_dir().join("yoyo_test_memory_create_dir");
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("subdir").join(MEMORY_FILE);

        let memory = ProjectMemory {
            entries: vec![MemoryEntry {
                note: "test".to_string(),
                timestamp: "now".to_string(),
            }],
        };

        let result = save_memories_to(&memory, &path);
        assert!(
            result.is_ok(),
            "Save should create parent dirs: {:?}",
            result
        );
        assert!(path.exists(), "File should exist after save");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_format_memories_for_prompt_empty() {
        let memory = ProjectMemory::default();
        assert!(format_memories_for_prompt(&memory).is_none());
    }

    #[test]
    fn test_format_memories_for_prompt_with_entries() {
        let memory = ProjectMemory {
            entries: vec![
                MemoryEntry {
                    note: "uses sqlx".to_string(),
                    timestamp: "2026-03-15 08:00".to_string(),
                },
                MemoryEntry {
                    note: "docker needed for tests".to_string(),
                    timestamp: "2026-03-15 09:00".to_string(),
                },
            ],
        };

        let prompt = format_memories_for_prompt(&memory).unwrap();
        assert!(prompt.contains("## Project Memories"));
        assert!(prompt.contains("uses sqlx"));
        assert!(prompt.contains("docker needed for tests"));
        assert!(prompt.contains("2026-03-15 08:00"));
    }

    #[test]
    fn test_memory_file_path() {
        let path = memory_file_path();
        assert!(path.to_string_lossy().contains(".yoyo"));
        assert!(path.to_string_lossy().contains("memory.json"));
    }

    // ====================================================================
    // Latent Space Connection Graph Tests
    // ====================================================================

    #[test]
    fn test_connection_graph_empty() {
        let graph = ConnectionGraph::default();
        assert_eq!(graph.connection_count(), 0);
        assert_eq!(graph.node_count(), 0);
        assert!(graph.format_for_prompt().is_none());
    }

    #[test]
    fn test_activate_connection_creates_new() {
        let mut graph = ConnectionGraph::default();
        let weight =
            graph.activate_connection("avoidance", "self_awareness", ConnectionKind::Semantic);
        assert!((weight - 0.1).abs() < 0.01);
        assert_eq!(graph.connection_count(), 1);
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn test_activate_connection_strengthens() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("avoidance", "self_awareness", ConnectionKind::Semantic);
        let w2 = graph.activate_connection("avoidance", "self_awareness", ConnectionKind::Semantic);
        // After 2 activations: ln(2) * 0.2 + 0.1 ≈ 0.239
        assert!(w2 > 0.1, "Weight should increase: {w2}");
        assert_eq!(graph.connection_count(), 1); // Still one connection
    }

    #[test]
    fn test_activate_connection_logarithmic_growth() {
        let mut graph = ConnectionGraph::default();
        let mut prev_weight = 0.0;
        for i in 0..20 {
            let w = graph.activate_connection("a", "b", ConnectionKind::Mathematical);
            if i > 0 {
                assert!(w > prev_weight, "Weight should grow: {w} > {prev_weight}");
            }
            prev_weight = w;
        }
        // After 20 activations: ln(20) * 0.2 + 0.1 ≈ 0.699
        assert!(
            prev_weight < 1.0,
            "Growth should be bounded-ish: {prev_weight}"
        );
    }

    #[test]
    fn test_strongest_connections() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("core", "a", ConnectionKind::Semantic);
        // Activate b more times to make it stronger
        for _ in 0..5 {
            graph.activate_connection("core", "b", ConnectionKind::Causal);
        }
        graph.activate_connection("core", "c", ConnectionKind::Temporal);

        let strongest = graph.strongest_connections("core", 2);
        assert_eq!(strongest.len(), 2);
        assert_eq!(strongest[0].to, "b"); // b is strongest (5 activations)
    }

    #[test]
    fn test_neighbors() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("a", "b", ConnectionKind::Semantic);
        graph.activate_connection("a", "c", ConnectionKind::Causal);
        graph.activate_connection("d", "a", ConnectionKind::Temporal); // Reverse

        let n = graph.neighbors("a");
        assert!(n.contains(&"b".to_string()));
        assert!(n.contains(&"c".to_string()));
        assert!(n.contains(&"d".to_string())); // Found via reverse lookup
    }

    #[test]
    fn test_causal_dag_rejects_cycle() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("a", "b", ConnectionKind::Causal);
        graph.activate_connection("b", "c", ConnectionKind::Causal);
        assert_eq!(graph.connection_count(), 2);
        // Adding c→a would create cycle a→b→c→a; must be rejected
        let w = graph.activate_connection("c", "a", ConnectionKind::Causal);
        assert_eq!(w, 0.1, "Should return default weight without adding");
        assert_eq!(
            graph.connection_count(),
            2,
            "Causal cycle edge must not be added"
        );
    }

    #[test]
    fn test_causal_dag_accepts_acyclic() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("a", "b", ConnectionKind::Causal);
        graph.activate_connection("a", "c", ConnectionKind::Causal);
        graph.activate_connection("b", "c", ConnectionKind::Causal); // No cycle: a→b→c, a→c
        assert_eq!(graph.connection_count(), 3);
    }

    #[test]
    fn test_causal_downstream() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("a", "b", ConnectionKind::Causal);
        graph.activate_connection("a", "c", ConnectionKind::Causal);
        graph.activate_connection("b", "c", ConnectionKind::Causal);
        graph.activate_connection("a", "x", ConnectionKind::Semantic); // not causal, ignored
        let down_a = graph.causal_downstream("a");
        assert_eq!(down_a, ["b", "c"], "From a: b and c (c via a→c and a→b→c)");
        assert_eq!(graph.causal_downstream("b"), ["c"]);
        assert!(graph.causal_downstream("c").is_empty());
        assert!(graph.causal_downstream("missing").is_empty());
    }

    #[test]
    fn test_concept_similarity() {
        let mut graph = ConnectionGraph::default();
        // a and b both connect to c and d
        graph.activate_connection("a", "c", ConnectionKind::Semantic);
        graph.activate_connection("a", "d", ConnectionKind::Semantic);
        graph.activate_connection("b", "c", ConnectionKind::Semantic);
        graph.activate_connection("b", "d", ConnectionKind::Semantic);
        // a also connects to e (not shared)
        graph.activate_connection("a", "e", ConnectionKind::Semantic);

        let sim = graph.concept_similarity("a", "b");
        assert!(
            sim > 0.5,
            "Similar concepts should have high similarity: {sim}"
        );

        let sim_unrelated = graph.concept_similarity("a", "z");
        assert!(sim_unrelated < 0.01, "Unrelated concepts: {sim_unrelated}");
    }

    #[test]
    fn test_ingest_scientific_learning() {
        let mut graph = ConnectionGraph::default();
        let learning = ScientificLearning {
            domain: "information_theory".to_string(),
            concept: "KL_divergence".to_string(),
            application: "Measures how one distribution diverges from another".to_string(),
            formula: Some("sum(P(x) * log(P(x)/Q(x)))".to_string()),
            tags: vec!["probability".to_string(), "optimization".to_string()],
        };
        let formed = graph.ingest_scientific_learning(&learning);
        assert_eq!(formed, 3); // 2 tags + 1 formula
        assert!(graph.node_count() >= 4); // concept + 2 tags + 1 formula
    }

    #[test]
    fn test_connection_graph_save_and_load() {
        let dir = std::env::temp_dir().join("yoyo_test_connections");
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("connections.jsonl");

        let mut graph = ConnectionGraph::default();
        graph.activate_connection("memory", "learning", ConnectionKind::Semantic);
        graph.activate_connection("memory", "persistence", ConnectionKind::Causal);
        for _ in 0..3 {
            graph.activate_connection("latent_space", "connections", ConnectionKind::Mathematical);
        }

        graph.save_to(&path).unwrap();

        let loaded = ConnectionGraph::load_from(&path);
        assert_eq!(loaded.connection_count(), 3);

        // Verify the strengthened connection loaded correctly
        let strong = loaded.strongest_connections("latent_space", 1);
        assert_eq!(strong.len(), 1);
        assert_eq!(strong[0].to, "connections");
        assert_eq!(strong[0].activations, 3);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_format_for_prompt() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("concept_a", "concept_b", ConnectionKind::Scientific);
        let prompt = graph.format_for_prompt();
        assert!(prompt.is_some());
        let text = prompt.unwrap();
        assert!(text.contains("Latent Space Connections"));
        assert!(text.contains("concept_a"));
        assert!(text.contains("concept_b"));
    }

    #[test]
    fn test_connection_kind_serialization() {
        let conn = Connection {
            from: "a".to_string(),
            to: "b".to_string(),
            weight: 0.5,
            activations: 3,
            last_activated: "2026-03-17 12:00".to_string(),
            kind: ConnectionKind::Mathematical,
            valid_when: None,
        };
        let json = serde_json::to_string(&conn).unwrap();
        assert!(json.contains("mathematical"));
        let parsed: Connection = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.kind, ConnectionKind::Mathematical);
    }

    #[test]
    fn test_effective_weight_temporal_decay() {
        let old = Connection {
            from: "a".to_string(),
            to: "b".to_string(),
            weight: 1.0,
            activations: 5,
            last_activated: "2026-03-01 12:00".to_string(),
            kind: ConnectionKind::Semantic,
            valid_when: None,
        };
        let recent = Connection {
            from: "c".to_string(),
            to: "d".to_string(),
            weight: 1.0,
            activations: 5,
            last_activated: "2026-03-18 12:00".to_string(),
            kind: ConnectionKind::Semantic,
            valid_when: None,
        };
        let now = "2026-03-18 18:00";
        let half_life = 7.0;
        let eff_old = effective_weight(&old, now, half_life);
        let eff_recent = effective_weight(&recent, now, half_life);
        assert!(
            eff_old < eff_recent,
            "older connection should have lower effective weight"
        );
        assert!(eff_recent <= 1.0 + 1e-6);
        assert!(eff_old > 0.0);
    }

    #[test]
    fn test_strongest_connections_weighted() {
        let mut graph = ConnectionGraph::default();
        // Insert two connections from "core" with same raw weight but different timestamps
        graph.edges.insert(
            "core".to_string(),
            vec![
                Connection {
                    from: "core".to_string(),
                    to: "old_concept".to_string(),
                    weight: 0.5,
                    activations: 3,
                    last_activated: "2026-02-01 12:00".to_string(),
                    kind: ConnectionKind::Semantic,
                    valid_when: None,
                },
                Connection {
                    from: "core".to_string(),
                    to: "recent_concept".to_string(),
                    weight: 0.5,
                    activations: 3,
                    last_activated: "2026-03-18 12:00".to_string(),
                    kind: ConnectionKind::Semantic,
                    valid_when: None,
                },
            ],
        );
        let now = "2026-03-18 18:00";
        let ranked = graph.strongest_connections_weighted("core", 2, now);
        assert_eq!(ranked.len(), 2);
        // Recent concept should rank first (higher effective weight)
        assert_eq!(ranked[0].0.to, "recent_concept");
        assert_eq!(ranked[1].0.to, "old_concept");
        // Effective weights should differ despite same raw weight
        assert!(ranked[0].1 > ranked[1].1);
    }

    #[test]
    fn test_connection_valid_when_backward_compat() {
        // Old JSONL line without valid_when must deserialize (valid_when defaults to None)
        let old_json = r#"{"from":"x","to":"y","weight":0.2,"activations":1,"last_activated":"2026-03-18 12:00","kind":"causal"}"#;
        let parsed: Connection = serde_json::from_str(old_json).unwrap();
        assert_eq!(parsed.valid_when, None);
        assert_eq!(parsed.kind, ConnectionKind::Causal);
    }

    #[test]
    fn test_connection_kind_display() {
        assert_eq!(ConnectionKind::Semantic.to_string(), "semantic");
        assert_eq!(ConnectionKind::Causal.to_string(), "causal");
        assert_eq!(ConnectionKind::Temporal.to_string(), "temporal");
        assert_eq!(ConnectionKind::Mathematical.to_string(), "mathematical");
        assert_eq!(ConnectionKind::Scientific.to_string(), "scientific");
    }

    #[test]
    fn test_parse_connection_kind() {
        assert_eq!(
            parse_connection_kind("semantic"),
            Some(ConnectionKind::Semantic)
        );
        assert_eq!(
            parse_connection_kind("Causal"),
            Some(ConnectionKind::Causal)
        );
        assert_eq!(
            parse_connection_kind("TEMPORAL"),
            Some(ConnectionKind::Temporal)
        );
        assert_eq!(
            parse_connection_kind("math"),
            Some(ConnectionKind::Mathematical)
        );
        assert_eq!(
            parse_connection_kind("mathematical"),
            Some(ConnectionKind::Mathematical)
        );
        assert_eq!(
            parse_connection_kind("scientific"),
            Some(ConnectionKind::Scientific)
        );
        assert_eq!(parse_connection_kind("unknown"), None);
        assert_eq!(parse_connection_kind(""), None);
    }

    #[test]
    fn test_neighbors_detailed() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("a", "b", ConnectionKind::Semantic);
        graph.activate_connection("a", "c", ConnectionKind::Causal);
        graph.activate_connection("d", "a", ConnectionKind::Temporal);

        let (out, inc) = graph.neighbors_detailed("a");
        assert_eq!(out.len(), 2); // a→b, a→c
        assert_eq!(inc.len(), 1); // d→a
        assert_eq!(inc[0].from, "d");

        // Concept with no connections
        let (out, inc) = graph.neighbors_detailed("missing");
        assert!(out.is_empty());
        assert!(inc.is_empty());
    }

    #[test]
    fn test_connections_by_kind() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("a", "b", ConnectionKind::Semantic);
        graph.activate_connection("a", "c", ConnectionKind::Semantic);
        graph.activate_connection("a", "d", ConnectionKind::Causal);

        let by_kind = graph.connections_by_kind();
        assert_eq!(by_kind.get("semantic"), Some(&2));
        assert_eq!(by_kind.get("causal"), Some(&1));
        assert_eq!(by_kind.get("temporal"), None);
    }

    #[test]
    fn test_full_crud_workflow() {
        let path = temp_memory_path("crud_workflow");

        // Start fresh
        let mut memory = load_memories_from(&path);
        assert!(memory.entries.is_empty());

        // Add entries (use force to bypass dedup for short strings)
        add_memory_force(&mut memory, "first");
        add_memory_force(&mut memory, "second");
        add_memory_force(&mut memory, "third");
        assert_eq!(memory.entries.len(), 3);

        // Save
        save_memories_to(&memory, &path).unwrap();

        // Reload
        let mut loaded = load_memories_from(&path);
        assert_eq!(loaded.entries.len(), 3);
        assert_eq!(loaded.entries[0].note, "first");

        // Remove middle entry
        let removed = remove_memory(&mut loaded, 1);
        assert_eq!(removed.unwrap().note, "second");
        assert_eq!(loaded.entries.len(), 2);

        // Save and reload again
        save_memories_to(&loaded, &path).unwrap();
        let final_load = load_memories_from(&path);
        assert_eq!(final_load.entries.len(), 2);
        assert_eq!(final_load.entries[0].note, "first");
        assert_eq!(final_load.entries[1].note, "third");

        cleanup(&path);
    }

    #[test]
    fn test_shortest_path_direct() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("a", "b", ConnectionKind::Semantic);
        let path = graph.shortest_path("a", "b");
        assert_eq!(path, Some(vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn test_shortest_path_multi_hop() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("a", "b", ConnectionKind::Semantic);
        graph.activate_connection("b", "c", ConnectionKind::Causal);
        graph.activate_connection("c", "d", ConnectionKind::Temporal);
        let path = graph.shortest_path("a", "d");
        assert_eq!(
            path,
            Some(vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string()
            ])
        );
    }

    #[test]
    fn test_shortest_path_no_path() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("a", "b", ConnectionKind::Semantic);
        graph.activate_connection("c", "d", ConnectionKind::Semantic);
        let path = graph.shortest_path("a", "d");
        assert_eq!(path, None);
    }

    #[test]
    fn test_shortest_path_self() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("a", "b", ConnectionKind::Semantic);
        let path = graph.shortest_path("a", "a");
        assert_eq!(path, Some(vec!["a".to_string()]));
    }

    #[test]
    fn test_shortest_path_reverse_direction() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("a", "b", ConnectionKind::Causal);
        let path = graph.shortest_path("b", "a");
        assert_eq!(path, Some(vec!["b".to_string(), "a".to_string()]));
    }

    #[test]
    fn test_shortest_path_picks_shortest() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("a", "d", ConnectionKind::Semantic);
        graph.activate_connection("a", "b", ConnectionKind::Semantic);
        graph.activate_connection("b", "c", ConnectionKind::Semantic);
        graph.activate_connection("c", "d", ConnectionKind::Semantic);
        let path = graph.shortest_path("a", "d").unwrap();
        assert_eq!(path.len(), 2, "BFS should find the 1-hop path");
    }

    #[test]
    fn test_shortest_path_empty_graph() {
        let graph = ConnectionGraph::default();
        let path = graph.shortest_path("a", "b");
        assert_eq!(path, None);
    }

    // ── Mutual information tests ─────────────────────────────────────────

    #[test]
    fn test_mutual_information_shared_neighbors() {
        let mut graph = ConnectionGraph::default();
        // a→c, b→c, a→d, b→d — a and b share neighbors c and d
        graph.activate_connection("a", "c", ConnectionKind::Semantic);
        graph.activate_connection("a", "d", ConnectionKind::Semantic);
        graph.activate_connection("b", "c", ConnectionKind::Semantic);
        graph.activate_connection("b", "d", ConnectionKind::Semantic);
        let mi = graph.mutual_information("a", "b");
        assert!(mi > 0.0, "Shared neighbors should produce positive MI");
    }

    #[test]
    fn test_mutual_information_no_shared_neighbors() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("a", "c", ConnectionKind::Semantic);
        graph.activate_connection("b", "d", ConnectionKind::Semantic);
        let mi = graph.mutual_information("a", "b");
        assert_eq!(mi, 0.0, "No shared neighbors → MI = 0");
    }

    #[test]
    fn test_mutual_information_empty_graph() {
        let graph = ConnectionGraph::default();
        let mi = graph.mutual_information("a", "b");
        assert_eq!(mi, 0.0);
    }

    #[test]
    fn test_mutual_information_unknown_concept() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("a", "b", ConnectionKind::Semantic);
        let mi = graph.mutual_information("a", "nonexistent");
        assert_eq!(mi, 0.0);
    }

    // ── Concept extraction tests ────────────────────────────────────────

    #[test]
    fn test_extract_concepts_basic() {
        let concepts =
            extract_concepts("Cleanup creates perception — you can't polish what you can't see");
        assert!(concepts.contains(&"cleanup".to_string()));
        assert!(concepts.contains(&"creates".to_string()));
        assert!(concepts.contains(&"perception".to_string()));
        assert!(concepts.contains(&"polish".to_string()));
        assert!(!concepts.contains(&"you".to_string()));
    }

    #[test]
    fn test_extract_concepts_filters_stop_words() {
        let concepts = extract_concepts("the quick brown fox jumps over the lazy dog");
        assert!(!concepts.contains(&"the".to_string()));
        assert!(!concepts.contains(&"over".to_string()));
        assert!(concepts.contains(&"quick".to_string()));
        assert!(concepts.contains(&"brown".to_string()));
        assert!(concepts.contains(&"fox".to_string()));
    }

    #[test]
    fn test_extract_concepts_filters_short_and_numbers() {
        let concepts = extract_concepts("Day 10 was great — 3 sessions of work in 42 minutes");
        assert!(!concepts.contains(&"10".to_string()));
        assert!(!concepts.contains(&"42".to_string()));
        assert!(!concepts.contains(&"was".to_string()));
    }

    #[test]
    fn test_extract_concepts_deduplicates() {
        let concepts = extract_concepts("avoidance pattern avoidance ritual avoidance");
        let avoidance_count = concepts.iter().filter(|c| *c == "avoidance").count();
        assert_eq!(avoidance_count, 1);
    }

    #[test]
    fn test_extract_concepts_empty_input() {
        let concepts = extract_concepts("");
        assert!(concepts.is_empty());
    }

    #[test]
    fn test_extract_concepts_detects_bigrams() {
        let concepts = extract_concepts("the connection graph has error handling");
        assert!(concepts.contains(&"connection_graph".to_string()));
        assert!(concepts.contains(&"error_handling".to_string()));
        // The individual words should NOT appear if consumed by a bigram
        assert!(!concepts.contains(&"connection".to_string()));
        assert!(!concepts.contains(&"graph".to_string()));
    }

    #[test]
    fn test_extract_concepts_weighted_title_first() {
        let concepts =
            extract_concepts_weighted("self awareness", "error handling is important for recovery");
        // Title concepts should appear first
        assert_eq!(concepts[0], "self_awareness");
        // Body concepts after
        assert!(concepts.contains(&"error_handling".to_string()));
        assert!(concepts.contains(&"recovery".to_string()));
    }

    #[test]
    fn test_extract_concepts_weighted_no_duplicates() {
        let concepts = extract_concepts_weighted("error handling", "error handling is key");
        let eh_count = concepts.iter().filter(|c| *c == "error_handling").count();
        assert_eq!(eh_count, 1);
    }

    // ── Populate from learnings tests ───────────────────────────────────

    #[test]
    fn test_populate_from_learnings_creates_connections() {
        let learnings = vec![LearningEntry {
            title: "Avoidance pattern creates guilt".to_string(),
            takeaway: "Self-awareness alone does not change behavior".to_string(),
            context: String::new(),
            source: "test".to_string(),
            day: Some(1),
            ts: "2026-03-10".to_string(),
            entry_type: "lesson".to_string(),
        }];

        let mut graph = ConnectionGraph::default();
        let (processed, connections) = graph.populate_from_learnings(&learnings);

        assert_eq!(processed, 1);
        assert!(connections > 0);
        assert!(graph.node_count() > 0);
        assert!(graph.connection_count() > 0);
    }

    #[test]
    fn test_populate_strengthens_repeated_concepts() {
        let learnings = vec![
            LearningEntry {
                title: "Avoidance creates guilt".to_string(),
                takeaway: "Avoidance blocks progress".to_string(),
                context: String::new(),
                source: "test".to_string(),
                day: Some(1),
                ts: "2026-03-10".to_string(),
                entry_type: "lesson".to_string(),
            },
            LearningEntry {
                title: "Guilt from avoidance".to_string(),
                takeaway: "Guilt is a signal not a punishment".to_string(),
                context: String::new(),
                source: "test".to_string(),
                day: Some(2),
                ts: "2026-03-11".to_string(),
                entry_type: "lesson".to_string(),
            },
        ];

        let mut graph = ConnectionGraph::default();
        graph.populate_from_learnings(&learnings);

        // "avoidance" and "guilt" co-occur in both learnings — alphabetically
        // normalized to avoidance → guilt, so the edge should be activated 2+ times
        let edges = graph.edges.get("avoidance");
        assert!(edges.is_some(), "Should have edges from 'avoidance'");
        let guilt_conn = edges.unwrap().iter().find(|c| c.to == "guilt");
        assert!(guilt_conn.is_some(), "Should have avoidance → guilt edge");
        assert!(
            guilt_conn.unwrap().activations >= 2,
            "Should be activated at least twice"
        );
        assert!(
            guilt_conn.unwrap().weight > 0.1,
            "Weight should grow with repeated activation"
        );
    }

    #[test]
    fn test_populate_empty_learnings() {
        let mut graph = ConnectionGraph::default();
        let (processed, connections) = graph.populate_from_learnings(&[]);
        assert_eq!(processed, 0);
        assert_eq!(connections, 0);
    }

    // ── Graph stats tests ───────────────────────────────────────────────

    #[test]
    fn test_compute_stats_empty_graph() {
        let graph = ConnectionGraph::default();
        let stats = graph.compute_stats();
        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
        assert_eq!(stats.avg_weight, 0.0);
        assert!(stats.strongest.is_none());
        assert!(stats.most_connected.is_none());
    }

    #[test]
    fn test_compute_stats_populated_graph() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("a", "b", ConnectionKind::Semantic);
        graph.activate_connection("a", "c", ConnectionKind::Causal);
        graph.activate_connection("b", "c", ConnectionKind::Semantic);

        let stats = graph.compute_stats();
        assert_eq!(stats.node_count, 3);
        assert_eq!(stats.edge_count, 3);
        assert!(stats.avg_weight > 0.0);
        assert!(stats.strongest.is_some());
        assert!(stats.most_connected.is_some());
        assert_eq!(*stats.by_kind.get("semantic").unwrap_or(&0), 2);
        assert_eq!(*stats.by_kind.get("causal").unwrap_or(&0), 1);
    }

    // ── Load learnings test ─────────────────────────────────────────────

    #[test]
    fn test_load_learnings_from_file() {
        let dir = std::env::temp_dir().join("yoyo_test_learnings");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("test_learnings.jsonl");
        let content = r#"{"type":"lesson","day":1,"ts":"2026-03-10","source":"test","title":"Test learning","context":"ctx","takeaway":"take"}
{"type":"lesson","day":2,"ts":"2026-03-11","source":"test","title":"Another learning","context":"ctx2","takeaway":"take2"}"#;
        fs::write(&path, content).unwrap();

        let learnings = load_learnings_from(&path);
        assert_eq!(learnings.len(), 2);
        assert_eq!(learnings[0].title, "Test learning");
        assert_eq!(learnings[1].title, "Another learning");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_learnings_missing_file() {
        let learnings = load_learnings_from(Path::new("/nonexistent/path.jsonl"));
        assert!(learnings.is_empty());
    }

    // ── Community detection tests ───────────────────────────────────────

    #[test]
    fn test_detect_communities_empty_graph() {
        let graph = ConnectionGraph::default();
        let communities = graph.detect_communities(10);
        assert!(communities.is_empty());
    }

    #[test]
    fn test_detect_communities_two_clusters() {
        let mut graph = ConnectionGraph::default();
        // Cluster 1: a-b-c (strongly connected)
        graph.activate_connection("a", "b", ConnectionKind::Semantic);
        graph.activate_connection("b", "c", ConnectionKind::Semantic);
        graph.activate_connection("a", "c", ConnectionKind::Semantic);
        // Cluster 2: x-y-z (strongly connected)
        graph.activate_connection("x", "y", ConnectionKind::Semantic);
        graph.activate_connection("y", "z", ConnectionKind::Semantic);
        graph.activate_connection("x", "z", ConnectionKind::Semantic);

        let communities = graph.detect_communities(20);
        // Should find at least 2 communities (disconnected components)
        assert!(communities.len() >= 2);
        // Each community should have 3 members
        let sizes: Vec<usize> = communities.iter().map(|(_, m)| m.len()).collect();
        assert!(sizes.contains(&3));
    }

    #[test]
    fn test_detect_communities_single_cluster() {
        let mut graph = ConnectionGraph::default();
        graph.activate_connection("a", "b", ConnectionKind::Semantic);
        graph.activate_connection("b", "c", ConnectionKind::Semantic);

        let communities = graph.detect_communities(20);
        // All nodes should converge to one community
        let non_singleton: Vec<_> = communities.iter().filter(|(_, m)| m.len() > 1).collect();
        assert!(!non_singleton.is_empty());
        // Total nodes across all communities = 3
        let total: usize = communities.iter().map(|(_, m)| m.len()).sum();
        assert_eq!(total, 3);
    }

    #[test]
    fn test_detect_communities_respects_weights() {
        let mut graph = ConnectionGraph::default();
        // Strong cluster: a-b (activate multiple times for higher weight)
        graph.activate_connection("a", "b", ConnectionKind::Semantic);
        graph.activate_connection("a", "b", ConnectionKind::Semantic);
        graph.activate_connection("a", "b", ConnectionKind::Semantic);
        // Weak bridge: b-c
        graph.activate_connection("b", "c", ConnectionKind::Semantic);

        let communities = graph.detect_communities(20);
        // Should have communities, total 3 nodes
        let total: usize = communities.iter().map(|(_, m)| m.len()).sum();
        assert_eq!(total, 3);
    }
}
