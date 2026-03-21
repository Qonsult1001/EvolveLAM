//! Intelligent Model Router — routes prompts to different models based on task complexity.
//!
//! The router classifies incoming prompts and selects the most cost-effective model
//! capable of handling the task. Simple queries go to fast/cheap models; complex
//! architectural work goes to the most capable models.
//!
//! This is the Model Router component from the .saidSo / RAX Engine architecture.

use std::path::Path;

/// Task complexity tiers for model routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskComplexity {
    /// Simple queries, clarifications, short answers
    Simple,
    /// Standard coding tasks, single-file changes
    Standard,
    /// Multi-file refactoring, architecture decisions
    Complex,
    /// Deep reasoning, evolution, novel problem solving
    Expert,
}

/// A model route — maps a complexity tier to a specific provider/model.
#[derive(Debug, Clone)]
pub struct ModelRoute {
    pub provider: String,
    pub model: String,
    pub tier: TaskComplexity,
}

/// The model router — classifies prompts and selects appropriate models.
pub struct ModelRouter {
    routes: Vec<ModelRoute>,
}

impl ModelRouter {
    /// Load router configuration.
    /// Tries `.yoyo/models.toml` first, falls back to hardcoded defaults.
    pub fn load() -> Self {
        let config_path = Path::new(".yoyo/models.toml");
        if let Ok(content) = std::fs::read_to_string(config_path) {
            if let Some(router) = Self::parse_config(&content) {
                return router;
            }
        }
        Self::default_routes()
    }

    /// Parse a TOML-style config into routes.
    /// Format: lines of `tier = provider/model`
    fn parse_config(content: &str) -> Option<Self> {
        let mut routes = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() != 2 {
                continue;
            }

            let tier_str = parts[0].trim();
            let model_str = parts[1].trim().trim_matches('"');

            let tier = match tier_str {
                "simple" => TaskComplexity::Simple,
                "standard" => TaskComplexity::Standard,
                "complex" => TaskComplexity::Complex,
                "expert" => TaskComplexity::Expert,
                _ => continue,
            };

            let model_parts: Vec<&str> = model_str.splitn(2, '/').collect();
            if model_parts.len() != 2 {
                continue;
            }

            routes.push(ModelRoute {
                provider: model_parts[0].to_string(),
                model: model_parts[1].to_string(),
                tier,
            });
        }

        if routes.is_empty() {
            None
        } else {
            Some(Self { routes })
        }
    }

    /// Default model routes when no config is available.
    fn default_routes() -> Self {
        Self {
            routes: vec![
                ModelRoute {
                    provider: "anthropic".to_string(),
                    model: "claude-haiku-4-5-20251001".to_string(),
                    tier: TaskComplexity::Simple,
                },
                ModelRoute {
                    provider: "anthropic".to_string(),
                    model: "claude-sonnet-4-6".to_string(),
                    tier: TaskComplexity::Standard,
                },
                ModelRoute {
                    provider: "anthropic".to_string(),
                    model: "claude-sonnet-4-6".to_string(),
                    tier: TaskComplexity::Complex,
                },
                ModelRoute {
                    provider: "anthropic".to_string(),
                    model: "claude-opus-4-6".to_string(),
                    tier: TaskComplexity::Expert,
                },
            ],
        }
    }

    /// Classify a prompt and return the best model route.
    pub fn route(&self, input: &str) -> &ModelRoute {
        let complexity = self.classify(input);

        // Find the best matching route (exact tier or closest lower tier)
        self.routes
            .iter()
            .filter(|r| r.tier <= complexity)
            .max_by_key(|r| r.tier)
            .unwrap_or_else(|| {
                self.routes
                    .first()
                    .expect("router must have at least one route")
            })
    }

    /// Classify the complexity of an input prompt.
    fn classify(&self, input: &str) -> TaskComplexity {
        let input_lower = input.to_lowercase();
        let word_count = input.split_whitespace().count();
        let has_code = input.contains("```") || input.contains("fn ") || input.contains("def ");

        // Expert indicators
        let expert_signals = [
            "think hard",
            "think deeply",
            "evolve",
            "architect",
            "redesign",
            "from scratch",
            "entire system",
            "full rewrite",
        ];
        if expert_signals.iter().any(|s| input_lower.contains(s)) {
            return TaskComplexity::Expert;
        }

        // Complex indicators
        let complex_signals = [
            "refactor",
            "multiple files",
            "multi-file",
            "cross-cutting",
            "migration",
            "integrate",
            "architecture",
            "design pattern",
        ];
        if complex_signals.iter().any(|s| input_lower.contains(s)) {
            return TaskComplexity::Complex;
        }

        // Standard: code present, or moderate length technical query
        if has_code || word_count > 30 {
            return TaskComplexity::Standard;
        }

        // Simple: short queries, clarifications
        if word_count <= 15 && !has_code {
            return TaskComplexity::Simple;
        }

        TaskComplexity::Standard
    }

    /// Get all configured routes (for status display).
    pub fn routes(&self) -> &[ModelRoute] {
        &self.routes
    }
}

