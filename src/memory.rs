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
}
