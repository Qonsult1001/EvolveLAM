//! SCA Context Lens — Active brain context injection.
//!
//! Instead of dumping the entire connection graph into the system prompt at startup,
//! the ContextLens queries the brain dynamically per-prompt, injecting only the
//! relevant patterns and connections for the current task.
//!
//! This is the equivalent of the SCA Context Lens in the .saidSo/RAX architecture:
//! "zero-latency signal injection" from the brain back into the active context.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::memory::{self, ConnectionGraph};

/// The context lens — queries the brain and returns relevant context for injection.
pub struct ContextLens {
    graph: ConnectionGraph,
    skills_dir: Option<PathBuf>,
}

/// Result of a context lens query.
pub struct InjectedContext {
    /// Formatted text block to inject into the prompt.
    pub text: String,
    /// Relevance score: 0.0 = nothing matched, 1.0 = perfect match.
    pub relevance_score: f64,
}

impl ContextLens {
    /// Create a new context lens with the given graph and skills directory.
    pub fn new(graph: ConnectionGraph, skills_dir: Option<PathBuf>) -> Self {
        Self { graph, skills_dir }
    }

    /// Query the brain for context relevant to the given input.
    /// Returns an InjectedContext with the formatted text and a relevance score.
    pub fn query(&self, input: &str) -> InjectedContext {
        let concepts = memory::extract_concepts(input);
        if concepts.is_empty() {
            return InjectedContext {
                text: String::new(),
                relevance_score: 0.0,
            };
        }

        // Query the graph for relevant connections
        let now_ts = chrono_now();
        let graph_lines = self.relevant_subgraph(&concepts, &now_ts, 15);

        // Query skill files for matching patterns
        let skill_patterns = self.match_skill_patterns(&concepts);

        // Calculate relevance: what fraction of extracted concepts had matches?
        let matched_concepts =
            self.count_matched_concepts(&concepts, &graph_lines, &skill_patterns);
        let relevance_score = if concepts.is_empty() {
            0.0
        } else {
            matched_concepts as f64 / concepts.len() as f64
        };

        let text = self.format_injection(&graph_lines, &skill_patterns);

        InjectedContext {
            text,
            relevance_score,
        }
    }

    /// Walk the graph from query concepts, collecting neighbors weighted by recency.
    fn relevant_subgraph(
        &self,
        concepts: &[String],
        now_ts: &str,
        max_nodes: usize,
    ) -> Vec<(String, String, f64, String)> {
        // (from, to, effective_weight, kind)
        let mut seen: HashSet<(String, String)> = HashSet::new();
        let mut results: Vec<(String, String, f64, String)> = Vec::new();

        for concept in concepts {
            let conns = self
                .graph
                .strongest_connections_weighted(concept, 5, now_ts);
            for (conn, eff_weight) in conns {
                let key = (conn.from.clone(), conn.to.clone());
                if seen.insert(key) {
                    results.push((
                        conn.from.clone(),
                        conn.to.clone(),
                        eff_weight,
                        conn.kind.to_string(),
                    ));
                }
            }

            // Also check reverse edges (concept as target)
            for edges in self.graph.edges.values() {
                for conn in edges {
                    if conn.to.to_lowercase() == concept.to_lowercase() {
                        let key = (conn.from.clone(), conn.to.clone());
                        if seen.insert(key) {
                            let eff_weight = conn.weight; // approximate for reverse
                            results.push((
                                conn.from.clone(),
                                conn.to.clone(),
                                eff_weight,
                                conn.kind.to_string(),
                            ));
                        }
                    }
                }
            }
        }

        // Sort by effective weight descending, cap at max_nodes
        results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(max_nodes);
        results
    }

