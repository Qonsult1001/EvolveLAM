//! Unified `.said` Persistence Layer
//!
//! Single append-only JSONL file replacing multiple separate files
//! (learnings.jsonl, social_learnings.jsonl, connections.jsonl, etc.)
//!
//! This is the L5 `.said Persistence` layer from the .saidSo / RAX Engine architecture.
//! All records are tagged with a `record_type` discriminator for efficient querying.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use crate::memory::{Connection, ConnectionGraph, ConnectionKind};

/// A unified record in the .said persistence layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "record_type")]
pub enum SaidRecord {
    /// A connection in the knowledge graph
    #[serde(rename = "connection")]
    Connection {
        from: String,
        to: String,
        weight: f64,
        activations: u64,
        last_activated: String,
        kind: String,
    },
    /// A self-reflection learning
    #[serde(rename = "learning")]
    Learning {
        ts: String,
        day: u32,
        source: String,
        title: String,
        context: String,
        takeaway: String,
    },
    /// A social insight
    #[serde(rename = "social")]
    Social {
        ts: String,
        day: u32,
        source: String,
        who: String,
        insight: String,
    },
    /// A memory note
    #[serde(rename = "memory")]
    Memory { ts: String, note: String },
    /// A tool execution event
    #[serde(rename = "tool_event")]
    ToolEvent {
        ts: String,
        tool: String,
        path: String,
    },
    /// A runtime error
    #[serde(rename = "runtime_error")]
    RuntimeError {
        ts: String,
        category: String,
        message: String,
    },
    /// A session summary
    #[serde(rename = "session")]
    Session { ts: String, day: u32, tasks: u32 },
}

/// The .said persistence store — append-only JSONL.
pub struct SaidStore {
    path: PathBuf,
}

impl SaidStore {
    /// Open (or create) a .said store at the given path.
    pub fn open(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }

    /// Append a record to the store.
    pub fn append(&mut self, record: &SaidRecord) -> Result<(), String> {
        // Hostile set enforcement for connections
        if let SaidRecord::Connection { kind, from, to, .. } = record {
            // Causal connections must not be self-referential
            if kind == "causal" && from == to {
                return Err(
                    "Causal connections cannot be self-referential (DAG constraint)".into(),
                );
            }
        }

        let line =
            serde_json::to_string(record).map_err(|e| format!("Serialization error: {e}"))?;

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("Failed to open .said file: {e}"))?;

        writeln!(file, "{line}").map_err(|e| format!("Failed to write to .said file: {e}"))?;