impl std::fmt::Display for TaskComplexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskComplexity::Simple => write!(f, "simple"),
            TaskComplexity::Standard => write!(f, "standard"),
            TaskComplexity::Complex => write!(f, "complex"),
            TaskComplexity::Expert => write!(f, "expert"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_simple() {
        let router = ModelRouter::default_routes();
        assert_eq!(router.classify("what is rust?"), TaskComplexity::Simple);
        assert_eq!(router.classify("hello"), TaskComplexity::Simple);
        assert_eq!(
            router.classify("explain this error"),
            TaskComplexity::Simple
        );
    }

    #[test]
    fn test_classify_standard() {
        let router = ModelRouter::default_routes();
        assert_eq!(
            router.classify("Write a function that parses JSON and extracts the user field. Here is the current code:\n```rust\nfn parse() {}\n```"),
            TaskComplexity::Standard
        );
    }

    #[test]
    fn test_classify_complex() {
        let router = ModelRouter::default_routes();
        assert_eq!(
            router.classify("Refactor the authentication module to use JWT tokens"),
            TaskComplexity::Complex
        );
        assert_eq!(
            router.classify("We need to integrate the payment system across multiple files"),
            TaskComplexity::Complex
        );
    }

    #[test]
    fn test_classify_expert() {
        let router = ModelRouter::default_routes();
        assert_eq!(
            router.classify("Think hard about how to redesign the entire system architecture"),
            TaskComplexity::Expert
        );
        assert_eq!(
            router.classify("Evolve the codebase to support distributed processing"),
            TaskComplexity::Expert
        );
    }

    #[test]
    fn test_route_returns_appropriate_model() {
        let router = ModelRouter::default_routes();
        let route = router.route("hello");
        assert_eq!(route.tier, TaskComplexity::Simple);
        assert!(route.model.contains("haiku"));

        let route = router.route("Think hard about this architecture problem");
        assert_eq!(route.tier, TaskComplexity::Expert);
        assert!(route.model.contains("opus"));
    }

    #[test]
    fn test_parse_config() {
        let config = r#"
# Model routing config
simple = "ollama/llama3"
standard = "anthropic/claude-sonnet-4-6"
complex = "anthropic/claude-sonnet-4-6"
expert = "anthropic/claude-opus-4-6"
"#;
        let router = ModelRouter::parse_config(config).unwrap();
        assert_eq!(router.routes.len(), 4);
        assert_eq!(router.routes[0].provider, "ollama");
        assert_eq!(router.routes[0].model, "llama3");
    }

    #[test]
    fn test_parse_config_empty() {
        assert!(ModelRouter::parse_config("").is_none());
        assert!(ModelRouter::parse_config("# just comments").is_none());
    }

    #[test]
    fn test_parse_config_invalid_lines_skipped() {
        let config = "simple = \"anthropic/haiku\"\ninvalid line\n";
        let router = ModelRouter::parse_config(config).unwrap();
        assert_eq!(router.routes.len(), 1);
    }

    #[test]
    fn test_default_routes_has_all_tiers() {
        let router = ModelRouter::default_routes();
        let tiers: Vec<TaskComplexity> = router.routes.iter().map(|r| r.tier).collect();
        assert!(tiers.contains(&TaskComplexity::Simple));
        assert!(tiers.contains(&TaskComplexity::Standard));
        assert!(tiers.contains(&TaskComplexity::Complex));
        assert!(tiers.contains(&TaskComplexity::Expert));
    }

    #[test]
    fn test_display_complexity() {
        assert_eq!(TaskComplexity::Simple.to_string(), "simple");
        assert_eq!(TaskComplexity::Expert.to_string(), "expert");
    }
}