    /// Scan skill files for patterns matching query concepts.
    /// Returns (domain_name, matching_pattern_line) pairs.
    fn match_skill_patterns(&self, concepts: &[String]) -> Vec<(String, String)> {
        let skills_dir = match &self.skills_dir {
            Some(d) => d.clone(),
            None => return Vec::new(),
        };

        let domains = [
            "code-rust",
            "code-web",
            "code-systems",
            "code-data",
            "code-devops",
            "code-testing",
        ];

        let mut matches = Vec::new();

        for domain in &domains {
            let path = skills_dir.join(domain).join("SKILL.md");
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Find the "Patterns Learned" section and extract pattern lines
            let mut in_patterns = false;
            for line in content.lines() {
                if line.contains("Patterns Learned") {
                    in_patterns = true;
                    continue;
                }
                if in_patterns && line.starts_with("## ") {
                    break;
                }
                if in_patterns && line.starts_with("- ") {
                    let pattern_text = &line[2..];
                    let pattern_lower = pattern_text.to_lowercase();
                    // Check if any concept appears in this pattern
                    for concept in concepts {
                        if pattern_lower.contains(&concept.to_lowercase()) {
                            matches.push((domain.to_string(), pattern_text.to_string()));
                            break;
                        }
                    }
                }
            }
        }

        matches
    }

    /// Count how many of the original concepts had at least one match.
    fn count_matched_concepts(
        &self,
        concepts: &[String],
        graph_lines: &[(String, String, f64, String)],
        skill_patterns: &[(String, String)],
    ) -> usize {
        let mut matched = 0;
        for concept in concepts {
            let c_lower = concept.to_lowercase();
            let in_graph = graph_lines.iter().any(|(from, to, _, _)| {
                from.to_lowercase().contains(&c_lower) || to.to_lowercase().contains(&c_lower)
            });
            let in_skills = skill_patterns
                .iter()
                .any(|(_, pattern)| pattern.to_lowercase().contains(&c_lower));
            if in_graph || in_skills {
                matched += 1;
            }
        }
        matched
    }

    /// Format the injection block as markdown.
    fn format_injection(
        &self,
        graph_lines: &[(String, String, f64, String)],
        skill_patterns: &[(String, String)],
    ) -> String {
        if graph_lines.is_empty() && skill_patterns.is_empty() {
            return String::new();
        }

        let mut lines = Vec::new();
        lines.push("[Brain context — relevant patterns from knowledge graph]".to_string());

        if !graph_lines.is_empty() {
            lines.push(String::new());
            lines.push("Connections:".to_string());
            for (from, to, weight, kind) in graph_lines.iter().take(10) {
                lines.push(format!("  {from} → {to} ({kind}, w:{weight:.2})"));
            }
        }

        if !skill_patterns.is_empty() {
            lines.push(String::new());
            lines.push("Learned patterns:".to_string());
            // Group by domain
            let mut by_domain: HashMap<&str, Vec<&str>> = HashMap::new();
            for (domain, pattern) in skill_patterns {
                by_domain
                    .entry(domain.as_str())
                    .or_default()
                    .push(pattern.as_str());
            }
            for (domain, patterns) in &by_domain {
                lines.push(format!("  [{domain}]"));
                for pattern in patterns.iter().take(5) {
                    lines.push(format!("  - {pattern}"));
                }
            }
        }

        lines.join("\n")
    }
}