        Ok(())
    }

    /// Read all records from the store.
    pub fn read_all(&self) -> Vec<SaidRecord> {
        let file = match std::fs::File::open(&self.path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        let reader = std::io::BufReader::new(file);
        let mut records = Vec::new();

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<SaidRecord>(&line) {
                Ok(record) => records.push(record),
                Err(_) => continue, // Skip malformed lines
            }
        }

        records
    }

    /// Extract a ConnectionGraph from all connection records.
    pub fn query_connections(&self) -> ConnectionGraph {
        let records = self.read_all();
        let mut graph = ConnectionGraph::default();

        for record in records {
            if let SaidRecord::Connection {
                from,
                to,
                weight,
                activations,
                last_activated,
                kind,
            } = record
            {
                let conn_kind = match kind.as_str() {
                    "causal" => ConnectionKind::Causal,
                    "temporal" => ConnectionKind::Temporal,
                    "mathematical" => ConnectionKind::Mathematical,
                    "scientific" => ConnectionKind::Scientific,
                    _ => ConnectionKind::Semantic,
                };
                let conn = Connection {
                    from: from.clone(),
                    to,
                    weight,
                    activations,
                    last_activated,
                    kind: conn_kind,
                    valid_when: None,
                };
                graph.edges.entry(from).or_default().push(conn);
            }
        }

        graph
    }

    /// Extract all learning records.
    pub fn query_learnings(&self) -> Vec<SaidRecord> {
        self.read_all()
            .into_iter()
            .filter(|r| matches!(r, SaidRecord::Learning { .. }))
            .collect()
    }

    /// Extract all memory records.
    pub fn query_memories(&self) -> Vec<SaidRecord> {
        self.read_all()
            .into_iter()
            .filter(|r| matches!(r, SaidRecord::Memory { .. }))
            .collect()
    }

    /// Migrate from legacy separate JSONL files into a unified .said file.
    pub fn migrate_from_legacy(said_path: &Path, memory_dir: &Path) -> Result<Self, String> {
        let mut store = Self::open(said_path);

        // Migrate connections.jsonl
        let conn_path = memory_dir.join("connections.jsonl");
        if conn_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&conn_path) {
                for line in content.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                        let record = SaidRecord::Connection {
                            from: val["from"].as_str().unwrap_or("").to_string(),
                            to: val["to"].as_str().unwrap_or("").to_string(),
                            weight: val["weight"].as_f64().unwrap_or(0.1),
                            activations: val["activations"].as_u64().unwrap_or(1),
                            last_activated: val["last_activated"]
                                .as_str()
                                .unwrap_or("")
                                .to_string(),
                            kind: val["kind"].as_str().unwrap_or("semantic").to_string(),
                        };
                        store.append(&record)?;
                    }
                }
            }
        }

        // Migrate learnings.jsonl
        let learn_path = memory_dir.join("learnings.jsonl");
        if learn_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&learn_path) {
                for line in content.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                        let record = SaidRecord::Learning {
                            ts: val["ts"].as_str().unwrap_or("").to_string(),
                            day: val["day"].as_u64().unwrap_or(0) as u32,
                            source: val["source"].as_str().unwrap_or("").to_string(),
                            title: val["title"].as_str().unwrap_or("").to_string(),
                            context: val["context"].as_str().unwrap_or("").to_string(),
                            takeaway: val["takeaway"].as_str().unwrap_or("").to_string(),
                        };
                        store.append(&record)?;
                    }
                }
            }
        }

        // Migrate social_learnings.jsonl
        let social_path = memory_dir.join("social_learnings.jsonl");
        if social_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&social_path) {
                for line in content.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                        let record = SaidRecord::Social {
                            ts: val["ts"].as_str().unwrap_or("").to_string(),
                            day: val["day"].as_u64().unwrap_or(0) as u32,
                            source: val["source"].as_str().unwrap_or("").to_string(),
                            who: val["who"].as_str().unwrap_or("").to_string(),
                            insight: val["insight"].as_str().unwrap_or("").to_string(),
                        };
                        store.append(&record)?;
                    }
                }
            }
        }

        Ok(store)
    }

    /// Check if the .said file exists.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Count records by type.
    pub fn count_by_type(&self) -> std::collections::HashMap<String, usize> {
        let mut counts = std::collections::HashMap::new();
        for record in self.read_all() {
            let key = match record {
                SaidRecord::Connection { .. } => "connection",
                SaidRecord::Learning { .. } => "learning",
                SaidRecord::Social { .. } => "social",
                SaidRecord::Memory { .. } => "memory",
                SaidRecord::ToolEvent { .. } => "tool_event",
                SaidRecord::RuntimeError { .. } => "runtime_error",
                SaidRecord::Session { .. } => "session",
            };
            *counts.entry(key.to_string()).or_insert(0) += 1;
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_and_read() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.said");
        let mut store = SaidStore::open(&path);

        let record = SaidRecord::Memory {
            ts: "2026-03-21T00:00:00Z".to_string(),
            note: "test memory".to_string(),
        };
        store.append(&record).unwrap();

        let records = store.read_all();
        assert_eq!(records.len(), 1);
        match &records[0] {
            SaidRecord::Memory { note, .. } => assert_eq!(note, "test memory"),
            _ => panic!("Expected Memory record"),
        }
    }

    #[test]
    fn test_append_connection() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.said");
        let mut store = SaidStore::open(&path);

        let record = SaidRecord::Connection {
            from: "rust".to_string(),
            to: "error_handling".to_string(),
            weight: 0.5,
            activations: 3,
            last_activated: "2026-03-21T00:00:00Z".to_string(),
            kind: "semantic".to_string(),
        };
        store.append(&record).unwrap();

        let graph = store.query_connections();
        assert!(graph.edges.contains_key("rust"));
    }

    #[test]
    fn test_causal_self_ref_rejected() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.said");
        let mut store = SaidStore::open(&path);

        let record = SaidRecord::Connection {
            from: "concept_a".to_string(),
            to: "concept_a".to_string(),
            weight: 0.5,
            activations: 1,
            last_activated: "2026-03-21T00:00:00Z".to_string(),
            kind: "causal".to_string(),
        };
        assert!(store.append(&record).is_err());
    }

    #[test]
    fn test_semantic_self_ref_allowed() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.said");
        let mut store = SaidStore::open(&path);

        let record = SaidRecord::Connection {
            from: "concept_a".to_string(),
            to: "concept_a".to_string(),
            weight: 0.5,
            activations: 1,
            last_activated: "2026-03-21T00:00:00Z".to_string(),
            kind: "semantic".to_string(),
        };
        assert!(store.append(&record).is_ok());
    }

    #[test]
    fn test_query_learnings() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.said");
        let mut store = SaidStore::open(&path);

        store
            .append(&SaidRecord::Learning {
                ts: "2026-03-21T00:00:00Z".to_string(),
                day: 21,
                source: "test".to_string(),
                title: "test learning".to_string(),
                context: "testing".to_string(),
                takeaway: "it works".to_string(),
            })
            .unwrap();
        store
            .append(&SaidRecord::Memory {
                ts: "2026-03-21T00:00:00Z".to_string(),
                note: "a memory".to_string(),
            })
            .unwrap();

        assert_eq!(store.query_learnings().len(), 1);
        assert_eq!(store.query_memories().len(), 1);
    }

    #[test]
    fn test_read_empty_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.said");
        let store = SaidStore::open(&path);
        assert!(store.read_all().is_empty());
    }

    #[test]
    fn test_count_by_type() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.said");
        let mut store = SaidStore::open(&path);

        store
            .append(&SaidRecord::Memory {
                ts: "t1".into(),
                note: "n1".into(),
            })
            .unwrap();
        store
            .append(&SaidRecord::Memory {
                ts: "t2".into(),
                note: "n2".into(),
            })
            .unwrap();
        store
            .append(&SaidRecord::Session {
                ts: "t3".into(),
                day: 1,
                tasks: 5,
            })
            .unwrap();

        let counts = store.count_by_type();
        assert_eq!(counts.get("memory"), Some(&2));
        assert_eq!(counts.get("session"), Some(&1));
    }

    #[test]
    fn test_migrate_from_legacy() {
        let dir = tempfile::TempDir::new().unwrap();
        let memory_dir = dir.path().join("memory");
        std::fs::create_dir_all(&memory_dir).unwrap();

        // Write a legacy connections file
        std::fs::write(
            memory_dir.join("connections.jsonl"),
            r#"{"from":"rust","to":"systems","weight":0.5,"activations":3,"last_activated":"2026-03-21T00:00:00Z","kind":"semantic"}"#,
        )
        .unwrap();

        // Write a legacy learnings file
        std::fs::write(
            memory_dir.join("learnings.jsonl"),
            r#"{"type":"lesson","day":1,"ts":"2026-01-01T00:00:00Z","source":"self","title":"Test","context":"ctx","takeaway":"learned"}"#,
        )
        .unwrap();

        let said_path = dir.path().join("test.said");
        let store = SaidStore::migrate_from_legacy(&said_path, &memory_dir).unwrap();

        let records = store.read_all();
        assert_eq!(records.len(), 2);

        let graph = store.query_connections();
        assert!(graph.edges.contains_key("rust"));

        let learnings = store.query_learnings();
        assert_eq!(learnings.len(), 1);
    }

    #[test]
    fn test_malformed_lines_skipped() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.said");

        // Write some valid and invalid lines
        std::fs::write(
            &path,
            r#"{"record_type":"memory","ts":"t1","note":"valid"}
not json at all
{"record_type":"memory","ts":"t2","note":"also valid"}
"#,
        )
        .unwrap();

        let store = SaidStore::open(&path);
        let records = store.read_all();
        assert_eq!(records.len(), 2);
    }
}