/// Get a rough ISO8601 timestamp for "now" without pulling in the chrono crate.
fn chrono_now() -> String {
    // Use the same approach as the rest of the codebase: shell out or use system time
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Convert to rough ISO8601 (good enough for weight decay calculations)
    let days_since_epoch = now / 86400;
    let years = 1970 + days_since_epoch / 365;
    let remaining_days = days_since_epoch % 365;
    let month = remaining_days / 30 + 1;
    let day = remaining_days % 30 + 1;
    format!("{years:04}-{month:02}-{day:02}T00:00:00Z")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::ConnectionGraph;

    #[test]
    fn test_empty_input_returns_zero_relevance() {
        let lens = ContextLens::new(ConnectionGraph::default(), None);
        let result = lens.query("");
        assert_eq!(result.relevance_score, 0.0);
        assert!(result.text.is_empty());
    }

    #[test]
    fn test_no_graph_no_skills_returns_zero_relevance() {
        let lens = ContextLens::new(ConnectionGraph::default(), None);
        let result = lens.query("rust error handling async");
        assert_eq!(result.relevance_score, 0.0);
    }

    #[test]
    fn test_query_with_loaded_graph() {
        // Create a graph with some connections
        let mut graph = ConnectionGraph::default();
        let conn = crate::memory::Connection {
            from: "error_handling".to_string(),
            to: "rust_patterns".to_string(),
            weight: 0.5,
            activations: 3,
            last_activated: "2026-03-21T00:00:00Z".to_string(),
            kind: crate::memory::ConnectionKind::Semantic,
            valid_when: None,
        };
        graph
            .edges
            .entry("error_handling".to_string())
            .or_default()
            .push(conn);

        let lens = ContextLens::new(graph, None);
        let result = lens.query("error handling in rust");
        // Should find the connection via "error_handling" concept
        assert!(result.relevance_score > 0.0);
        assert!(result.text.contains("error_handling"));
    }

    #[test]
    fn test_skill_pattern_matching() {
        let dir = tempfile::TempDir::new().unwrap();
        let rust_dir = dir.path().join("code-rust");
        std::fs::create_dir_all(&rust_dir).unwrap();
        std::fs::write(
            rust_dir.join("SKILL.md"),
            "# Rust\n\n## Patterns Learned\n\n- use Cow<str> for conditional ownership in rust\n- prefer &str over String in function parameters\n",
        )
        .unwrap();

        let lens = ContextLens::new(ConnectionGraph::default(), Some(dir.path().to_path_buf()));
        let result = lens.query("rust ownership and borrowing with Cow");
        // Should match "use Cow<str>..." pattern via "rust" or "cow" concept
        assert!(!result.text.is_empty());
    }

    #[test]
    fn test_format_injection_empty() {
        let lens = ContextLens::new(ConnectionGraph::default(), None);
        let result = lens.format_injection(&[], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_format_injection_with_connections() {
        let lens = ContextLens::new(ConnectionGraph::default(), None);
        let graph_lines = vec![(
            "error".to_string(),
            "handling".to_string(),
            0.5,
            "semantic".to_string(),
        )];
        let result = lens.format_injection(&graph_lines, &[]);
        assert!(result.contains("[Brain context"));
        assert!(result.contains("error → handling"));
    }

    #[test]
    fn test_format_injection_with_patterns() {
        let lens = ContextLens::new(ConnectionGraph::default(), None);
        let patterns = vec![(
            "code-rust".to_string(),
            "use Cow<str> for conditional ownership".to_string(),
        )];
        let result = lens.format_injection(&[], &patterns);
        assert!(result.contains("[Brain context"));
        assert!(result.contains("code-rust"));
        assert!(result.contains("Cow<str>"));
    }

    #[test]
    fn test_chrono_now_format() {
        let ts = chrono_now();
        assert!(ts.contains('T'));
        assert!(ts.ends_with('Z'));
        assert!(ts.len() == 20);
    }

    #[test]
    fn test_max_nodes_caps_results() {
        let mut graph = ConnectionGraph::default();
        // Add 20 connections from the same concept
        for i in 0..20 {
            let conn = crate::memory::Connection {
                from: "test_concept".to_string(),
                to: format!("target_{i}"),
                weight: 0.1 * (i as f64),
                activations: 1,
                last_activated: "2026-03-21T00:00:00Z".to_string(),
                kind: crate::memory::ConnectionKind::Semantic,
                valid_when: None,
            };
            graph
                .edges
                .entry("test_concept".to_string())
                .or_default()
                .push(conn);
        }

        let lens = ContextLens::new(graph, None);
        let results =
            lens.relevant_subgraph(&["test_concept".to_string()], "2026-03-21T00:00:00Z", 5);
        assert!(results.len() <= 5);
    }
}
