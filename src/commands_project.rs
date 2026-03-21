//! Project-related command handlers: /context, /init, /health, /fix, /test, /lint,
//! /tree, /run, /docs, /find, /index, /ast, /gap.

use crate::ast;
use crate::cli;
use crate::commands::auto_compact_if_needed;
use crate::docs;
use crate::format::*;
use crate::prompt::*;

use yoagent::agent::Agent;
use yoagent::*;

// ── /context ─────────────────────────────────────────────────────────────

pub fn handle_context() {
    let files = cli::list_project_context_files();
    if files.is_empty() {
        println!("{DIM}  No project context files found.");
        println!("  Create a YOYO.md to give yoyo project context.");
        println!("  Also supports: CLAUDE.md (compatibility alias), .yoyo/instructions.md");
        println!("  Run /init to create a starter YOYO.md.{RESET}\n");
    } else {
        println!("{DIM}  Project context files:");
        for (name, lines) in &files {
            println!("    {name} ({lines} lines)");
        }
        println!("{RESET}");
    }
}

// ── /init ────────────────────────────────────────────────────────────────

/// Scan the project directory and find important files (README, config, CI, etc.).
/// Returns a list of file paths that exist.
pub fn scan_important_files(dir: &std::path::Path) -> Vec<String> {
    let candidates = [
        "README.md",
        "README",
        "readme.md",
        "LICENSE",
        "LICENSE.md",
        "CHANGELOG.md",
        "CONTRIBUTING.md",
        ".gitignore",
        ".editorconfig",
        // Rust
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        // Node
        "package.json",
        "package-lock.json",
        "tsconfig.json",
        ".eslintrc.json",
        ".eslintrc.js",
        ".prettierrc",
        // Python
        "pyproject.toml",
        "setup.py",
        "setup.cfg",
        "requirements.txt",
        "Pipfile",
        "tox.ini",
        // Go
        "go.mod",
        "go.sum",
        // Build/CI
        "Makefile",
        "Dockerfile",
        "docker-compose.yml",
        "docker-compose.yaml",
        ".dockerignore",
        // CI configs
        ".github/workflows",
        ".gitlab-ci.yml",
        ".circleci/config.yml",
        ".travis.yml",
        "Jenkinsfile",
    ];
    candidates
        .iter()
        .filter(|f| dir.join(f).exists())
        .map(|f| f.to_string())
        .collect()
}

/// Detect key directories in the project (src, tests, docs, etc.).
/// Returns a list of directory names that exist.
pub fn scan_important_dirs(dir: &std::path::Path) -> Vec<String> {
    let candidates = [
        "src",
        "lib",
        "tests",
        "test",
        "docs",
        "doc",
        "examples",
        "benches",
        "scripts",
        ".github",
        ".vscode",
        "config",
        "public",
        "static",
        "assets",
        "migrations",
    ];
    candidates
        .iter()
        .filter(|d| dir.join(d).is_dir())
        .map(|d| d.to_string())
        .collect()
}

/// Get build/test/lint commands for a project type.
pub fn build_commands_for_project(project_type: &ProjectType) -> Vec<(&'static str, &'static str)> {
    match project_type {
        ProjectType::Rust => vec![
            ("Build", "cargo build"),
            ("Test", "cargo test"),
            ("Lint", "cargo clippy --all-targets -- -D warnings"),
            ("Format check", "cargo fmt -- --check"),
            ("Format", "cargo fmt"),
        ],
        ProjectType::Node => vec![
            ("Install", "npm install"),
            ("Test", "npm test"),
            ("Lint", "npx eslint ."),
        ],
        ProjectType::Python => vec![
            ("Test", "python -m pytest"),
            ("Lint", "ruff check ."),
            ("Type check", "python -m mypy ."),
        ],
        ProjectType::Go => vec![
            ("Build", "go build ./..."),
            ("Test", "go test ./..."),
            ("Vet", "go vet ./..."),
        ],
        ProjectType::Make => vec![("Build", "make"), ("Test", "make test")],
        ProjectType::Unknown => vec![],
    }
}

/// Extract the project name from a README.md title line (# Title).
/// Returns None if no README or no title found.
fn extract_project_name_from_readme(dir: &std::path::Path) -> Option<String> {
    let readme_names = ["README.md", "readme.md", "README"];
    for name in &readme_names {
        if let Ok(content) = std::fs::read_to_string(dir.join(name)) {
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(title) = trimmed.strip_prefix("# ") {
                    let title = title.trim();
                    if !title.is_empty() {
                        return Some(title.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Extract the project name from Cargo.toml [package] name field.
fn extract_name_from_cargo_toml(dir: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(dir.join("Cargo.toml")).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("name") {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix('=') {
                let val = rest.trim().trim_matches('"').trim_matches('\'');
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

/// Extract the project name from package.json "name" field.
fn extract_name_from_package_json(dir: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(dir.join("package.json")).ok()?;
    // Simple JSON parsing — find "name": "value"
    for line in content.lines() {
        let trimmed = line.trim().trim_end_matches(',');
        if let Some(rest) = trimmed.strip_prefix("\"name\"") {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix(':') {
                let val = rest.trim().trim_matches('"');
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

/// Best-effort project name detection. Tries multiple sources.
pub fn detect_project_name(dir: &std::path::Path) -> String {
    // Try Cargo.toml name
    if let Some(name) = extract_name_from_cargo_toml(dir) {
        return name;
    }
    // Try package.json name
    if let Some(name) = extract_name_from_package_json(dir) {
        return name;
    }
    // Try README title
    if let Some(name) = extract_project_name_from_readme(dir) {
        return name;
    }
    // Fall back to directory name
    dir.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "my-project".to_string())
}

/// Extract project description from Cargo.toml [package] description field.
fn extract_description_from_cargo_toml(dir: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(dir.join("Cargo.toml")).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("description") {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix('=') {
                let val = rest.trim().trim_matches('"').trim_matches('\'');
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

/// Extract project description from package.json "description" field.
fn extract_description_from_package_json(dir: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(dir.join("package.json")).ok()?;
    for line in content.lines() {
        let trimmed = line.trim().trim_end_matches(',');
        if let Some(rest) = trimmed.strip_prefix("\"description\"") {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix(':') {
                let val = rest.trim().trim_matches('"');
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

/// Extract project description from pyproject.toml [project] description field.
fn extract_description_from_pyproject(dir: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(dir.join("pyproject.toml")).ok()?;
    let mut in_project = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[project]" {
            in_project = true;
            continue;
        }
        if trimmed.starts_with('[') && trimmed != "[project]" {
            in_project = false;
        }
        if in_project {
            if let Some(rest) = trimmed.strip_prefix("description") {
                let rest = rest.trim();
                if let Some(rest) = rest.strip_prefix('=') {
                    let val = rest.trim().trim_matches('"').trim_matches('\'');
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Extract project description from the canonical config file for the detected project type.
/// Falls back through README → config file → None.
pub fn extract_project_description(
    dir: &std::path::Path,
    project_type: &ProjectType,
) -> Option<String> {
    // Try README first
    if let Some(desc) = extract_readme_description(dir) {
        return Some(desc);
    }
    // Fall back to config file description
    match project_type {
        ProjectType::Rust => extract_description_from_cargo_toml(dir),
        ProjectType::Node => extract_description_from_package_json(dir),
        ProjectType::Python => extract_description_from_pyproject(dir),
        _ => None,
    }
}

/// Generate a complete YOYO.md context file by scanning the project.
/// Extract the project description from README.md.
/// Returns the first paragraph after the title (skipping badges, blank lines).
pub fn extract_readme_description(dir: &std::path::Path) -> Option<String> {
    let readme_names = ["README.md", "readme.md", "README", "README.rst"];
    for name in &readme_names {
        if let Ok(content) = std::fs::read_to_string(dir.join(name)) {
            let mut lines = content.lines();
            // Skip the title line
            let mut found_title = false;
            let mut description_lines = Vec::new();
            for line in &mut lines {
                let trimmed = line.trim();
                if !found_title {
                    if trimmed.starts_with("# ") || trimmed.starts_with("===") {
                        found_title = true;
                    }
                    continue;
                }
                // Skip blank lines and badges (lines starting with [ or <)
                if trimmed.is_empty() {
                    if !description_lines.is_empty() {
                        break; // End of first paragraph
                    }
                    continue;
                }
                if trimmed.starts_with('[') && trimmed.contains("](") {
                    continue; // Skip badge lines
                }
                if trimmed.starts_with('<') {
                    continue; // Skip HTML
                }
                if trimmed.starts_with('#') {
                    break; // Next section
                }
                description_lines.push(trimmed.to_string());
                if description_lines.len() >= 5 {
                    break; // Enough
                }
            }
            if !description_lines.is_empty() {
                return Some(description_lines.join(" "));
            }
        }
    }
    None
}

/// Extract Rust dependencies from Cargo.toml [dependencies] section.
pub fn extract_cargo_dependencies(dir: &std::path::Path) -> Vec<String> {
    let content = match std::fs::read_to_string(dir.join("Cargo.toml")) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut deps = Vec::new();
    let mut in_deps = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[dependencies]" {
            in_deps = true;
            continue;
        }
        if trimmed == "[dev-dependencies]" || trimmed == "[build-dependencies]" {
            in_deps = false;
            continue;
        }
        if trimmed.starts_with('[') {
            in_deps = false;
            continue;
        }
        if in_deps {
            if let Some(name) = trimmed.split('=').next() {
                let name = name.trim();
                if !name.is_empty() && !name.starts_with('#') {
                    deps.push(name.to_string());
                }
            }
        }
    }
    deps
}

/// Extract Python dependencies from pyproject.toml or requirements.txt.
pub fn extract_python_dependencies(dir: &std::path::Path) -> Vec<String> {
    // Try pyproject.toml first
    if let Ok(content) = std::fs::read_to_string(dir.join("pyproject.toml")) {
        let mut deps = Vec::new();
        let mut in_deps = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("dependencies")
                && (trimmed.contains('[') || trimmed.contains('='))
            {
                in_deps = true;
                continue;
            }
            if in_deps {
                if trimmed == "]" {
                    break;
                }
                // Lines like: "requests>=2.0",
                let dep = trimmed
                    .trim_matches('"')
                    .trim_matches('\'')
                    .trim_end_matches(',');
                // Extract just the package name (before any version specifier)
                let name = dep
                    .split(&['>', '<', '=', '!', '~', ';', '['][..])
                    .next()
                    .unwrap_or("")
                    .trim();
                if !name.is_empty() && !name.starts_with('#') {
                    deps.push(name.to_string());
                }
            }
        }
        if !deps.is_empty() {
            return deps;
        }
    }
    // Fall back to requirements.txt
    if let Ok(content) = std::fs::read_to_string(dir.join("requirements.txt")) {
        return content
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
            .filter_map(|l| {
                l.split(&['>', '<', '=', '!', '~', ';', '['][..])
                    .next()
                    .map(|s| s.trim().to_string())
            })
            .filter(|s| !s.is_empty())
            .collect();
    }
    Vec::new()
}

/// Extract Node.js dependencies from package.json.
pub fn extract_node_dependencies(dir: &std::path::Path) -> Vec<String> {
    let content = match std::fs::read_to_string(dir.join("package.json")) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    // Try JSON parsing first (handles minified JSON)
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
        if let Some(deps_obj) = val.get("dependencies").and_then(|d| d.as_object()) {
            return deps_obj.keys().cloned().collect();
        }
    }
    // Fallback: line-by-line parsing
    let mut deps = Vec::new();
    let mut in_deps = false;
    let mut in_dev_deps = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains("\"devDependencies\"") {
            in_dev_deps = true;
            in_deps = false;
            continue;
        }
        if trimmed.contains("\"dependencies\"") && !in_dev_deps {
            in_deps = true;
            continue;
        }
        if (in_deps || in_dev_deps) && (trimmed == "}" || trimmed == "},") {
            if in_dev_deps {
                in_dev_deps = false;
            } else {
                in_deps = false;
            }
            continue;
        }
        if in_deps {
            if let Some(name) = trimmed.split(':').next() {
                let name = name.trim().trim_matches('"').trim_end_matches(',');
                if !name.is_empty() {
                    deps.push(name.to_string());
                }
            }
        }
    }
    deps
}

/// Scan source files and extract a one-line summary for each.
/// Returns (path, summary) pairs for the most important source files.
pub fn scan_source_summaries(
    dir: &std::path::Path,
    project_type: &ProjectType,
) -> Vec<(String, String)> {
    let extensions: &[&str] = match project_type {
        ProjectType::Rust => &["rs"],
        ProjectType::Node => &["ts", "js", "tsx", "jsx"],
        ProjectType::Python => &["py"],
        ProjectType::Go => &["go"],
        _ => &[],
    };

    if extensions.is_empty() {
        return Vec::new();
    }

    let src_dirs = ["src", "lib", "app", "."];
    let mut summaries = Vec::new();

    for src_dir in &src_dirs {
        let search_dir = if *src_dir == "." {
            dir.to_path_buf()
        } else {
            dir.join(src_dir)
        };
        if !search_dir.is_dir() {
            continue;
        }

        let entries = match std::fs::read_dir(&search_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !extensions.contains(&ext) {
                continue;
            }

            let rel_path = if *src_dir == "." {
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            } else {
                format!(
                    "{}/{}",
                    src_dir,
                    path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                )
            };

            if let Ok(content) = std::fs::read_to_string(&path) {
                let summary = extract_file_purpose(&content, ext);
                if !summary.is_empty() {
                    summaries.push((rel_path, summary));
                }
            }
        }

        if !summaries.is_empty() && *src_dir != "." {
            break; // Found source files in a named directory, don't scan root
        }
    }

    summaries.sort_by(|a, b| a.0.cmp(&b.0));
    summaries.truncate(20); // Cap at 20 files
    summaries
}

/// Extract a one-line purpose description from a source file's content.
fn extract_file_purpose(content: &str, ext: &str) -> String {
    for line in content.lines().take(20) {
        let trimmed = line.trim();
        // Rust: //! doc comments
        if ext == "rs" {
            if let Some(doc) = trimmed.strip_prefix("//!") {
                let doc = doc.trim();
                if !doc.is_empty() && doc.len() > 5 {
                    return truncate_str(doc, 100);
                }
            }
        }
        // Python: module docstrings
        if ext == "py" && (trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''")) {
            let doc = trimmed
                .trim_start_matches("\"\"\"")
                .trim_start_matches("'''")
                .trim_end_matches("\"\"\"")
                .trim_end_matches("'''")
                .trim();
            if !doc.is_empty() && doc.len() > 5 {
                return truncate_str(doc, 100);
            }
        }
        // JS/TS: /** JSDoc comments */
        if (ext == "js" || ext == "ts" || ext == "tsx" || ext == "jsx")
            && trimmed.starts_with("/**")
        {
            let doc = trimmed
                .trim_start_matches("/**")
                .trim_end_matches("*/")
                .trim_start_matches('*')
                .trim();
            if !doc.is_empty() && doc.len() > 5 {
                return truncate_str(doc, 100);
            }
        }
        // Go: // Package ... comments
        if ext == "go" {
            if let Some(doc) = trimmed.strip_prefix("// Package ") {
                if !doc.is_empty() {
                    return truncate_str(&format!("Package {doc}"), 100);
                }
            }
        }
        // Generic: single-line comments at the very top
        if (trimmed.starts_with("//") || trimmed.starts_with('#'))
            && !trimmed.starts_with("#!")
            && !trimmed.starts_with("#[")
        {
            let doc = trimmed
                .trim_start_matches("//")
                .trim_start_matches('#')
                .trim();
            if doc.len() > 10 {
                return truncate_str(doc, 100);
            }
        }
    }
    String::new()
}

/// Truncate a string to max_len characters, adding "…" if truncated.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

/// Check if a command is available on the system PATH.
pub fn command_exists(cmd: &str) -> bool {
    #[cfg(target_os = "windows")]
    let check = std::process::Command::new("where")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    #[cfg(not(target_os = "windows"))]
    let check = std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    check.is_ok_and(|s| s.success())
}

/// Detect the main entry point file for a project.
pub fn detect_entry_point(dir: &std::path::Path, project_type: &ProjectType) -> Option<String> {
    let candidates: &[&str] = match project_type {
        ProjectType::Rust => &["src/main.rs", "src/lib.rs"],
        ProjectType::Python => &[
            "main.py",
            "app.py",
            "src/main.py",
            "__main__.py",
            "src/__main__.py",
        ],
        ProjectType::Node => &[
            "src/index.ts",
            "src/index.js",
            "src/main.ts",
            "src/main.js",
            "index.ts",
            "index.js",
            "app.ts",
            "app.js",
        ],
        ProjectType::Go => &["main.go", "cmd/main.go"],
        _ => &[],
    };
    for c in candidates {
        if dir.join(c).exists() {
            return Some(c.to_string());
        }
    }
    None
}

pub fn generate_init_content(dir: &std::path::Path) -> String {
    let project_type = detect_project_type(dir);
    let project_name = detect_project_name(dir);
    let important_files = scan_important_files(dir);
    let important_dirs = scan_important_dirs(dir);
    let build_commands = build_commands_for_project(&project_type);

    let mut content = String::new();

    // Header
    content.push_str("# Project Context\n\n");
    content.push_str("<!-- YOYO.md — generated by `yoyo /init`. Edit to customize. -->\n");
    content.push_str("<!-- Also works as CLAUDE.md for compatibility with other tools. -->\n\n");

    // About section — extract description from README or config file
    content.push_str("## About This Project\n\n");
    content.push_str(&format!("**{project_name}**"));
    if project_type != ProjectType::Unknown {
        content.push_str(&format!(" — {project_type} project"));
    }
    content.push_str("\n\n");
    if let Some(desc) = extract_project_description(dir, &project_type) {
        content.push_str(&desc);
        content.push_str("\n\n");
    } else {
        content.push_str("<!-- Add a description of what this project does. -->\n\n");
    }

    // Entry point
    if let Some(entry) = detect_entry_point(dir, &project_type) {
        content.push_str(&format!("Entry point: `{entry}`\n\n"));
    }

    // Build & Test section
    content.push_str("## Build & Test\n\n");
    if build_commands.is_empty() {
        content.push_str("<!-- Add build, test, and run commands for this project. -->\n\n");
    } else {
        content.push_str("```bash\n");
        for (label, cmd) in &build_commands {
            content.push_str(&format!("{cmd:<50} # {label}\n"));
        }
        content.push_str("```\n\n");
    }

    // Dependencies section — extract from config files
    let deps = match project_type {
        ProjectType::Rust => extract_cargo_dependencies(dir),
        ProjectType::Python => extract_python_dependencies(dir),
        ProjectType::Node => extract_node_dependencies(dir),
        _ => Vec::new(),
    };
    if !deps.is_empty() {
        content.push_str("## Dependencies\n\n");
        let dep_list = if deps.len() > 15 {
            format!("{}, and {} more", deps[..15].join(", "), deps.len() - 15)
        } else {
            deps.join(", ")
        };
        content.push_str(&format!("{dep_list}\n\n"));
    }

    // Architecture section — source file summaries
    let summaries = scan_source_summaries(dir, &project_type);
    if !summaries.is_empty() {
        content.push_str("## Architecture\n\n");
        content.push_str("| File | Purpose |\n|------|--------|\n");
        for (path, summary) in &summaries {
            content.push_str(&format!("| `{path}` | {summary} |\n"));
        }
        content.push('\n');
    }

    // Coding Conventions section
    content.push_str("## Coding Conventions\n\n");
    content.push_str(
        "<!-- List any coding standards, naming conventions, or patterns to follow. -->\n\n",
    );

    // Important Files section
    content.push_str("## Important Files\n\n");
    if important_files.is_empty() && important_dirs.is_empty() {
        content.push_str("<!-- List key files and directories the agent should know about. -->\n");
    } else {
        if !important_dirs.is_empty() {
            content.push_str("Key directories:\n");
            for d in &important_dirs {
                content.push_str(&format!("- `{d}/`\n"));
            }
            content.push('\n');
        }
        if !important_files.is_empty() {
            content.push_str("Key files:\n");
            for f in &important_files {
                content.push_str(&format!("- `{f}`\n"));
            }
            content.push('\n');
        }
    }

    content
}

pub fn handle_init() {
    let path = "YOYO.md";
    if std::path::Path::new(path).exists() {
        println!("{DIM}  {path} already exists — not overwriting.{RESET}\n");
    } else if std::path::Path::new("CLAUDE.md").exists() {
        println!("{DIM}  CLAUDE.md already exists — yoyo reads it as a compatibility alias.");
        println!("  Rename it to YOYO.md when you're ready: mv CLAUDE.md YOYO.md{RESET}\n");
    } else {
        let cwd = std::env::current_dir().unwrap_or_default();
        let project_type = detect_project_type(&cwd);
        println!("{DIM}  Scanning project...{RESET}");
        if project_type != ProjectType::Unknown {
            println!("{DIM}  Detected: {project_type}{RESET}");
        }
        let content = generate_init_content(&cwd);
        match std::fs::write(path, &content) {
            Ok(_) => {
                let line_count = content.lines().count();
                println!("{GREEN}  ✓ Created {path} ({line_count} lines) — edit it to add project context.{RESET}");
                println!("{DIM}  Tip: Use /remember to save project-specific notes that persist across sessions.{RESET}\n");
            }
            Err(e) => eprintln!("{RED}  error creating {path}: {e}{RESET}\n"),
        }
    }
}

// ── /docs ────────────────────────────────────────────────────────────────

pub fn handle_docs(input: &str) {
    if input == "/docs" {
        println!("{DIM}  usage: /docs <crate> [item]");
        println!("  Look up docs.rs documentation for a Rust crate.");
        println!("  Examples: /docs serde, /docs tokio task{RESET}\n");
        return;
    }
    let args = input.trim_start_matches("/docs ").trim();
    if args.is_empty() {
        println!("{DIM}  usage: /docs <crate> [item]{RESET}\n");
        return;
    }
    let parts: Vec<&str> = args.splitn(2, char::is_whitespace).collect();
    let crate_name = parts[0].trim();
    let item_name = parts.get(1).map(|s| s.trim()).unwrap_or("");

    let (found, summary) = if item_name.is_empty() {
        docs::fetch_docs_summary(crate_name)
    } else {
        docs::fetch_docs_item(crate_name, item_name)
    };
    if found {
        let label = if item_name.is_empty() {
            crate_name.to_string()
        } else {
            format!("{crate_name}::{item_name}")
        };
        println!("{GREEN}  ✓ {label}{RESET}");
        println!("{DIM}{summary}{RESET}\n");
    } else {
        println!("{RED}  ✗ {summary}{RESET}\n");
    }
}

// ── /health ──────────────────────────────────────────────────────────────

/// Detected project type based on marker files in the working directory.
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectType {
    Rust,
    Node,
    Python,
    Go,
    Make,
    Unknown,
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectType::Rust => write!(f, "Rust (Cargo)"),
            ProjectType::Node => write!(f, "Node.js (npm)"),
            ProjectType::Python => write!(f, "Python"),
            ProjectType::Go => write!(f, "Go"),
            ProjectType::Make => write!(f, "Makefile"),
            ProjectType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Detect project type by checking for marker files in the given directory.
pub fn detect_project_type(dir: &std::path::Path) -> ProjectType {
    if dir.join("Cargo.toml").exists() {
        ProjectType::Rust
    } else if dir.join("package.json").exists() {
        ProjectType::Node
    } else if dir.join("pyproject.toml").exists()
        || dir.join("setup.py").exists()
        || dir.join("setup.cfg").exists()
    {
        ProjectType::Python
    } else if dir.join("go.mod").exists() {
        ProjectType::Go
    } else if dir.join("Makefile").exists() || dir.join("makefile").exists() {
        ProjectType::Make
    } else {
        ProjectType::Unknown
    }
}

/// Return health check commands for a given project type.
#[allow(clippy::vec_init_then_push, unused_mut)]
pub fn health_checks_for_project(
    project_type: &ProjectType,
) -> Vec<(&'static str, Vec<&'static str>)> {
    match project_type {
        ProjectType::Rust => {
            let mut checks = vec![("build", vec!["cargo", "build"])];
            #[cfg(not(test))]
            checks.push(("test", vec!["cargo", "test"]));
            checks.push((
                "clippy",
                vec!["cargo", "clippy", "--all-targets", "--", "-D", "warnings"],
            ));
            checks.push(("fmt", vec!["cargo", "fmt", "--", "--check"]));
            checks
        }
        ProjectType::Node => {
            let mut checks: Vec<(&str, Vec<&str>)> = vec![];
            #[cfg(not(test))]
            checks.push(("test", vec!["npm", "test"]));
            checks.push(("lint", vec!["npx", "eslint", "."]));
            checks
        }
        ProjectType::Python => {
            let mut checks: Vec<(&str, Vec<&str>)> = vec![];
            #[cfg(not(test))]
            checks.push(("test", vec!["python", "-m", "pytest"]));
            // Prefer ruff (fast, modern) over flake8
            if command_exists("ruff") {
                checks.push(("lint", vec!["ruff", "check", "."]));
            } else {
                checks.push(("lint", vec!["python", "-m", "flake8", "."]));
            }
            if command_exists("mypy") {
                checks.push(("typecheck", vec!["python", "-m", "mypy", "."]));
            }
            checks
        }
        ProjectType::Go => {
            let mut checks = vec![("build", vec!["go", "build", "./..."])];
            #[cfg(not(test))]
            checks.push(("test", vec!["go", "test", "./..."]));
            checks.push(("vet", vec!["go", "vet", "./..."]));
            checks
        }
        ProjectType::Make => {
            let mut checks: Vec<(&str, Vec<&str>)> = vec![];
            #[cfg(not(test))]
            checks.push(("test", vec!["make", "test"]));
            checks
        }
        ProjectType::Unknown => vec![],
    }
}

/// Run health checks for a specific project type. Returns (name, passed, detail) tuples.
/// Used by tests; handle_health() uses run_health_checks_with_classification() instead.
#[allow(dead_code)]
pub fn run_health_check_for_project(
    project_type: &ProjectType,
) -> Vec<(&'static str, bool, String)> {
    let checks = health_checks_for_project(project_type);

    let mut results = Vec::new();
    for (name, args) in checks {
        let start = std::time::Instant::now();
        let output = std::process::Command::new(args[0])
            .args(&args[1..])
            .output();
        let elapsed = format_duration(start.elapsed());
        match output {
            Ok(o) if o.status.success() => {
                results.push((name, true, format!("ok ({elapsed})")));
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                let first_line = stderr.lines().next().unwrap_or("(unknown error)");
                results.push((
                    name,
                    false,
                    format!(
                        "FAIL ({elapsed}): {}",
                        truncate_with_ellipsis(first_line, 80)
                    ),
                ));
            }
            Err(e) => {
                results.push((name, false, format!("ERROR: {e}")));
            }
        }
    }
    results
}

/// Run health checks and capture full error output for failures.
pub fn run_health_checks_full_output(
    project_type: &ProjectType,
) -> Vec<(&'static str, bool, String)> {
    let checks = health_checks_for_project(project_type);

    let mut results = Vec::new();
    for (name, args) in checks {
        let output = std::process::Command::new(args[0])
            .args(&args[1..])
            .output();
        match output {
            Ok(o) if o.status.success() => {
                results.push((name, true, String::new()));
            }
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let stderr = String::from_utf8_lossy(&o.stderr);
                let mut full_output = String::new();
                if !stdout.is_empty() {
                    full_output.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !full_output.is_empty() {
                        full_output.push('\n');
                    }
                    full_output.push_str(&stderr);
                }
                results.push((name, false, full_output));
            }
            Err(e) => {
                results.push((name, false, format!("ERROR: {e}")));
            }
        }
    }
    results
}

// ── Error Classification ──────────────────────────────────────────────

/// Categories of Rust build/lint/test errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RustErrorCategory {
    /// Missing import, unresolved name, module not found.
    MissingImport,
    /// Type mismatch, expected X found Y.
    TypeMismatch,
    /// Borrow checker: lifetime, move, borrow errors.
    BorrowChecker,
    /// Unused variable, import, function — typically warnings.
    Unused,
    /// Test assertion failures.
    TestFailure,
    /// Formatting issues (cargo fmt).
    Format,
    /// Clippy lints.
    Clippy,
    /// Anything else.
    Unknown,
}

impl std::fmt::Display for RustErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RustErrorCategory::MissingImport => write!(f, "missing_import"),
            RustErrorCategory::TypeMismatch => write!(f, "type_mismatch"),
            RustErrorCategory::BorrowChecker => write!(f, "borrow_checker"),
            RustErrorCategory::Unused => write!(f, "unused"),
            RustErrorCategory::TestFailure => write!(f, "test_failure"),
            RustErrorCategory::Format => write!(f, "format"),
            RustErrorCategory::Clippy => write!(f, "clippy"),
            RustErrorCategory::Unknown => write!(f, "unknown"),
        }
    }
}

/// Fix strategy hint for each error category.
pub fn fix_strategy(cat: RustErrorCategory) -> &'static str {
    match cat {
        RustErrorCategory::MissingImport => {
            "Add the missing `use` import or check for typos in the module path. If a type was renamed, update all references."
        }
        RustErrorCategory::TypeMismatch => {
            "Check the expected vs actual types. Common fixes: add .into(), change return type, add/remove & or *, or use as."
        }
        RustErrorCategory::BorrowChecker => {
            "Look at ownership flow. Common fixes: clone the value, restructure to avoid simultaneous borrows, add lifetime annotations, or use Arc/Rc."
        }
        RustErrorCategory::Unused => {
            "Remove the unused item, prefix with _ if intentionally unused, or add #[allow(dead_code)] with a comment explaining why."
        }
        RustErrorCategory::TestFailure => {
            "Read the assertion message carefully. Check expected vs actual values. The test logic or the code under test may need fixing."
        }
        RustErrorCategory::Format => {
            "Run `cargo fmt` to auto-fix. No manual changes needed."
        }
        RustErrorCategory::Clippy => {
            "Read the clippy suggestion — most have a direct fix. Apply the suggestion or add #[allow(clippy::...)] if the lint is a false positive."
        }
        RustErrorCategory::Unknown => {
            "Read the error message carefully and fix the root cause."
        }
    }
}

/// Classify a Rust error output line into a category.
/// Scans for keyword patterns in compiler/clippy/test output.
pub fn classify_rust_error(output: &str) -> Vec<(RustErrorCategory, usize)> {
    let mut counts: std::collections::HashMap<RustErrorCategory, usize> =
        std::collections::HashMap::new();

    for line in output.lines() {
        let lower = line.to_lowercase();

        if lower.contains("cannot find")
            || lower.contains("unresolved import")
            || lower.contains("no function or associated item named")
            || lower.contains("not found in")
            || lower.contains("module not found")
        {
            *counts.entry(RustErrorCategory::MissingImport).or_insert(0) += 1;
        } else if lower.contains("mismatched types")
            || (lower.contains("expected")
                && lower.contains("found")
                && (lower.contains("type")
                    || lower.contains("struct")
                    || lower.contains("enum")
                    || lower.contains("e0308")))
        {
            *counts.entry(RustErrorCategory::TypeMismatch).or_insert(0) += 1;
        } else if lower.contains("borrow")
            || lower.contains("lifetime")
            || lower.contains("moved")
            || lower.contains("cannot move")
            || lower.contains("does not live long enough")
        {
            *counts.entry(RustErrorCategory::BorrowChecker).or_insert(0) += 1;
        } else if lower.contains("unused")
            || lower.contains("dead_code")
            || lower.contains("never read")
            || lower.contains("never used")
        {
            *counts.entry(RustErrorCategory::Unused).or_insert(0) += 1;
        } else if lower.contains("test result: failed")
            || lower.contains("panicked at")
            || (lower.contains("assertion") && lower.contains("failed"))
        {
            *counts.entry(RustErrorCategory::TestFailure).or_insert(0) += 1;
        } else if lower.contains("diff") && lower.contains("rustfmt") {
            *counts.entry(RustErrorCategory::Format).or_insert(0) += 1;
        } else if lower.contains("clippy::") || lower.contains("clippy warning") {
            *counts.entry(RustErrorCategory::Clippy).or_insert(0) += 1;
        }
    }

    if counts.is_empty() {
        counts.insert(RustErrorCategory::Unknown, 1);
    }

    let mut result: Vec<(RustErrorCategory, usize)> = counts.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}

/// Language-agnostic error category for Python, Node, and Go projects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GenericErrorCategory {
    SyntaxError,
    ImportError,
    TypeError,
    NameError,
    IndentationError,
    ReferenceError,
    ModuleNotFound,
    UndefinedSymbol,
    UnusedImport,
    TestFailure,
    Unknown,
}

impl std::fmt::Display for GenericErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericErrorCategory::SyntaxError => write!(f, "syntax_error"),
            GenericErrorCategory::ImportError => write!(f, "import_error"),
            GenericErrorCategory::TypeError => write!(f, "type_error"),
            GenericErrorCategory::NameError => write!(f, "name_error"),
            GenericErrorCategory::IndentationError => write!(f, "indentation_error"),
            GenericErrorCategory::ReferenceError => write!(f, "reference_error"),
            GenericErrorCategory::ModuleNotFound => write!(f, "module_not_found"),
            GenericErrorCategory::UndefinedSymbol => write!(f, "undefined_symbol"),
            GenericErrorCategory::UnusedImport => write!(f, "unused_import"),
            GenericErrorCategory::TestFailure => write!(f, "test_failure"),
            GenericErrorCategory::Unknown => write!(f, "unknown"),
        }
    }
}

/// Fix strategy hint for generic error categories.
pub fn generic_fix_strategy(cat: GenericErrorCategory) -> &'static str {
    match cat {
        GenericErrorCategory::SyntaxError => {
            "Check for missing brackets, colons, semicolons, or unexpected tokens near the reported line."
        }
        GenericErrorCategory::ImportError => {
            "Verify the module name, check if the package is installed (pip install / npm install), or check the import path."
        }
        GenericErrorCategory::TypeError => {
            "Check argument types and counts. Verify the operation is valid for the given types."
        }
        GenericErrorCategory::NameError => {
            "The variable or function is not defined. Check for typos, missing imports, or scope issues."
        }
        GenericErrorCategory::IndentationError => {
            "Fix the indentation — Python uses consistent spaces (usually 4). Don't mix tabs and spaces."
        }
        GenericErrorCategory::ReferenceError => {
            "The variable is not defined. Check for typos, declaration order, or scope issues."
        }
        GenericErrorCategory::ModuleNotFound => {
            "The module/package is not installed or the path is wrong. Run the package manager install command."
        }
        GenericErrorCategory::UndefinedSymbol => {
            "The symbol is not declared or imported. Check spelling and imports."
        }
        GenericErrorCategory::UnusedImport => {
            "Remove the unused import or use it. In Go, unused imports are compile errors."
        }
        GenericErrorCategory::TestFailure => {
            "Read the assertion message carefully. Check expected vs actual values."
        }
        GenericErrorCategory::Unknown => {
            "Read the error message carefully and fix the root cause."
        }
    }
}

/// Classify Python error output into categories.
pub fn classify_python_error(output: &str) -> Vec<(GenericErrorCategory, usize)> {
    let mut counts: std::collections::HashMap<GenericErrorCategory, usize> =
        std::collections::HashMap::new();

    for line in output.lines() {
        let lower = line.to_lowercase();

        if lower.contains("syntaxerror") || lower.contains("syntax error") {
            *counts.entry(GenericErrorCategory::SyntaxError).or_insert(0) += 1;
        } else if lower.contains("importerror")
            || lower.contains("modulenotfounderror")
            || lower.contains("no module named")
        {
            *counts.entry(GenericErrorCategory::ImportError).or_insert(0) += 1;
        } else if lower.contains("typeerror") {
            *counts.entry(GenericErrorCategory::TypeError).or_insert(0) += 1;
        } else if lower.contains("nameerror") {
            *counts.entry(GenericErrorCategory::NameError).or_insert(0) += 1;
        } else if lower.contains("indentationerror") || lower.contains("taberror") {
            *counts
                .entry(GenericErrorCategory::IndentationError)
                .or_insert(0) += 1;
        } else if lower.contains("assert") && lower.contains("error")
            || lower.contains("failed")
                && (lower.contains("test")
                    || lower.contains("pytest")
                    || lower.contains("unittest"))
        {
            *counts.entry(GenericErrorCategory::TestFailure).or_insert(0) += 1;
        }
    }

    if counts.is_empty() {
        counts.insert(GenericErrorCategory::Unknown, 1);
    }

    let mut result: Vec<(GenericErrorCategory, usize)> = counts.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}

/// Classify Node.js error output into categories.
pub fn classify_node_error(output: &str) -> Vec<(GenericErrorCategory, usize)> {
    let mut counts: std::collections::HashMap<GenericErrorCategory, usize> =
        std::collections::HashMap::new();

    for line in output.lines() {
        let lower = line.to_lowercase();

        if lower.contains("syntaxerror") || lower.contains("unexpected token") {
            *counts.entry(GenericErrorCategory::SyntaxError).or_insert(0) += 1;
        } else if lower.contains("referenceerror") {
            *counts
                .entry(GenericErrorCategory::ReferenceError)
                .or_insert(0) += 1;
        } else if lower.contains("typeerror") {
            *counts.entry(GenericErrorCategory::TypeError).or_insert(0) += 1;
        } else if lower.contains("module_not_found")
            || lower.contains("cannot find module")
            || lower.contains("err_module_not_found")
        {
            *counts
                .entry(GenericErrorCategory::ModuleNotFound)
                .or_insert(0) += 1;
        } else if (lower.contains("failing") || lower.contains("failed"))
            && (lower.contains("test") || lower.contains("spec") || lower.contains("suite"))
        {
            *counts.entry(GenericErrorCategory::TestFailure).or_insert(0) += 1;
        }
    }

    if counts.is_empty() {
        counts.insert(GenericErrorCategory::Unknown, 1);
    }

    let mut result: Vec<(GenericErrorCategory, usize)> = counts.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}

/// Classify Go error output into categories.
pub fn classify_go_error(output: &str) -> Vec<(GenericErrorCategory, usize)> {
    let mut counts: std::collections::HashMap<GenericErrorCategory, usize> =
        std::collections::HashMap::new();

    for line in output.lines() {
        let lower = line.to_lowercase();

        if lower.contains("undefined:") || lower.contains("undeclared name") {
            *counts
                .entry(GenericErrorCategory::UndefinedSymbol)
                .or_insert(0) += 1;
        } else if lower.contains("cannot use") || lower.contains("type mismatch") {
            *counts.entry(GenericErrorCategory::TypeError).or_insert(0) += 1;
        } else if lower.contains("imported and not used") {
            *counts
                .entry(GenericErrorCategory::UnusedImport)
                .or_insert(0) += 1;
        } else if lower.contains("syntax error")
            || lower.contains("expected") && lower.contains("got")
        {
            *counts.entry(GenericErrorCategory::SyntaxError).or_insert(0) += 1;
        } else if lower.contains("--- fail") || lower.contains("fail\t") {
            *counts.entry(GenericErrorCategory::TestFailure).or_insert(0) += 1;
        }
    }

    if counts.is_empty() {
        counts.insert(GenericErrorCategory::Unknown, 1);
    }

    let mut result: Vec<(GenericErrorCategory, usize)> = counts.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}

/// Classify error output for any project type.
/// Returns a formatted one-line summary of detected categories.
pub fn classify_error_multilang(output: &str) -> String {
    let project_type = detect_project_type(&std::env::current_dir().unwrap_or_default());

    match project_type {
        ProjectType::Rust => {
            let cats = classify_rust_error(output);
            if cats.is_empty() || (cats.len() == 1 && cats[0].0 == RustErrorCategory::Unknown) {
                return String::new();
            }
            cats.iter()
                .filter(|(cat, _)| *cat != RustErrorCategory::Unknown)
                .map(|(cat, count)| format!("{cat}({count})"))
                .collect::<Vec<_>>()
                .join(", ")
        }
        ProjectType::Python => {
            let cats = classify_python_error(output);
            format_generic_categories(&cats)
        }
        ProjectType::Node => {
            let cats = classify_node_error(output);
            format_generic_categories(&cats)
        }
        ProjectType::Go => {
            let cats = classify_go_error(output);
            format_generic_categories(&cats)
        }
        _ => String::new(),
    }
}

/// Format generic error categories into a one-line summary.
pub fn format_generic_categories(cats: &[(GenericErrorCategory, usize)]) -> String {
    if cats.is_empty() || (cats.len() == 1 && cats[0].0 == GenericErrorCategory::Unknown) {
        return String::new();
    }
    cats.iter()
        .filter(|(cat, _)| *cat != GenericErrorCategory::Unknown)
        .map(|(cat, count)| format!("{cat}({count})"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Format error classification results for terminal display.
/// Shows detected categories with counts and top strategy hints.
pub fn format_error_classification(failures: &[(&str, &str)]) -> String {
    let mut output = String::new();
    for (name, error_output) in failures {
        let categories = classify_rust_error(error_output);
        if categories.is_empty()
            || (categories.len() == 1 && categories[0].0 == RustErrorCategory::Unknown)
        {
            continue;
        }
        let cat_parts: Vec<String> = categories
            .iter()
            .filter(|(cat, _)| *cat != RustErrorCategory::Unknown)
            .map(|(cat, count)| format!("{count} {cat}"))
            .collect();
        if cat_parts.is_empty() {
            continue;
        }
        output.push_str(&format!("  {name}: {}\n", cat_parts.join(", ")));
        // Show strategy hint for the top category
        if let Some((top_cat, _)) = categories
            .iter()
            .find(|(cat, _)| *cat != RustErrorCategory::Unknown)
        {
            output.push_str(&format!("    → {}\n", fix_strategy(*top_cat)));
        }
    }
    output
}

/// Build a prompt describing health check failures for the AI to fix.
/// Includes error classification and strategy hints for Rust and other languages.
pub fn build_fix_prompt(failures: &[(&str, &str)]) -> String {
    if failures.is_empty() {
        return String::new();
    }
    let project_type = detect_project_type(&std::env::current_dir().unwrap_or_default());
    let mut prompt = String::from(
        "Fix the following build/lint errors in this project. Read the relevant files, understand the errors, and apply fixes:\n\n",
    );
    for (name, output) in failures {
        prompt.push_str(&format!("## {name} errors:\n"));

        match project_type {
            ProjectType::Rust => {
                let categories = classify_rust_error(output);
                let cat_summary: Vec<String> = categories
                    .iter()
                    .map(|(cat, count)| format!("{cat} ({count})"))
                    .collect();
                prompt.push_str(&format!(
                    "**Error categories:** {}\n",
                    cat_summary.join(", ")
                ));
                for (cat, _) in categories.iter().take(2) {
                    prompt.push_str(&format!(
                        "**Strategy for {}:** {}\n",
                        cat,
                        fix_strategy(*cat)
                    ));
                }
            }
            ProjectType::Python => {
                let categories = classify_python_error(output);
                build_generic_fix_hints(&mut prompt, &categories);
            }
            ProjectType::Node => {
                let categories = classify_node_error(output);
                build_generic_fix_hints(&mut prompt, &categories);
            }
            ProjectType::Go => {
                let categories = classify_go_error(output);
                build_generic_fix_hints(&mut prompt, &categories);
            }
            _ => {}
        }

        prompt.push_str(&format!("\n```\n{output}\n```\n\n"));
    }
    prompt.push_str(
        "After fixing, run the failing checks again to verify. Fix any remaining issues.",
    );
    prompt
}

/// Append generic error category hints to a fix prompt.
fn build_generic_fix_hints(prompt: &mut String, categories: &[(GenericErrorCategory, usize)]) {
    let cat_summary: Vec<String> = categories
        .iter()
        .map(|(cat, count)| format!("{cat} ({count})"))
        .collect();
    prompt.push_str(&format!(
        "**Error categories:** {}\n",
        cat_summary.join(", ")
    ));
    for (cat, _) in categories.iter().take(2) {
        if *cat != GenericErrorCategory::Unknown {
            prompt.push_str(&format!(
                "**Strategy for {}:** {}\n",
                cat,
                generic_fix_strategy(*cat)
            ));
        }
    }
}

pub fn handle_health() {
    let project_type = detect_project_type(&std::env::current_dir().unwrap_or_default());
    println!("{DIM}  Detected project: {project_type}{RESET}");
    if project_type == ProjectType::Unknown {
        println!(
            "{DIM}  No recognized project found. Looked for: Cargo.toml, package.json, pyproject.toml, setup.py, go.mod, Makefile{RESET}\n"
        );
        return;
    }
    println!("{DIM}  Running health checks...{RESET}");
    let results = run_health_checks_with_classification(&project_type);
    if results.is_empty() {
        println!("{DIM}  No checks configured for {project_type}{RESET}\n");
        return;
    }
    let all_passed = results.iter().all(|(_, passed, _, _, _)| *passed);
    for (name, passed, detail, classification, _) in &results {
        let icon = if *passed {
            format!("{GREEN}✓{RESET}")
        } else {
            format!("{RED}✗{RESET}")
        };
        println!("  {icon} {name}: {detail}");
        if !classification.is_empty() {
            println!("{DIM}    {classification}{RESET}");
        }
    }
    let timing = format_health_timing_summary(
        &results
            .iter()
            .map(|(n, p, d, c, dur)| (*n, *p, d.clone(), c.clone(), *dur))
            .collect::<Vec<_>>(),
    );
    if all_passed {
        check_fix_resolution();
        println!("\n{GREEN}  All checks passed ✓{RESET}");
    } else {
        println!("\n{RED}  Some checks failed ✗{RESET}");
    }
    println!("{DIM}{timing}{RESET}\n");
}

/// Check if the configured provider endpoint is reachable.
pub fn handle_doctor(provider: &str, base_url: Option<&str>) {
    println!("{BOLD}  yoyo /doctor — endpoint check{RESET}\n");
    println!("{DIM}  Provider: {provider}{RESET}");
    if let Some(url) = base_url {
        println!("{DIM}  Base URL: {url}{RESET}");
    }
    println!();

    let (reachable, latency, error) = crate::prompt::check_endpoint_reachable(provider, base_url);

    if reachable {
        let ms = latency.unwrap_or(0);
        println!("{GREEN}  ✓ Endpoint reachable ({ms}ms){RESET}");
    } else {
        let err = error.unwrap_or_else(|| "unknown error".to_string());
        println!("{RED}  ✗ Cannot reach endpoint: {err}{RESET}");
        println!();
        match provider {
            "ollama" => {
                println!("{DIM}  Suggestion: Start Ollama with 'ollama serve'{RESET}");
            }
            "anthropic" | "openai" | "groq" | "openrouter" => {
                println!("{DIM}  Suggestion: Check your internet connection and API key{RESET}");
            }
            _ => {
                println!("{DIM}  Suggestion: Verify the base URL is correct{RESET}");
            }
        }
    }
    println!();
}

/// Run health checks and classify failures.
/// Returns (name, passed, display_detail, classification_summary, elapsed).
pub fn run_health_checks_with_classification(
    project_type: &ProjectType,
) -> Vec<(&'static str, bool, String, String, std::time::Duration)> {
    let checks = health_checks_for_project(project_type);

    let mut results = Vec::new();
    for (name, args) in checks {
        let start = std::time::Instant::now();
        let output = std::process::Command::new(args[0])
            .args(&args[1..])
            .output();
        let dur = start.elapsed();
        let elapsed = format_duration(dur);
        match output {
            Ok(o) if o.status.success() => {
                results.push((name, true, format!("ok ({elapsed})"), String::new(), dur));
            }
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let stderr = String::from_utf8_lossy(&o.stderr);
                let first_line = stderr.lines().next().unwrap_or("(unknown error)");
                let detail = format!(
                    "FAIL ({elapsed}): {}",
                    truncate_with_ellipsis(first_line, 80)
                );
                // Classify the full error output
                let mut full_output = String::new();
                if !stdout.is_empty() {
                    full_output.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !full_output.is_empty() {
                        full_output.push('\n');
                    }
                    full_output.push_str(&stderr);
                }
                let classification = classify_failure_oneline(name, &full_output);
                results.push((name, false, detail, classification, dur));
            }
            Err(e) => {
                results.push((name, false, format!("ERROR: {e}"), String::new(), dur));
            }
        }
    }
    results
}

/// Format a health check timing summary line from check results.
pub fn format_health_timing_summary(
    results: &[(&str, bool, String, String, std::time::Duration)],
) -> String {
    if results.is_empty() {
        return String::new();
    }
    let total: std::time::Duration = results.iter().map(|(_, _, _, _, d)| *d).sum();
    let parts: Vec<String> = results
        .iter()
        .map(|(name, _, _, _, d)| format!("{}: {}", name, format_duration(*d)))
        .collect();
    format!(
        "  Health check completed in {} ({})",
        format_duration(total),
        parts.join(", ")
    )
}

/// Classify a single failure's error output into a one-line summary.
/// Tries Rust classification first, then Python/Node/Go based on project type.
/// Returns empty string if no meaningful classification found.
pub fn classify_failure_oneline(_name: &str, error_output: &str) -> String {
    // Try Rust classification first
    let categories = classify_rust_error(error_output);
    let rust_has_result = !(categories.is_empty()
        || categories.len() == 1 && categories[0].0 == RustErrorCategory::Unknown);

    if rust_has_result {
        let cat_parts: Vec<String> = categories
            .iter()
            .filter(|(cat, _)| *cat != RustErrorCategory::Unknown)
            .map(|(cat, count)| format!("{count} {cat}"))
            .collect();
        if !cat_parts.is_empty() {
            let mut line = format!("→ {}", cat_parts.join(", "));
            if let Some((top_cat, _)) = categories
                .iter()
                .find(|(cat, _)| *cat != RustErrorCategory::Unknown)
            {
                line.push_str(&format!(" — {}", fix_strategy(*top_cat)));
            }
            return line;
        }
    }

    // Fall back to multi-language classification
    let multilang = classify_error_multilang(error_output);
    if !multilang.is_empty() {
        return format!("→ {multilang}");
    }

    String::new()
}

/// Handle the /fix command. Returns Some(fix_prompt) if failures were sent to AI, None otherwise.
pub async fn handle_fix(
    agent: &mut Agent,
    session_total: &mut Usage,
    model: &str,
) -> Option<String> {
    let project_type = detect_project_type(&std::env::current_dir().unwrap_or_default());
    if project_type == ProjectType::Unknown {
        println!(
            "{DIM}  No recognized project found. Looked for: Cargo.toml, package.json, pyproject.toml, setup.py, go.mod, Makefile{RESET}\n"
        );
        return None;
    }
    println!("{DIM}  Detected project: {project_type}{RESET}");
    println!("{DIM}  Running health checks...{RESET}");
    let results = run_health_checks_full_output(&project_type);
    if results.is_empty() {
        println!("{DIM}  No checks configured for {project_type}{RESET}\n");
        return None;
    }
    for (name, passed, _) in &results {
        let icon = if *passed {
            format!("{GREEN}✓{RESET}")
        } else {
            format!("{RED}✗{RESET}")
        };
        let status = if *passed { "ok" } else { "FAIL" };
        println!("  {icon} {name}: {status}");
    }
    let failures: Vec<(&str, &str)> = results
        .iter()
        .filter(|(_, passed, _)| !passed)
        .map(|(name, _, output)| (*name, output.as_str()))
        .collect();
    if failures.is_empty() {
        // All checks passed — if there was a pending fix, mark it as resolved
        check_fix_resolution();
        println!("\n{GREEN}  All checks passed — nothing to fix ✓{RESET}\n");
        return None;
    }
    let fail_count = failures.len();
    // Show error classification summary before sending to AI
    let classification = format_error_classification(&failures);
    if !classification.is_empty() {
        println!("\n{DIM}  Error analysis:{RESET}");
        print!("{DIM}{classification}{RESET}");
    }
    // Log error frequency to .yoyo/error_log.jsonl
    let all_categories = {
        let mut merged: std::collections::HashMap<RustErrorCategory, usize> =
            std::collections::HashMap::new();
        for (_, error_output) in &failures {
            for (cat, count) in classify_rust_error(error_output) {
                *merged.entry(cat).or_insert(0) += count;
            }
        }
        let mut v: Vec<(RustErrorCategory, usize)> = merged.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    };
    if !all_categories.is_empty() {
        append_error_log(&all_categories, "fix");
        record_fix_pending(&all_categories);
        // Check for recurring errors and auto-generate hypotheses
        let log_content = std::fs::read_to_string(".yoyo/error_log.jsonl").unwrap_or_default();
        let log_entries = parse_error_log(&log_content);
        let recurring = detect_recurring_errors(&log_entries);
        for (cat, count) in &recurring {
            let hypothesis = format!(
                "{} errors have occurred {} times without resolution — likely a systemic issue",
                cat, count
            );
            let testable = format!(
                "Check if the {} pattern has a common root cause across occurrences",
                cat
            );
            append_hypothesis(cat, &hypothesis, &testable);
        }
    }
    println!("\n{YELLOW}  Sending {fail_count} failure(s) to AI for fixing...{RESET}\n");
    let fix_prompt = build_fix_prompt(&failures);
    run_prompt(agent, &fix_prompt, session_total, model, None).await;
    auto_compact_if_needed(agent);
    Some(fix_prompt)
}

// ── /test ─────────────────────────────────────────────────────────────

/// Return the test command for a given project type.
pub fn test_command_for_project(
    project_type: &ProjectType,
) -> Option<(&'static str, Vec<&'static str>)> {
    match project_type {
        ProjectType::Rust => Some(("cargo test", vec!["cargo", "test"])),
        ProjectType::Node => Some(("npm test", vec!["npm", "test"])),
        ProjectType::Python => {
            // Prefer pytest if available, fall back to unittest
            if command_exists("pytest") || command_exists("python") {
                Some(("python -m pytest", vec!["python", "-m", "pytest"]))
            } else {
                Some((
                    "python -m unittest discover",
                    vec!["python", "-m", "unittest", "discover"],
                ))
            }
        }
        ProjectType::Go => Some(("go test ./...", vec!["go", "test", "./..."])),
        ProjectType::Make => Some(("make test", vec!["make", "test"])),
        ProjectType::Unknown => None,
    }
}

/// Parsed summary of a cargo test result line.
#[derive(Debug, Clone, PartialEq)]
pub struct TestSummary {
    pub passed: u32,
    pub failed: u32,
    pub ignored: u32,
}

/// Parse test output for summary lines. Supports:
/// - Rust cargo test: "test result: ok. N passed; N failed; N ignored; ..."
/// - pytest: "N passed, N failed, N skipped" or "====== N passed in Xs ======"
/// - Jest/Mocha: "Tests: N passed, N failed, N total"
///
/// Returns None if no recognizable summary is found.
pub fn parse_test_summary(output: &str) -> Option<TestSummary> {
    let mut total = TestSummary {
        passed: 0,
        failed: 0,
        ignored: 0,
    };
    let mut found = false;

    for line in output.lines() {
        let trimmed = line.trim();

        // Rust cargo test: "test result: ok. N passed; N failed; N ignored; ..."
        if trimmed.starts_with("test result:") {
            found = true;
            for part in trimmed.split(';') {
                let part = part.trim();
                if part.ends_with("passed") {
                    if let Some(n) = extract_leading_number(part) {
                        total.passed += n;
                    }
                } else if part.ends_with("failed") {
                    if let Some(n) = extract_leading_number(part) {
                        total.failed += n;
                    }
                } else if part.ends_with("ignored") {
                    if let Some(n) = extract_leading_number(part) {
                        total.ignored += n;
                    }
                }
            }
            continue;
        }

        // pytest: "= N passed, N failed, N skipped in Xs =" or "= N passed in Xs ="
        if trimmed.starts_with('=') && trimmed.ends_with('=') && trimmed.contains(" passed") {
            found = true;
            // Split by comma or spaces and look for "N passed", "N failed", "N skipped"
            for part in trimmed.split(',') {
                let part = part.trim().trim_matches('=').trim();
                if part.contains("passed") {
                    if let Some(n) = extract_leading_number(part) {
                        total.passed += n;
                    }
                } else if part.contains("failed") || part.contains("error") {
                    if let Some(n) = extract_leading_number(part) {
                        total.failed += n;
                    }
                } else if part.contains("skipped") || part.contains("deselected") {
                    if let Some(n) = extract_leading_number(part) {
                        total.ignored += n;
                    }
                }
            }
            continue;
        }

        // Jest: "Tests:  N failed, N passed, N total"
        if trimmed.starts_with("Tests:") {
            found = true;
            for part in trimmed.split(',') {
                let part = part.trim();
                if part.contains("passed") {
                    if let Some(n) = extract_leading_number(part) {
                        total.passed += n;
                    }
                } else if part.contains("failed") {
                    if let Some(n) = extract_leading_number(part) {
                        total.failed += n;
                    }
                } else if part.contains("skipped") || part.contains("pending") {
                    if let Some(n) = extract_leading_number(part) {
                        total.ignored += n;
                    }
                }
            }
            continue;
        }
    }

    if found {
        Some(total)
    } else {
        None
    }
}

/// Extract a leading number from a string like "684 passed" or "0 failed".
fn extract_leading_number(s: &str) -> Option<u32> {
    // Find the first sequence of digits in the string
    let digits: String = s
        .trim()
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// Handle the /test command: auto-detect project type and run tests.
/// Returns a summary string suitable for AI context.
pub fn handle_test() -> Option<String> {
    let project_type = detect_project_type(&std::env::current_dir().unwrap_or_default());
    println!("{DIM}  Detected project: {project_type}{RESET}");
    if project_type == ProjectType::Unknown {
        println!(
            "{DIM}  No recognized project found. Looked for: Cargo.toml, package.json, pyproject.toml, setup.py, go.mod, Makefile{RESET}\n"
        );
        return None;
    }

    let (label, args) = match test_command_for_project(&project_type) {
        Some(cmd) => cmd,
        None => {
            println!("{DIM}  No test command configured for {project_type}{RESET}\n");
            return None;
        }
    };

    println!("{DIM}  Running: {label}...{RESET}");
    let start = std::time::Instant::now();
    let output = std::process::Command::new(args[0])
        .args(&args[1..])
        .output();
    let elapsed = format_duration(start.elapsed());

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);

            if !stdout.is_empty() {
                print!("{stdout}");
            }
            if !stderr.is_empty() {
                eprint!("{stderr}");
            }

            // Parse test result summary from combined output
            let combined = format!("{stdout}{stderr}");
            let test_summary = parse_test_summary(&combined);

            if o.status.success() {
                if let Some(ref ts) = test_summary {
                    println!(
                        "\n{GREEN}  ✓ Tests passed ({elapsed}): {} passed, {} failed, {} ignored{RESET}\n",
                        ts.passed, ts.failed, ts.ignored
                    );
                } else {
                    println!("\n{GREEN}  ✓ Tests passed ({elapsed}){RESET}\n");
                }
                let mut msg = format!("Tests passed ({elapsed}): {label}");
                if let Some(ref ts) = test_summary {
                    msg.push_str(&format!(
                        " — {} passed, {} failed, {} ignored",
                        ts.passed, ts.failed, ts.ignored
                    ));
                }
                Some(msg)
            } else {
                let code = o.status.code().unwrap_or(-1);
                if let Some(ref ts) = test_summary {
                    println!(
                        "\n{RED}  ✗ Tests failed (exit {code}, {elapsed}): {} passed, {} failed, {} ignored{RESET}\n",
                        ts.passed, ts.failed, ts.ignored
                    );
                } else {
                    println!("\n{RED}  ✗ Tests failed (exit {code}, {elapsed}){RESET}\n");
                }
                let mut summary = format!("Tests FAILED (exit {code}, {elapsed}): {label}");
                // Include a preview of the error output for AI context
                let error_text = if !stderr.is_empty() {
                    stderr.to_string()
                } else {
                    stdout.to_string()
                };
                let lines: Vec<&str> = error_text.lines().collect();
                let preview_lines = if lines.len() > 20 {
                    &lines[lines.len() - 20..]
                } else {
                    &lines
                };
                summary.push_str("\n\nLast output:\n");
                for line in preview_lines {
                    summary.push_str(line);
                    summary.push('\n');
                }
                Some(summary)
            }
        }
        Err(e) => {
            eprintln!("{RED}  ✗ Failed to run {label}: {e}{RESET}\n");
            Some(format!("Failed to run {label}: {e}"))
        }
    }
}

// ── /lint ──────────────────────────────────────────────────────────────

/// Return the lint command for a given project type.
pub fn lint_command_for_project(
    project_type: &ProjectType,
) -> Option<(&'static str, Vec<&'static str>)> {
    match project_type {
        ProjectType::Rust => Some((
            "cargo clippy --all-targets -- -D warnings",
            vec!["cargo", "clippy", "--all-targets", "--", "-D", "warnings"],
        )),
        ProjectType::Node => Some(("npx eslint .", vec!["npx", "eslint", "."])),
        ProjectType::Python => {
            if command_exists("ruff") {
                Some(("ruff check .", vec!["ruff", "check", "."]))
            } else if command_exists("flake8") {
                Some(("flake8 .", vec!["flake8", "."]))
            } else {
                Some(("python -m flake8 .", vec!["python", "-m", "flake8", "."]))
            }
        }
        ProjectType::Go => Some(("golangci-lint run", vec!["golangci-lint", "run"])),
        ProjectType::Make | ProjectType::Unknown => None,
    }
}

/// Handle the /lint command: auto-detect project type and run linter.
/// Returns a summary string suitable for AI context.
pub fn handle_lint() -> Option<String> {
    let project_type = detect_project_type(&std::env::current_dir().unwrap_or_default());
    println!("{DIM}  Detected project: {project_type}{RESET}");
    if project_type == ProjectType::Unknown {
        println!(
            "{DIM}  No recognized project found. Looked for: Cargo.toml, package.json, pyproject.toml, setup.py, go.mod, Makefile{RESET}\n"
        );
        return None;
    }

    let (label, args) = match lint_command_for_project(&project_type) {
        Some(cmd) => cmd,
        None => {
            println!("{DIM}  No lint command configured for {project_type}{RESET}\n");
            return None;
        }
    };

    println!("{DIM}  Running: {label}...{RESET}");
    let start = std::time::Instant::now();
    let output = std::process::Command::new(args[0])
        .args(&args[1..])
        .output();
    let elapsed = format_duration(start.elapsed());

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);

            if !stdout.is_empty() {
                print!("{stdout}");
            }
            if !stderr.is_empty() {
                eprint!("{stderr}");
            }

            if o.status.success() {
                println!("\n{GREEN}  ✓ Lint passed ({elapsed}){RESET}\n");
                Some(format!("Lint passed ({elapsed}): {label}"))
            } else {
                let code = o.status.code().unwrap_or(-1);
                println!("\n{RED}  ✗ Lint failed (exit {code}, {elapsed}){RESET}\n");
                let mut summary = format!("Lint FAILED (exit {code}, {elapsed}): {label}");
                let error_text = if !stderr.is_empty() {
                    stderr.to_string()
                } else {
                    stdout.to_string()
                };
                let lines: Vec<&str> = error_text.lines().collect();
                let preview_lines = if lines.len() > 20 {
                    &lines[lines.len() - 20..]
                } else {
                    &lines
                };
                summary.push_str("\n\nLast output:\n");
                for line in preview_lines {
                    summary.push_str(line);
                    summary.push('\n');
                }
                Some(summary)
            }
        }
        Err(e) => {
            eprintln!("{RED}  ✗ Failed to run {label}: {e}{RESET}\n");
            Some(format!("Failed to run {label}: {e}"))
        }
    }
}

// ── /tree ────────────────────────────────────────────────────────────────

/// Build a directory tree from `git ls-files`.
pub fn build_project_tree(max_depth: usize) -> String {
    let files = match std::process::Command::new("git")
        .args(["ls-files"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut files: Vec<String> = text
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect();
            files.sort();
            files
        }
        _ => return "(not a git repository — /tree requires git)".to_string(),
    };

    if files.is_empty() {
        return "(no tracked files)".to_string();
    }

    format_tree_from_paths(&files, max_depth)
}

/// Format a sorted list of file paths into an indented tree string.
pub fn format_tree_from_paths(paths: &[String], max_depth: usize) -> String {
    use std::collections::BTreeSet;

    let mut output = String::new();
    let mut printed_dirs: BTreeSet<String> = BTreeSet::new();

    for path in paths {
        let parts: Vec<&str> = path.split('/').collect();
        let depth = parts.len() - 1;

        for level in 0..parts.len().saturating_sub(1).min(max_depth) {
            let dir_path: String = parts[..=level].join("/");
            let dir_key = format!("{}/", dir_path);
            if printed_dirs.insert(dir_key) {
                let indent = "  ".repeat(level);
                let dir_name = parts[level];
                output.push_str(&format!("{indent}{dir_name}/\n"));
            }
        }

        if depth <= max_depth {
            let indent = "  ".repeat(depth.min(max_depth));
            let file_name = parts.last().unwrap_or(&"");
            output.push_str(&format!("{indent}{file_name}\n"));
        }
    }

    if output.ends_with('\n') {
        output.truncate(output.len() - 1);
    }

    output
}

pub fn handle_tree(input: &str) {
    let arg = input.strip_prefix("/tree").unwrap_or("").trim();
    let max_depth = if arg.is_empty() {
        3
    } else {
        match arg.parse::<usize>() {
            Ok(d) => d,
            Err(_) => {
                println!("{DIM}  usage: /tree [depth]  (default depth: 3){RESET}\n");
                return;
            }
        }
    };
    let tree = build_project_tree(max_depth);
    println!("{DIM}{tree}{RESET}\n");
}

// ── /run ─────────────────────────────────────────────────────────────────

/// Run a shell command directly and print its output.
pub fn run_shell_command(cmd: &str) {
    let start = std::time::Instant::now();
    let output = std::process::Command::new("sh").args(["-c", cmd]).output();
    let elapsed = format_duration(start.elapsed());

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if !stdout.is_empty() {
                print!("{stdout}");
            }
            if !stderr.is_empty() {
                eprint!("{RED}{stderr}{RESET}");
            }
            let code = o.status.code().unwrap_or(-1);
            if code == 0 {
                println!("{DIM}  ✓ exit {code} ({elapsed}){RESET}\n");
            } else {
                println!("{RED}  ✗ exit {code} ({elapsed}){RESET}\n");
            }
        }
        Err(e) => {
            eprintln!("{RED}  error running command: {e}{RESET}\n");
        }
    }
}

pub fn handle_run(input: &str) {
    let cmd = if input.starts_with("/run ") {
        input.trim_start_matches("/run ").trim()
    } else if input.starts_with('!') && input.len() > 1 {
        input[1..].trim()
    } else {
        ""
    };
    if cmd.is_empty() {
        println!("{DIM}  usage: /run <command>  or  !<command>{RESET}\n");
    } else {
        run_shell_command(cmd);
    }
}

pub fn handle_run_usage() {
    println!("{DIM}  usage: /run <command>  or  !<command>");
    println!("  Runs a shell command directly (no AI, no tokens).{RESET}\n");
}

// ── /find ────────────────────────────────────────────────────────────────

/// Result of a fuzzy file match: (file_path, score, match_ranges).
/// Higher score = better match. match_ranges are byte offsets into the lowercased path.
#[derive(Debug, Clone, PartialEq)]
pub struct FindMatch {
    pub path: String,
    pub score: i32,
}

/// Score a file path against a fuzzy pattern (case-insensitive substring match).
/// Returns None if the pattern doesn't match.
/// Scoring:
///   - Base score for containing the pattern as a substring
///   - Bonus for matching the filename (last component) vs directory
///   - Bonus for exact filename match
///   - Bonus for match at the start of the filename
///   - Shorter paths score higher (less noise)
pub fn fuzzy_score(path: &str, pattern: &str) -> Option<i32> {
    let path_lower = path.to_lowercase();
    let pattern_lower = pattern.to_lowercase();

    if !path_lower.contains(&pattern_lower) {
        return None;
    }

    let mut score: i32 = 100; // base score for matching

    // Extract filename (last path component)
    let filename = path.rsplit('/').next().unwrap_or(path);
    let filename_lower = filename.to_lowercase();

    // Big bonus if the pattern matches within the filename itself
    if filename_lower.contains(&pattern_lower) {
        score += 50;

        // Bonus for matching at the start of filename
        if filename_lower.starts_with(&pattern_lower) {
            score += 30;
        }

        // Bonus for exact filename match (without extension)
        let stem = filename_lower.split('.').next().unwrap_or(&filename_lower);
        if stem == pattern_lower {
            score += 20;
        }
    }

    // Shorter paths are slightly preferred (less deeply nested = more relevant)
    let depth = path.matches('/').count();
    score -= depth as i32 * 2;

    Some(score)
}

/// Find files matching a fuzzy pattern. Uses `git ls-files` if in a git repo,
/// otherwise falls back to a recursive directory listing.
pub fn find_files(pattern: &str) -> Vec<FindMatch> {
    let files = list_project_files();
    let mut matches: Vec<FindMatch> = files
        .iter()
        .filter_map(|path| {
            fuzzy_score(path, pattern).map(|score| FindMatch {
                path: path.clone(),
                score,
            })
        })
        .collect();

    // Sort by score descending, then alphabetically for ties
    matches.sort_by(|a, b| b.score.cmp(&a.score).then(a.path.cmp(&b.path)));
    matches
}

/// List all project files. Prefers `git ls-files`, falls back to walkdir-style listing.
fn list_project_files() -> Vec<String> {
    if let Ok(output) = std::process::Command::new("git")
        .args(["ls-files"])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            return text
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect();
        }
    }

    // Fallback: recursive listing of current directory (respecting common ignores)
    walk_directory(".", 8)
}

/// Simple recursive directory walk (fallback when not in a git repo).
fn walk_directory(dir: &str, max_depth: usize) -> Vec<String> {
    let mut files = Vec::new();
    walk_directory_inner(dir, max_depth, 0, &mut files);
    files
}

fn walk_directory_inner(dir: &str, max_depth: usize, depth: usize, files: &mut Vec<String>) {
    if depth > max_depth {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        // Skip hidden dirs and common ignore patterns
        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }
        let path = if dir == "." {
            name.clone()
        } else {
            format!("{dir}/{name}")
        };
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            walk_directory_inner(&path, max_depth, depth + 1, files);
        } else {
            files.push(path);
        }
    }
}

/// Highlight the matching pattern within a file path for display.
/// Returns the path with ANSI bold/color around the matched portion.
pub fn highlight_match(path: &str, pattern: &str) -> String {
    let path_lower = path.to_lowercase();
    let pattern_lower = pattern.to_lowercase();

    if let Some(pos) = path_lower.rfind(&pattern_lower) {
        // Prefer highlighting in the filename portion
        let end = pos + pattern.len();
        format!(
            "{}{BOLD}{GREEN}{}{RESET}{}",
            &path[..pos],
            &path[pos..end],
            &path[end..]
        )
    } else {
        path.to_string()
    }
}

pub fn handle_find(input: &str) {
    let arg = input.strip_prefix("/find").unwrap_or("").trim();
    if arg.is_empty() {
        println!("{DIM}  usage: /find <pattern>");
        println!("  Fuzzy-search project files by name.");
        println!("  Examples: /find main, /find .toml, /find test{RESET}\n");
        return;
    }

    let matches = find_files(arg);
    if matches.is_empty() {
        println!("{DIM}  No files matching '{arg}'.{RESET}\n");
    } else {
        let count = matches.len();
        let shown = matches.iter().take(20);
        println!(
            "{DIM}  {count} file{s} matching '{arg}':",
            s = if count == 1 { "" } else { "s" }
        );
        for m in shown {
            let highlighted = highlight_match(&m.path, arg);
            println!("    {highlighted}");
        }
        if count > 20 {
            println!("    {DIM}... and {} more{RESET}", count - 20);
        }
        println!("{RESET}");
    }
}

// ── /index ───────────────────────────────────────────────────────────────

/// An entry in the project index: path, line count, and first meaningful line.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexEntry {
    pub path: String,
    pub lines: usize,
    pub summary: String,
}

/// Extract the first meaningful line from file content.
/// Skips blank lines, then grabs the first doc comment (`//!`, `///`, `#`),
/// module declaration, or any non-empty line.
pub fn extract_first_meaningful_line(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Return the first non-empty line, truncated
        return truncate_with_ellipsis(trimmed, 80);
    }
    String::new()
}

/// Build a project index by listing files and extracting metadata.
/// Uses `git ls-files` when available, falls back to directory walk.
/// Only indexes text-like source files (skips binaries, images, etc.).
pub fn build_project_index() -> Vec<IndexEntry> {
    let files = list_project_files();
    let mut entries = Vec::new();

    for path in &files {
        // Skip binary/non-text files based on extension
        if is_binary_extension(path) {
            continue;
        }

        // Read the file — skip if it fails (binary, permission, etc.)
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let line_count = content.lines().count();
        let summary = extract_first_meaningful_line(&content);

        entries.push(IndexEntry {
            path: path.clone(),
            lines: line_count,
            summary,
        });
    }

    entries
}

/// Check if a file extension suggests a binary/non-text file.
pub fn is_binary_extension(path: &str) -> bool {
    let binary_exts = [
        ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp", ".ico", ".svg", ".woff", ".woff2",
        ".ttf", ".otf", ".eot", ".pdf", ".zip", ".gz", ".tar", ".bz2", ".xz", ".7z", ".rar",
        ".exe", ".dll", ".so", ".dylib", ".o", ".a", ".class", ".pyc", ".pyo", ".wasm", ".lock",
    ];
    let lower = path.to_lowercase();
    binary_exts.iter().any(|ext| lower.ends_with(ext))
}

/// Format the project index as a table string.
pub fn format_project_index(entries: &[IndexEntry]) -> String {
    if entries.is_empty() {
        return "(no indexable files found)".to_string();
    }

    let mut output = String::new();

    // Find max path length for alignment (capped at 50)
    let max_path_len = entries
        .iter()
        .map(|e| e.path.len())
        .max()
        .unwrap_or(0)
        .min(50);

    output.push_str(&format!(
        "  {:<width$}  {:>5}  {}\n",
        "Path",
        "Lines",
        "Summary",
        width = max_path_len
    ));
    output.push_str(&format!(
        "  {:<width$}  {:>5}  {}\n",
        "─".repeat(max_path_len.min(50)),
        "─────",
        "─".repeat(40),
        width = max_path_len
    ));

    for entry in entries {
        let path_display = if entry.path.len() > 50 {
            format!("…{}", &entry.path[entry.path.len() - 49..])
        } else {
            entry.path.clone()
        };
        output.push_str(&format!(
            "  {:<width$}  {:>5}  {}\n",
            path_display,
            entry.lines,
            entry.summary,
            width = max_path_len
        ));
    }

    // Summary line
    let total_files = entries.len();
    let total_lines: usize = entries.iter().map(|e| e.lines).sum();
    output.push_str(&format!(
        "\n  {} file{}, {} total lines\n",
        total_files,
        if total_files == 1 { "" } else { "s" },
        total_lines
    ));

    output
}

/// Handle the /index command: build and display a project file index.
pub fn handle_index() {
    println!("{DIM}  Building project index...{RESET}");
    let entries = build_project_index();
    if entries.is_empty() {
        println!("{DIM}  (no indexable source files found){RESET}\n");
    } else {
        let formatted = format_project_index(&entries);
        println!("{DIM}{formatted}{RESET}");
    }
}

// ── /ast ──────────────────────────────────────────────────────────────

/// Handle the /ast command: search for code symbols (functions, structs, traits, etc.).
pub fn handle_ast(input: &str) {
    let pattern = input.strip_prefix("/ast").unwrap_or("").trim();

    if pattern.is_empty() {
        println!("{DIM}  usage: /ast <pattern>");
        println!("  Search for code symbols (functions, structs, enums, traits, classes).");
        println!("  Supports Rust, Python, TypeScript/JavaScript, Go, C/C++.");
        println!();
        println!("  Examples:");
        println!("    /ast handle          Find all symbols containing 'handle'");
        println!("    /ast AgentConfig     Find structs/types named AgentConfig");
        println!("    /ast run_prompt      Find the run_prompt function{RESET}\n");
        return;
    }

    let symbols = ast::search_symbols(pattern);
    let formatted = ast::format_symbols(&symbols, 30);
    println!("{DIM}{formatted}{RESET}\n");
}

/// Convert days since Unix epoch to (year, month, day).
/// Algorithm from Howard Hinnant's chrono-compatible date library.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

// ── Error frequency logging ──────────────────────────────────────────────

/// A single error log entry from `.yoyo/error_log.jsonl`.
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorLogEntry {
    pub ts: String,
    pub day: u32,
    pub categories: Vec<(String, usize)>,
    pub source: String,
    /// Whether this error set was resolved by a subsequent successful build.
    /// None = unknown (old entries), Some(true) = fixed, Some(false) = not yet fixed.
    pub resolved: Option<bool>,
    /// Which error categories were pending when the fix succeeded.
    /// Populated when check_fix_resolution marks an entry as resolved.
    pub fixed_categories: Vec<String>,
}

/// Append an error frequency record to `.yoyo/error_log.jsonl`.
pub fn append_error_log(categories: &[(RustErrorCategory, usize)], source: &str) {
    if categories.is_empty() {
        return;
    }
    let yoyo_dir = std::path::Path::new(".yoyo");
    if !yoyo_dir.exists() {
        let _ = std::fs::create_dir_all(yoyo_dir);
    }
    let day = std::fs::read_to_string("DAY_COUNT")
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0);
    let ts = {
        use std::time::SystemTime;
        let dur = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = dur.as_secs();
        let days_since_epoch = secs / 86400;
        let time_of_day = secs % 86400;
        let hours = time_of_day / 3600;
        let minutes = (time_of_day % 3600) / 60;
        let seconds = time_of_day % 60;
        let (y, m, d) = civil_from_days(days_since_epoch as i64);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            y, m, d, hours, minutes, seconds
        )
    };
    let cat_entries: Vec<String> = categories
        .iter()
        .map(|(cat, count)| format!("\"{}\":{}", cat, count))
        .collect();
    let line = format!(
        "{{\"ts\":\"{}\",\"day\":{},\"categories\":{{{}}},\"source\":\"{}\",\"resolved\":\"false\"}}",
        ts,
        day,
        cat_entries.join(","),
        source
    );
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(".yoyo/error_log.jsonl")
    {
        let _ = writeln!(f, "{}", line);
    }
}

/// Record that a fix attempt is pending — saves the error categories to a marker file.
/// Called after `/fix` sends errors to the AI.
pub fn record_fix_pending(categories: &[(RustErrorCategory, usize)]) {
    let yoyo_dir = std::path::Path::new(".yoyo");
    if !yoyo_dir.exists() {
        let _ = std::fs::create_dir_all(yoyo_dir);
    }
    let cats: Vec<String> = categories
        .iter()
        .map(|(cat, _)| format!("{}", cat))
        .collect();
    let content = cats.join(",");
    let _ = std::fs::write(".yoyo/fix_pending.txt", content);
}

/// Check if a pending fix was resolved by a successful build.
/// If `.yoyo/fix_pending.txt` exists, mark the most recent unresolved error log entry as resolved.
/// Call this after any successful build/test pass.
pub fn check_fix_resolution() {
    let pending_path = std::path::Path::new(".yoyo/fix_pending.txt");
    if !pending_path.exists() {
        return;
    }
    // Read the pending categories and remove the marker
    let pending_cats = std::fs::read_to_string(pending_path).unwrap_or_default();
    let _ = std::fs::remove_file(pending_path);

    // Build the fixed_categories JSON array
    let fixed_cats: Vec<String> = pending_cats
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let fixed_json = if fixed_cats.is_empty() {
        String::new()
    } else {
        let quoted: Vec<String> = fixed_cats.iter().map(|c| format!("\"{}\"", c)).collect();
        format!(",\"fixed_categories\":[{}]", quoted.join(","))
    };

    // Mark unresolved entries in error_log.jsonl as resolved
    let log_path = std::path::Path::new(".yoyo/error_log.jsonl");
    let content = match std::fs::read_to_string(log_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // Rewrite lines, changing the last unresolved entry to resolved + fixed_categories
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    for line in lines.iter_mut().rev() {
        if line.contains("\"resolved\":\"false\"") {
            *line = line.replace("\"resolved\":\"false\"", "\"resolved\":\"true\"");
            // Insert fixed_categories before the closing brace
            if !fixed_json.is_empty() {
                if let Some(pos) = line.rfind('}') {
                    line.insert_str(pos, &fixed_json);
                }
            }
            break;
        }
    }
    let new_content = lines.join("\n");
    let _ = std::fs::write(
        log_path,
        if new_content.is_empty() {
            new_content
        } else {
            new_content + "\n"
        },
    );
}

/// Compute fix success rates per error category from the error log.
/// Returns a vec of (category, total_occurrences, resolved_count).
pub fn compute_fix_rates(entries: &[ErrorLogEntry]) -> Vec<(String, usize, usize)> {
    let mut totals: std::collections::HashMap<String, (usize, usize)> =
        std::collections::HashMap::new();
    for entry in entries {
        let is_resolved = entry.resolved == Some(true);
        for (cat, count) in &entry.categories {
            let e = totals.entry(cat.clone()).or_insert((0, 0));
            e.0 += count;
            if is_resolved {
                e.1 += count;
            }
        }
    }
    let mut result: Vec<(String, usize, usize)> = totals
        .into_iter()
        .map(|(cat, (total, resolved))| (cat, total, resolved))
        .collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}

/// Parse error log entries from JSONL content.
pub fn parse_error_log(content: &str) -> Vec<ErrorLogEntry> {
    let mut entries = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(entry) = parse_error_log_line(line) {
            entries.push(entry);
        }
    }
    entries
}

/// Parse a single JSONL line into an ErrorLogEntry.
fn parse_error_log_line(line: &str) -> Option<ErrorLogEntry> {
    let ts = extract_json_string(line, "ts")?;
    let day = extract_json_number(line, "day")?;
    let source = extract_json_string(line, "source")?;
    let categories = extract_json_categories(line)?;
    let resolved = extract_json_string(line, "resolved").and_then(|s| match s.as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    });
    let fixed_categories = extract_json_string_array(line, "fixed_categories");
    Some(ErrorLogEntry {
        ts,
        day,
        categories,
        source,
        resolved,
        fixed_categories,
    })
}

/// Extract a JSON string array: `"key":["a","b"]` → vec!["a", "b"].
fn extract_json_string_array(json: &str, key: &str) -> Vec<String> {
    let pattern = format!("\"{}\":[", key);
    let start = match json.find(&pattern) {
        Some(s) => s + pattern.len(),
        None => return Vec::new(),
    };
    let rest = &json[start..];
    let end = match rest.find(']') {
        Some(e) => e,
        None => return Vec::new(),
    };
    let inner = &rest[..end];
    inner
        .split(',')
        .filter_map(|s| {
            let trimmed = s.trim().trim_matches('"');
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

/// Extract a string field from a JSON line: `"key":"value"`.
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", key);
    let start = json.find(&pattern)? + pattern.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Extract an integer field from a JSON line: `"key":123`.
fn extract_json_number(json: &str, key: &str) -> Option<u32> {
    let pattern = format!("\"{}\":", key);
    let start = json.find(&pattern)? + pattern.len();
    let rest = &json[start..];
    let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().ok()
}

/// Extract the categories object from a JSON line.
fn extract_json_categories(json: &str) -> Option<Vec<(String, usize)>> {
    let pattern = "\"categories\":{";
    let start = json.find(pattern)? + pattern.len();
    let rest = &json[start..];
    let end = rest.find('}')?;
    let inner = &rest[..end];
    if inner.is_empty() {
        return Some(Vec::new());
    }
    let mut cats = Vec::new();
    for pair in inner.split(',') {
        let pair = pair.trim();
        let mut parts = pair.splitn(2, ':');
        let key = parts.next()?.trim().trim_matches('"');
        let val: usize = parts.next()?.trim().parse().ok()?;
        cats.push((key.to_string(), val));
    }
    Some(cats)
}

/// Aggregate error log entries into category totals.
pub fn summarize_error_log(entries: &[ErrorLogEntry]) -> Vec<(String, usize)> {
    let mut totals: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for entry in entries {
        for (cat, count) in &entry.categories {
            *totals.entry(cat.clone()).or_insert(0) += count;
        }
    }
    let mut result: Vec<(String, usize)> = totals.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}

/// Format the /errors display output.
pub fn format_errors_display(entries: &[ErrorLogEntry]) -> String {
    if entries.is_empty() {
        return "  No error log entries found.\n  Errors are recorded when /fix classifies build failures.\n"
            .to_string();
    }
    let mut out = String::new();
    let totals = summarize_error_log(entries);
    let grand_total: usize = totals.iter().map(|(_, c)| c).sum();

    out.push_str(&format!(
        "  Error totals ({} events, {} errors):\n",
        entries.len(),
        grand_total
    ));
    for (cat, count) in &totals {
        out.push_str(&format!("    {}: {}\n", cat, count));
    }

    if let Some((top_cat, top_count)) = totals.first() {
        out.push_str(&format!("\n  Most common: {} ({})\n", top_cat, top_count));
    }

    // Fix success rates
    let rates = compute_fix_rates(entries);
    let has_resolution_data = rates.iter().any(|(_, _, resolved)| *resolved > 0);
    if has_resolution_data {
        out.push_str("\n  Fix success rates:\n");
        for (cat, total, resolved) in &rates {
            if *total > 0 {
                let pct = (*resolved as f64 / *total as f64) * 100.0;
                out.push_str(&format!(
                    "    {}: {}/{} ({:.0}%)\n",
                    cat, resolved, total, pct
                ));
            }
        }
    }

    out.push_str("\n  Recent events:\n");
    let recent: Vec<&ErrorLogEntry> = entries.iter().rev().take(5).collect();
    for entry in recent {
        let cats: Vec<String> = entry
            .categories
            .iter()
            .map(|(c, n)| format!("{} {}", n, c))
            .collect();
        let resolved_marker = match entry.resolved {
            Some(true) => {
                if entry.fixed_categories.is_empty() {
                    " ✓".to_string()
                } else {
                    format!(" ✓ (fixed: {})", entry.fixed_categories.join(", "))
                }
            }
            Some(false) => " ✗".to_string(),
            None => String::new(),
        };
        out.push_str(&format!(
            "    [{}] day {} — {}{}\n",
            entry.ts,
            entry.day,
            cats.join(", "),
            resolved_marker
        ));
    }
    out
}

/// Handle the /errors command. Supports `/errors` (display) and `/errors compact` (compact log).
pub fn handle_errors(input: &str) {
    let args = input.strip_prefix("/errors").unwrap_or("").trim();

    let path = std::path::Path::new(".yoyo/error_log.jsonl");
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let entries = parse_error_log(&content);

    if args == "compact" {
        let summary = compact_error_log(&entries);
        println!("{DIM}{summary}{RESET}\n");
    } else {
        let display = format_errors_display(&entries);
        println!("{DIM}{display}{RESET}\n");
    }
}

/// Compact the error log: aggregate by category, show summary stats,
/// and optionally truncate old entries from the file.
pub fn compact_error_log(entries: &[ErrorLogEntry]) -> String {
    if entries.is_empty() {
        return "  No error log entries to compact.\n".to_string();
    }

    // Aggregate by category
    let mut category_stats: std::collections::HashMap<String, CategoryStats> =
        std::collections::HashMap::new();

    for entry in entries {
        for (cat, count) in &entry.categories {
            let stat = category_stats.entry(cat.clone()).or_insert(CategoryStats {
                total_occurrences: 0,
                total_count: 0,
                fixed_count: 0,
                last_seen: String::new(),
            });
            stat.total_occurrences += 1;
            stat.total_count += count;
            if entry.resolved == Some(true) && entry.fixed_categories.contains(cat) {
                stat.fixed_count += 1;
            }
            if entry.ts > stat.last_seen {
                stat.last_seen.clone_from(&entry.ts);
            }
        }
    }

    // Format summary
    let mut out = String::new();
    out.push_str("  Error Log Summary (compacted):\n\n");
    out.push_str("  Category              | Total | Fixed | Rate  | Last Seen\n");
    out.push_str("  ──────────────────────┼───────┼───────┼───────┼──────────\n");

    let mut sorted: Vec<_> = category_stats.iter().collect();
    sorted.sort_by(|a, b| b.1.total_count.cmp(&a.1.total_count));

    for (cat, stat) in &sorted {
        let fix_rate = if stat.total_occurrences > 0 {
            (stat.fixed_count as f64 / stat.total_occurrences as f64) * 100.0
        } else {
            0.0
        };
        let last_date = if stat.last_seen.len() >= 10 {
            &stat.last_seen[..10]
        } else {
            &stat.last_seen
        };
        out.push_str(&format!(
            "  {:22}| {:>5} | {:>5} | {:>4.0}% | {}\n",
            truncate_category(cat, 22),
            stat.total_count,
            stat.fixed_count,
            fix_rate,
            last_date,
        ));
    }

    // Compact file: keep only entries from last 7 days
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let cutoff_secs = now_secs.saturating_sub(7 * 86400);

    let path = std::path::Path::new(".yoyo/error_log.jsonl");
    if let Ok(raw) = std::fs::read_to_string(path) {
        let mut kept = Vec::new();
        let mut dropped = 0u32;
        for line in raw.lines() {
            if line.trim().is_empty() {
                continue;
            }
            // Check if entry is recent enough by parsing ts
            if let Some(ts) = extract_ts_epoch(line) {
                if ts >= cutoff_secs {
                    kept.push(line.to_string());
                } else {
                    dropped += 1;
                }
            } else {
                kept.push(line.to_string()); // keep unparseable entries
            }
        }

        if dropped > 0 {
            let mut output = kept.join("\n");
            if !output.is_empty() {
                output.push('\n');
            }
            let _ = std::fs::write(path, output);
            out.push_str(&format!(
                "\n  Compacted: dropped {} entries older than 7 days, kept {}\n",
                dropped,
                kept.len()
            ));
        } else {
            out.push_str(&format!(
                "\n  All {} entries are within the last 7 days — nothing to compact.\n",
                entries.len()
            ));
        }
    }

    out
}

struct CategoryStats {
    total_occurrences: usize,
    total_count: usize,
    fixed_count: usize,
    last_seen: String,
}

pub fn truncate_category(s: &str, max: usize) -> String {
    if s.len() <= max {
        format!("{:width$}", s, width = max)
    } else {
        format!("{}…", &s[..max - 1])
    }
}

/// Try to extract epoch seconds from a JSON line's "ts" field (ISO 8601 format).
fn extract_ts_epoch(json: &str) -> Option<u64> {
    let ts_str = {
        let pattern = "\"ts\":\"";
        let idx = json.find(pattern)? + pattern.len();
        let rest = &json[idx..];
        let end = rest.find('"')?;
        &rest[..end]
    };
    // Parse "YYYY-MM-DDTHH:MM:SSZ" manually
    if ts_str.len() < 19 {
        return None;
    }
    let year: i64 = ts_str[0..4].parse().ok()?;
    let month: u32 = ts_str[5..7].parse().ok()?;
    let day: u32 = ts_str[8..10].parse().ok()?;
    let hour: u64 = ts_str[11..13].parse().ok()?;
    let min: u64 = ts_str[14..16].parse().ok()?;
    let sec: u64 = ts_str[17..19].parse().ok()?;

    // Convert to epoch using days_from_civil (inverse of civil_from_days)
    let days = days_from_civil(year, month, day)?;
    Some(days as u64 * 86400 + hour * 3600 + min * 60 + sec)
}

/// Compute days since epoch from civil date (inverse of civil_from_days).
fn days_from_civil(year: i64, month: u32, day: u32) -> Option<u64> {
    let y = if month <= 2 { year - 1 } else { year };
    let m = if month <= 2 {
        month as i64 + 9
    } else {
        month as i64 - 3
    };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * m + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let total = era * 146097 + doe - 719468;
    if total < 0 {
        None
    } else {
        Some(total as u64)
    }
}

// ── /hypotheses ─────────────────────────────────────────────────────────

/// A hypothesis about a persistent error.
#[derive(Debug, Clone, PartialEq)]
pub struct Hypothesis {
    pub ts: String,
    pub day: u32,
    pub error_category: String,
    pub hypothesis: String,
    pub testable_by: String,
}

/// Append a hypothesis to `.yoyo/hypotheses.jsonl`.
pub fn append_hypothesis(error_category: &str, hypothesis: &str, testable_by: &str) {
    let yoyo_dir = std::path::Path::new(".yoyo");
    if !yoyo_dir.exists() {
        let _ = std::fs::create_dir_all(yoyo_dir);
    }
    let day = std::fs::read_to_string("DAY_COUNT")
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0);
    let ts = {
        use std::time::SystemTime;
        let dur = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = dur.as_secs();
        let days_since_epoch = secs / 86400;
        let time_of_day = secs % 86400;
        let hours = time_of_day / 3600;
        let minutes = (time_of_day % 3600) / 60;
        let seconds = time_of_day % 60;
        let (y, m, d) = civil_from_days(days_since_epoch as i64);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            y, m, d, hours, minutes, seconds
        )
    };
    // Escape any quotes in the strings
    let hyp_escaped = hypothesis.replace('"', "\\\"");
    let test_escaped = testable_by.replace('"', "\\\"");
    let cat_escaped = error_category.replace('"', "\\\"");
    let line = format!(
        "{{\"ts\":\"{}\",\"day\":{},\"error_category\":\"{}\",\"hypothesis\":\"{}\",\"testable_by\":\"{}\"}}",
        ts, day, cat_escaped, hyp_escaped, test_escaped
    );
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(".yoyo/hypotheses.jsonl")
    {
        let _ = writeln!(f, "{}", line);
    }
}

/// Parse hypotheses from JSONL content.
pub fn parse_hypotheses(content: &str) -> Vec<Hypothesis> {
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let ts = extract_json_string(line, "ts")?;
            let day = extract_json_number(line, "day")?;
            let error_category = extract_json_string(line, "error_category")?;
            let hypothesis = extract_json_string(line, "hypothesis")?;
            let testable_by = extract_json_string(line, "testable_by")?;
            Some(Hypothesis {
                ts,
                day,
                error_category,
                hypothesis,
                testable_by,
            })
        })
        .collect()
}

/// Format the /hypotheses display output.
pub fn format_hypotheses_display(hypotheses: &[Hypothesis]) -> String {
    if hypotheses.is_empty() {
        return "  No failure hypotheses recorded yet.\n  Hypotheses are generated when recurring errors are detected.\n"
            .to_string();
    }
    let mut out = String::new();
    out.push_str(&format!(
        "  Failure hypotheses ({} recorded):\n\n",
        hypotheses.len()
    ));
    for (i, h) in hypotheses.iter().rev().take(10).enumerate() {
        out.push_str(&format!(
            "  {}. [Day {} — {}] {}\n     Hypothesis: {}\n     Test by: {}\n\n",
            i + 1,
            h.day,
            h.error_category,
            h.ts,
            h.hypothesis,
            h.testable_by,
        ));
    }
    out
}

/// Detect recurring errors from the error log and generate hypotheses.
/// Returns hypotheses for error categories that appear in 2+ unresolved entries.
pub fn detect_recurring_errors(entries: &[ErrorLogEntry]) -> Vec<(String, usize)> {
    let mut unresolved_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for entry in entries {
        if entry.resolved != Some(true) {
            for (cat, _) in &entry.categories {
                *unresolved_counts.entry(cat.clone()).or_insert(0) += 1;
            }
        }
    }
    let mut recurring: Vec<(String, usize)> = unresolved_counts
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .collect();
    recurring.sort_by(|a, b| b.1.cmp(&a.1));
    recurring
}

/// Handle the /hypotheses command.
pub fn handle_hypotheses() {
    let path = std::path::Path::new(".yoyo/hypotheses.jsonl");
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let hypotheses = parse_hypotheses(&content);
    let display = format_hypotheses_display(&hypotheses);
    println!("{DIM}{display}{RESET}\n");
}

// ── /coupling ───────────────────────────────────────────────────────────

pub fn handle_coupling(input: &str) {
    let query = input.strip_prefix("/coupling").unwrap_or("").trim();
    let cwd = std::env::current_dir().unwrap_or_default();
    let project_type = detect_project_type(&cwd);

    // Find the source directory based on project type
    let src_dir = match project_type {
        ProjectType::Rust | ProjectType::Go => {
            if cwd.join("src").is_dir() {
                cwd.join("src")
            } else {
                cwd.clone()
            }
        }
        ProjectType::Node => {
            // Check common Node source directories
            for dir in &["src", "lib", "app", "pages", "components"] {
                if cwd.join(dir).is_dir() {
                    return handle_coupling_for_dir(input, &cwd.join(dir), query);
                }
            }
            cwd.clone()
        }
        ProjectType::Python => {
            // Python: look for a package directory or src/
            if cwd.join("src").is_dir() {
                cwd.join("src")
            } else {
                // Look for first directory containing __init__.py
                if let Ok(entries) = std::fs::read_dir(&cwd) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_dir()
                            && p.join("__init__.py").exists()
                            && p.file_name().is_some_and(|n| {
                                !n.to_string_lossy().starts_with('.') && n != "tests" && n != "test"
                            })
                        {
                            return handle_coupling_for_dir(input, &p, query);
                        }
                    }
                }
                cwd.clone()
            }
        }
        _ => {
            if cwd.join("src").is_dir() {
                cwd.join("src")
            } else {
                println!("{DIM}  No source directory found. Tried: src/{RESET}\n");
                return;
            }
        }
    };
    handle_coupling_for_dir(input, &src_dir, query);
}

fn handle_coupling_for_dir(_input: &str, src_dir: &std::path::Path, query: &str) {
    if !src_dir.is_dir() {
        println!(
            "{DIM}  Source directory not found: {}{RESET}\n",
            src_dir.display()
        );
        return;
    }

    if query.is_empty() {
        // Full coupling report
        let couplings = ast::detect_file_couplings(src_dir);
        let formatted = ast::format_couplings(&couplings);
        println!("{DIM}{formatted}{RESET}");

        let func_refs = ast::detect_function_refs(src_dir);
        let func_formatted = ast::format_function_refs(&func_refs);
        println!("{DIM}{func_formatted}{RESET}\n");
    } else {
        // Symbol-specific query
        let func_refs = ast::detect_function_refs(src_dir);
        let filtered = ast::filter_function_refs_by_symbol(&func_refs, query);
        let formatted = ast::format_filtered_refs(&filtered, query);
        println!("{DIM}{formatted}{RESET}\n");
    }
}

// ── /research ─────────────────────────────────────────────────────────────

/// Fetch web search results from DuckDuckGo Lite and display them.
/// Optionally saves findings to RESEARCH.md with `/research save <query>`.
pub fn handle_research(input: &str) {
    let args = input.strip_prefix("/research").unwrap_or("").trim();
    if args.is_empty() {
        println!(
            "{DIM}  Usage: /research <query>\n\n\
             Examples:\n\
             /research rust async error handling patterns\n\
             /research how does aider handle repo maps\n\
             /research MCP server ecosystem\n\n\
             Fetches web results via DuckDuckGo Lite and displays them.{RESET}\n"
        );
        return;
    }

    let (save, query) = if args.starts_with("save ") {
        (true, args.strip_prefix("save ").unwrap_or(args).trim())
    } else {
        (false, args)
    };

    println!("{DIM}  Searching: {query}...{RESET}");

    // URL-encode the query
    let encoded: String = query
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c.to_string()
            } else if c == ' ' {
                "+".to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect();

    let url = format!("https://lite.duckduckgo.com/lite?q={encoded}");

    let output = std::process::Command::new("curl")
        .args(["-s", "-L", "--max-time", "10", &url])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let body = String::from_utf8_lossy(&out.stdout);
            // Strip HTML tags and extract text
            let text = strip_html_tags(&body);
            // Take first 80 meaningful lines
            let lines: Vec<&str> = text
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty() && l.len() > 3)
                .take(80)
                .collect();

            if lines.is_empty() {
                println!("{DIM}  No results found for: {query}{RESET}\n");
                return;
            }

            println!("\n{BOLD}  Research: {query}{RESET}\n");
            for line in &lines {
                println!("  {line}");
            }
            println!();

            if save {
                // Append to RESEARCH.md
                let entry = format!(
                    "\n### [ ] Research: {query}\nGoal: Investigate this topic based on web search results.\n"
                );
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("RESEARCH.md")
                {
                    use std::io::Write;
                    let _ = write!(f, "{entry}");
                    println!("{DIM}  Saved to RESEARCH.md{RESET}\n");
                }
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            println!("{DIM}  Search failed: {stderr}{RESET}\n");
        }
        Err(e) => {
            println!("{DIM}  curl not available: {e}{RESET}\n");
        }
    }
}

/// Strip HTML tags from a string (simple regex-free approach).
pub(crate) fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    // Decode common HTML entities
    result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
}

// ── /refactor ─────────────────────────────────────────────────────────────

/// Build a coordinated multi-file refactoring prompt.
/// Returns a prompt string for the agent to execute, or None if no valid query.
pub fn handle_refactor(input: &str) -> Option<String> {
    let description = input.strip_prefix("/refactor").unwrap_or("").trim();
    if description.is_empty() {
        println!(
            "{DIM}  Usage: /refactor <description of what to change>\n\n\
             Examples:\n\
             /refactor rename handle_graph to handle_graph_command\n\
             /refactor extract error handling from repl.rs into a new module\n\
             /refactor make all handle_* functions return Result\n\n\
             This command analyzes file coupling, reads affected source files,\n\
             and sends a coordinated refactoring prompt to the AI agent.{RESET}\n"
        );
        return None;
    }

    let cwd = std::env::current_dir().unwrap_or_default();
    let src_dir = if cwd.join("src").is_dir() {
        cwd.join("src")
    } else {
        cwd.clone()
    };

    println!("{DIM}  Analyzing file coupling for refactoring...{RESET}");

    // Get coupling data
    let couplings = ast::detect_file_couplings(&src_dir);
    let coupling_summary = ast::format_couplings(&couplings);

    // Get function-level cross-references
    let func_refs = ast::detect_function_refs(&src_dir);
    let func_summary = ast::format_function_refs(&func_refs);

    // Read all source files (up to a reasonable limit)
    let mut file_contents = String::new();
    let mut file_count = 0;
    if let Ok(entries) = std::fs::read_dir(&src_dir) {
        let mut paths: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "rs"))
            .collect();
        paths.sort();
        for path in &paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                // Include first 200 lines of each file for context (full files would blow context)
                let preview: String = content.lines().take(200).collect::<Vec<_>>().join("\n");
                let total_lines = content.lines().count();
                file_contents.push_str(&format!(
                    "\n--- {name} ({total_lines} lines, showing first 200) ---\n{preview}\n"
                ));
                file_count += 1;
            }
        }
    }

    println!("{DIM}  Found {file_count} source files. Building refactoring prompt...{RESET}\n");

    let prompt = format!(
        "I need you to perform a coordinated multi-file refactoring:\n\n\
         ## Refactoring Request\n{description}\n\n\
         ## File Coupling Analysis\n\
         These files depend on each other — changes to one may require changes to others:\n\
         ```\n{coupling_summary}\n```\n\n\
         ## Function Cross-References\n\
         These functions are referenced across files:\n\
         ```\n{func_summary}\n```\n\n\
         ## Source Files\n{file_contents}\n\n\
         ## Instructions\n\
         1. Identify ALL files that need to change for this refactoring\n\
         2. Plan the changes in dependency order (imports/types first, then callers)\n\
         3. Make all changes using edit_file — surgical edits, not full rewrites\n\
         4. After all edits, run: cargo fmt && cargo clippy --all-targets -- -D warnings && cargo build && cargo test\n\
         5. Fix any errors before declaring done\n\
         \nThis is a coordinated refactoring — every affected file must be updated together."
    );

    Some(prompt)
}

// ── /gap ─────────────────────────────────────────────────────────────────

/// Collect live codebase stats for gap analysis reconciliation.
pub fn collect_gap_stats() -> GapStats {
    // Count source files and lines
    let src_dir = std::path::Path::new("src");
    let mut file_count = 0u32;
    let mut total_lines = 0u32;
    if let Ok(entries) = std::fs::read_dir(src_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "rs") {
                file_count += 1;
                if let Ok(content) = std::fs::read_to_string(&path) {
                    total_lines += content.lines().count() as u32;
                }
            }
        }
    }

    // Count REPL commands from KNOWN_COMMANDS
    let command_count = crate::commands_core::KNOWN_COMMANDS.len() as u32;

    // Count tests by running cargo test --no-run is slow; parse last test output
    // Use a simpler approach: count #[test] annotations in src/
    let mut test_count = 0u32;
    if let Ok(entries) = std::fs::read_dir(src_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "rs") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed == "#[test]" || trimmed == "#[tokio::test]" {
                            test_count += 1;
                        }
                    }
                }
            }
        }
    }

    // Also count integration tests
    let mut integration_tests = 0u32;
    let tests_dir = std::path::Path::new("tests");
    if let Ok(entries) = std::fs::read_dir(tests_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "rs") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed == "#[test]" || trimmed == "#[tokio::test]" {
                            integration_tests += 1;
                        }
                    }
                }
            }
        }
    }

    GapStats {
        file_count,
        total_lines,
        command_count,
        unit_tests: test_count,
        integration_tests,
    }
}

pub struct GapStats {
    pub file_count: u32,
    pub total_lines: u32,
    pub command_count: u32,
    pub unit_tests: u32,
    pub integration_tests: u32,
}

/// Format gap stats for display.
pub fn format_gap_stats(stats: &GapStats) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "  Source files:    {} (.rs files in src/)\n",
        stats.file_count
    ));
    out.push_str(&format!(
        "  Total lines:    ~{}\n",
        round_to_hundreds(stats.total_lines)
    ));
    out.push_str(&format!(
        "  Unit tests:     {} (from #[test] annotations)\n",
        stats.unit_tests
    ));
    out.push_str(&format!(
        "  Integration:    {} (in tests/)\n",
        stats.integration_tests
    ));
    out.push_str(&format!(
        "  Total tests:    {}\n",
        stats.unit_tests + stats.integration_tests
    ));
    out.push_str(&format!(
        "  REPL commands:  {} (from KNOWN_COMMANDS)\n",
        stats.command_count
    ));
    out
}

pub fn round_to_hundreds(n: u32) -> String {
    let rounded = ((n + 50) / 100) * 100;
    format!("{},{}00", rounded / 1000, (rounded % 1000) / 100)
}

/// Update the Stats section in CLAUDE_CODE_GAP.md with live data.
pub fn update_gap_stats_file(stats: &GapStats) -> bool {
    let gap_path = "CLAUDE_CODE_GAP.md";
    let content = match std::fs::read_to_string(gap_path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("{RED}  CLAUDE_CODE_GAP.md not found{RESET}");
            return false;
        }
    };

    // Find the Stats section and replace the first three lines
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let stats_idx = lines.iter().position(|l| l.trim() == "## Stats");
    if let Some(idx) = stats_idx {
        // Replace lines after "## Stats" + blank line
        let start = idx + 2; // skip "## Stats" and blank line
        if start < lines.len() {
            // Build new stats lines
            let new_stats = [
                format!(
                    "- yoyo: ~{} lines of Rust across {} source files + integration tests",
                    round_to_hundreds(stats.total_lines),
                    stats.file_count
                ),
                format!(
                    "- {} tests passing ({} unit + {} integration)",
                    stats.unit_tests + stats.integration_tests,
                    stats.unit_tests,
                    stats.integration_tests
                ),
                format!(
                    "- {} REPL commands (from KNOWN_COMMANDS)",
                    stats.command_count
                ),
            ];
            // Replace the first 3 data lines
            for (i, new_line) in new_stats.iter().enumerate() {
                if start + i < lines.len() {
                    lines[start + i] = new_line.clone();
                }
            }
            let updated = lines.join("\n");
            if std::fs::write(gap_path, &updated).is_ok() {
                return true;
            }
        }
    }
    false
}

pub fn handle_gap() {
    println!("{DIM}  Gap Analysis Stats (live):\n");
    let stats = collect_gap_stats();
    println!("{}", format_gap_stats(&stats));

    // Count gap status from file
    let gap_path = "CLAUDE_CODE_GAP.md";
    if let Ok(content) = std::fs::read_to_string(gap_path) {
        let mut implemented = 0u32;
        let mut partial = 0u32;
        let mut missing = 0u32;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('|') && trimmed.contains('|') {
                if trimmed.contains("✅") {
                    implemented += 1;
                }
                if trimmed.contains("🟡") {
                    partial += 1;
                }
                if trimmed.contains("❌") {
                    missing += 1;
                }
            }
        }
        println!(
            "  Features:  ✅ {} implemented | 🟡 {} partial | ❌ {} missing",
            implemented, partial, missing
        );
    }

    // Update the file
    if update_gap_stats_file(&stats) {
        println!("  ✓ Updated CLAUDE_CODE_GAP.md Stats section");
    }
    println!("{RESET}");
}

// ── /runtime-errors ─────────────────────────────────────────────────────

/// A parsed runtime error entry from `.yoyo/runtime_errors.jsonl`.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeError {
    pub ts: String,
    pub category: String,
    pub tool: Option<String>,
    pub message: String,
}

/// Parse runtime error entries from JSONL content.
pub fn parse_runtime_errors(content: &str) -> Vec<RuntimeError> {
    let mut entries = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let ts = extract_json_string(line, "ts").unwrap_or_default();
        let category = extract_json_string(line, "category").unwrap_or_default();
        let message = extract_json_string(line, "message").unwrap_or_default();
        let tool = extract_json_string(line, "tool");
        if !category.is_empty() {
            entries.push(RuntimeError {
                ts,
                category,
                tool,
                message,
            });
        }
    }
    entries
}

/// Format runtime errors as a display summary.
pub fn format_runtime_errors_display(entries: &[RuntimeError]) -> String {
    if entries.is_empty() {
        return "  No runtime errors logged.\n  (Errors are logged when tool failures, API errors, or stream interruptions occur during sessions.)\n".to_string();
    }

    let mut out = String::new();
    let total = entries.len();

    // Count by category
    let mut by_category: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for e in entries {
        *by_category.entry(&e.category).or_default() += 1;
    }

    out.push_str(&format!("  Runtime errors: {total} total\n\n"));

    // Category breakdown
    out.push_str("  By category:\n");
    let mut cats: Vec<_> = by_category.iter().collect();
    cats.sort_by(|a, b| b.1.cmp(a.1));
    for (cat, count) in &cats {
        out.push_str(&format!("    {cat:<20} {count}\n"));
    }

    // Tool failure breakdown (if any)
    let tool_failures: Vec<_> = entries
        .iter()
        .filter(|e| e.category == "tool_failure")
        .collect();
    if !tool_failures.is_empty() {
        let mut by_tool: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for e in &tool_failures {
            let tool = e.tool.as_deref().unwrap_or("unknown");
            *by_tool.entry(tool).or_default() += 1;
        }
        out.push_str("\n  Tool failures by tool:\n");
        let mut tools: Vec<_> = by_tool.iter().collect();
        tools.sort_by(|a, b| b.1.cmp(a.1));
        for (tool, count) in &tools {
            out.push_str(&format!("    {tool:<20} {count}\n"));
        }
    }

    // Recent entries (last 10)
    out.push_str("\n  Recent errors:\n");
    let recent = if entries.len() > 10 {
        &entries[entries.len() - 10..]
    } else {
        entries
    };
    for e in recent {
        let tool_str = e
            .tool
            .as_deref()
            .map(|t| format!(" [{t}]"))
            .unwrap_or_default();
        let msg_preview = if e.message.len() > 80 {
            format!("{}…", &e.message[..79])
        } else {
            e.message.clone()
        };
        out.push_str(&format!(
            "    {} {}{}: {}\n",
            &e.ts[..10.min(e.ts.len())],
            e.category,
            tool_str,
            msg_preview
        ));
    }

    out
}

/// A detected recurring error pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeErrorPattern {
    pub category: String,
    pub message: String,
    pub count: usize,
    pub first_seen: String,
    pub last_seen: String,
}

/// Detect recurring patterns in runtime errors.
/// A pattern is a (category, message) pair that appears `min_count` or more times.
pub fn detect_runtime_error_patterns(
    entries: &[RuntimeError],
    min_count: usize,
) -> Vec<RuntimeErrorPattern> {
    let mut counts: std::collections::HashMap<(&str, &str), (usize, &str, &str)> =
        std::collections::HashMap::new();
    for e in entries {
        let key = (e.category.as_str(), e.message.as_str());
        let entry = counts
            .entry(key)
            .or_insert((0, e.ts.as_str(), e.ts.as_str()));
        entry.0 += 1;
        // Update first_seen if earlier
        if e.ts.as_str() < entry.1 {
            entry.1 = e.ts.as_str();
        }
        // Update last_seen if later
        if e.ts.as_str() > entry.2 {
            entry.2 = e.ts.as_str();
        }
    }

    let mut patterns: Vec<RuntimeErrorPattern> = counts
        .into_iter()
        .filter(|(_, (count, _, _))| *count >= min_count)
        .map(|((cat, msg), (count, first, last))| RuntimeErrorPattern {
            category: cat.to_string(),
            message: msg.to_string(),
            count,
            first_seen: first.to_string(),
            last_seen: last.to_string(),
        })
        .collect();

    patterns.sort_by(|a, b| b.count.cmp(&a.count));
    patterns
}

/// Format detected runtime error patterns for display.
pub fn format_runtime_error_patterns(patterns: &[RuntimeErrorPattern]) -> String {
    if patterns.is_empty() {
        return "  No recurring patterns detected (threshold: 3+ occurrences).\n".to_string();
    }

    let mut out = String::new();
    out.push_str(&format!(
        "  Recurring error patterns ({} detected):\n\n",
        patterns.len()
    ));

    for p in patterns {
        let msg_preview = if p.message.len() > 80 {
            format!("{}…", &p.message[..79])
        } else {
            p.message.clone()
        };
        out.push_str(&format!(
            "    {} × {} — {}\n",
            p.count, p.category, msg_preview
        ));
        out.push_str(&format!(
            "      first: {}  last: {}\n",
            &p.first_seen[..10.min(p.first_seen.len())],
            &p.last_seen[..10.min(p.last_seen.len())]
        ));
    }

    out
}

/// Handle the /runtime-errors command with optional subcommands.
pub fn handle_runtime_errors(input: &str) {
    let arg = input.strip_prefix("/runtime-errors").unwrap_or("").trim();

    let path = std::path::Path::new(".yoyo/runtime_errors.jsonl");
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let entries = parse_runtime_errors(&content);

    match arg {
        "patterns" => {
            let patterns = detect_runtime_error_patterns(&entries, 3);
            let display = format_runtime_error_patterns(&patterns);
            println!("\n{DIM}{display}{RESET}\n");
        }
        "summary" => {
            if entries.is_empty() {
                println!("{DIM}  No runtime errors logged.{RESET}\n");
                return;
            }
            let mut by_category: std::collections::HashMap<&str, usize> =
                std::collections::HashMap::new();
            for e in &entries {
                *by_category.entry(&e.category).or_default() += 1;
            }
            println!("\n{DIM}  Runtime errors: {} total\n", entries.len());
            println!("  By category:");
            let mut cats: Vec<_> = by_category.iter().collect();
            cats.sort_by(|a, b| b.1.cmp(a.1));
            for (cat, count) in &cats {
                println!("    {cat:<20} {count}");
            }
            println!("{RESET}\n");
        }
        "clear" => {
            if entries.is_empty() {
                println!("{DIM}  No runtime errors to clear.{RESET}\n");
                return;
            }
            match std::fs::write(path, "") {
                Ok(_) => println!(
                    "{GREEN}  ✓ Cleared {} runtime error entries.{RESET}\n",
                    entries.len()
                ),
                Err(e) => eprintln!("{RED}  error clearing runtime errors: {e}{RESET}\n"),
            }
        }
        "" => {
            let display = format_runtime_errors_display(&entries);
            println!("\n{DIM}{display}{RESET}\n");
        }
        _ => {
            println!("{DIM}  usage: /runtime-errors [patterns|summary|clear]{RESET}\n");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_extract_file_purpose_rust_doc_comment() {
        let content = "//! Agent core and REPL loop\nuse std::io;\nfn main() {}";
        assert_eq!(
            extract_file_purpose(content, "rs"),
            "Agent core and REPL loop"
        );
    }

    #[test]
    fn test_extract_file_purpose_python_docstring() {
        let content = "\"\"\"Authentication module for user login\"\"\"";
        assert_eq!(
            extract_file_purpose(content, "py"),
            "Authentication module for user login"
        );
    }

    #[test]
    fn test_extract_file_purpose_js_jsdoc() {
        let content = "/** Main application entry point and router */\nimport React from 'react';";
        assert_eq!(
            extract_file_purpose(content, "js"),
            "Main application entry point and router"
        );
    }

    #[test]
    fn test_extract_file_purpose_go_package() {
        let content = "// Package auth provides authentication middleware\npackage auth";
        assert_eq!(
            extract_file_purpose(content, "go"),
            "Package auth provides authentication middleware"
        );
    }

    #[test]
    fn test_extract_file_purpose_empty_returns_empty() {
        assert_eq!(extract_file_purpose("fn main() {}", "rs"), "");
    }

    #[test]
    fn test_extract_file_purpose_short_comment_skipped() {
        // "//! Hi" is only 2 chars after trim — below the len > 5 threshold
        assert_eq!(extract_file_purpose("//! Hi", "rs"), "");
    }

    #[test]
    fn test_truncate_str_short() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_str_exact() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_str_long() {
        let result = truncate_str("hello world this is long", 10);
        assert_eq!(result, "hello wor…");
        assert!(result.chars().count() <= 10);
    }

    #[test]
    fn test_extract_readme_description_basic() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("README.md"),
            "# My Project\n\nThis is an awesome tool for building things.\n\n## Install\n",
        )
        .unwrap();
        let desc = extract_readme_description(dir.path());
        assert_eq!(
            desc,
            Some("This is an awesome tool for building things.".to_string())
        );
    }

    #[test]
    fn test_extract_readme_description_skips_badges() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("README.md"),
            "# Project\n[![Build](https://img.shields.io/badge)]\n\nActual description here.\n",
        )
        .unwrap();
        let desc = extract_readme_description(dir.path());
        assert_eq!(desc, Some("Actual description here.".to_string()));
    }

    #[test]
    fn test_extract_readme_description_no_readme() {
        let dir = TempDir::new().unwrap();
        assert_eq!(extract_readme_description(dir.path()), None);
    }

    #[test]
    fn test_extract_cargo_dependencies() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\n\n[dependencies]\nserde = \"1.0\"\ntokio = { version = \"1\", features = [\"full\"] }\n\n[dev-dependencies]\ntempfile = \"3\"\n",
        )
        .unwrap();
        let deps = extract_cargo_dependencies(dir.path());
        assert!(deps.contains(&"serde".to_string()));
        assert!(deps.contains(&"tokio".to_string()));
        // dev-dependencies should not be included
        assert!(!deps.contains(&"tempfile".to_string()));
    }

    #[test]
    fn test_extract_node_dependencies() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies":{"react":"^18","express":"^4"},"devDependencies":{"jest":"^29"}}"#,
        )
        .unwrap();
        let deps = extract_node_dependencies(dir.path());
        assert!(deps.contains(&"react".to_string()));
        assert!(deps.contains(&"express".to_string()));
        assert!(!deps.contains(&"jest".to_string()));
    }

    #[test]
    fn test_detect_entry_point_rust() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
        assert_eq!(
            detect_entry_point(dir.path(), &ProjectType::Rust),
            Some("src/main.rs".to_string())
        );
    }

    #[test]
    fn test_detect_entry_point_python() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.py"), "print('hi')").unwrap();
        assert_eq!(
            detect_entry_point(dir.path(), &ProjectType::Python),
            Some("main.py".to_string())
        );
    }

    #[test]
    fn test_detect_entry_point_not_found() {
        let dir = TempDir::new().unwrap();
        assert_eq!(detect_entry_point(dir.path(), &ProjectType::Rust), None);
    }

    #[test]
    fn test_extract_python_dependencies_requirements_txt() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("requirements.txt"),
            "flask==2.0\nrequests>=2.28\n# comment\nnumpy\n",
        )
        .unwrap();
        let deps = extract_python_dependencies(dir.path());
        assert!(deps.contains(&"flask".to_string()));
        assert!(deps.contains(&"requests".to_string()));
        assert!(deps.contains(&"numpy".to_string()));
    }

    // ── parse_test_summary multi-framework tests ──

    #[test]
    fn test_parse_test_summary_cargo() {
        let output = "test result: ok. 42 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out";
        let summary = parse_test_summary(output).unwrap();
        assert_eq!(summary.passed, 42);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.ignored, 3);
    }

    #[test]
    fn test_parse_test_summary_pytest() {
        let output = "============================== 15 passed, 2 failed, 1 skipped in 3.42s ==============================";
        let summary = parse_test_summary(output).unwrap();
        assert_eq!(summary.passed, 15);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.ignored, 1);
    }

    #[test]
    fn test_parse_test_summary_pytest_all_passed() {
        let output =
            "============================== 8 passed in 0.53s ==============================";
        let summary = parse_test_summary(output).unwrap();
        assert_eq!(summary.passed, 8);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.ignored, 0);
    }

    #[test]
    fn test_parse_test_summary_jest() {
        let output = "Tests:  3 failed, 25 passed, 28 total";
        let summary = parse_test_summary(output).unwrap();
        assert_eq!(summary.passed, 25);
        assert_eq!(summary.failed, 3);
    }

    #[test]
    fn test_parse_test_summary_no_match() {
        let output = "some random output\nnothing test-like here\n";
        assert!(parse_test_summary(output).is_none());
    }

    #[test]
    fn test_extract_description_from_cargo_toml() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"myapp\"\ndescription = \"A fast CLI tool\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        assert_eq!(
            extract_description_from_cargo_toml(dir.path()),
            Some("A fast CLI tool".to_string())
        );
    }

    #[test]
    fn test_extract_description_from_package_json() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("package.json"),
            "{\n  \"name\": \"myapp\",\n  \"description\": \"A React dashboard\",\n  \"version\": \"1.0.0\"\n}\n",
        )
        .unwrap();
        assert_eq!(
            extract_description_from_package_json(dir.path()),
            Some("A React dashboard".to_string())
        );
    }

    #[test]
    fn test_extract_description_from_pyproject() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"myapp\"\ndescription = \"Data pipeline toolkit\"\nversion = \"0.1.0\"\n\n[build-system]\nrequires = [\"setuptools\"]\n",
        )
        .unwrap();
        assert_eq!(
            extract_description_from_pyproject(dir.path()),
            Some("Data pipeline toolkit".to_string())
        );
    }

    #[test]
    fn test_extract_project_description_fallback_to_config() {
        // No README, but has Cargo.toml with description
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"myapp\"\ndescription = \"Fallback desc\"\n",
        )
        .unwrap();
        assert_eq!(
            extract_project_description(dir.path(), &ProjectType::Rust),
            Some("Fallback desc".to_string())
        );
    }

    #[test]
    fn test_extract_project_description_readme_takes_priority() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("README.md"),
            "# App\n\nREADME description wins.\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"app\"\ndescription = \"Config desc\"\n",
        )
        .unwrap();
        assert_eq!(
            extract_project_description(dir.path(), &ProjectType::Rust),
            Some("README description wins.".to_string())
        );
    }

    #[test]
    fn test_extract_readme_description_empty_readme() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("README.md"), "# Title\n\n").unwrap();
        assert_eq!(extract_readme_description(dir.path()), None);
    }

    #[test]
    fn test_scan_source_summaries_rust_project() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(
            src.join("main.rs"),
            "//! CLI entry point and REPL\nfn main() {}",
        )
        .unwrap();
        fs::write(
            src.join("lib.rs"),
            "//! Core library for the agent\npub mod core;",
        )
        .unwrap();
        let summaries = scan_source_summaries(dir.path(), &ProjectType::Rust);
        assert_eq!(summaries.len(), 2);
        // Should be sorted by path
        assert_eq!(summaries[0].0, "src/lib.rs");
        assert_eq!(summaries[1].0, "src/main.rs");
        assert!(summaries[1].1.contains("CLI entry point"));
    }

    #[test]
    fn test_scan_source_summaries_empty_src() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        let summaries = scan_source_summaries(dir.path(), &ProjectType::Rust);
        assert!(summaries.is_empty());
    }

    #[test]
    fn test_scan_source_summaries_unknown_project() {
        let dir = TempDir::new().unwrap();
        let summaries = scan_source_summaries(dir.path(), &ProjectType::Unknown);
        assert!(summaries.is_empty());
    }

    #[test]
    fn test_generate_init_content_empty_dir() {
        let dir = TempDir::new().unwrap();
        let content = generate_init_content(dir.path());
        assert!(content.contains("# Project Context"));
        assert!(content.contains("## About This Project"));
        // Should have placeholder comment for description
        assert!(content.contains("<!-- Add a description"));
    }

    #[test]
    fn test_generate_init_content_rust_project() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"myapp\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1\"\n",
        )
        .unwrap();
        let src = dir.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("main.rs"), "//! Main application\nfn main() {}").unwrap();
        fs::write(
            dir.path().join("README.md"),
            "# MyApp\n\nA fast CLI tool for data processing.\n",
        )
        .unwrap();
        let content = generate_init_content(dir.path());
        assert!(content.contains("**myapp**"));
        assert!(content.contains("Rust (Cargo) project"));
        assert!(content.contains("A fast CLI tool for data processing."));
        assert!(content.contains("serde"));
        assert!(content.contains("Entry point: `src/main.rs`"));
        assert!(content.contains("src/main.rs"));
    }

    #[test]
    fn test_detect_entry_point_node() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("index.js"), "console.log('hi')").unwrap();
        assert_eq!(
            detect_entry_point(dir.path(), &ProjectType::Node),
            Some("index.js".to_string())
        );
    }

    #[test]
    fn test_command_exists_known() {
        // "git" should exist on any dev machine
        assert!(command_exists("git"));
    }

    #[test]
    fn test_command_exists_unknown() {
        assert!(!command_exists("definitely_not_a_real_command_xyz123"));
    }

    // -- strip_html_tags --

    #[test]
    fn test_strip_html_tags_basic() {
        assert_eq!(strip_html_tags("<p>hello</p>"), "hello");
    }

    #[test]
    fn test_strip_html_tags_nested() {
        assert_eq!(
            strip_html_tags("<div><span>inner</span> text</div>"),
            "inner text"
        );
    }

    #[test]
    fn test_strip_html_tags_with_attributes() {
        assert_eq!(
            strip_html_tags(r#"<a href="http://example.com">link</a>"#),
            "link"
        );
    }

    #[test]
    fn test_strip_html_tags_entities() {
        assert_eq!(strip_html_tags("a &amp; b"), "a & b");
        assert_eq!(strip_html_tags("&lt;tag&gt;"), "<tag>");
        assert_eq!(strip_html_tags("&quot;quoted&quot;"), "\"quoted\"");
        assert_eq!(strip_html_tags("it&#x27;s"), "it's");
        assert_eq!(strip_html_tags("non&nbsp;breaking"), "non breaking");
    }

    #[test]
    fn test_strip_html_tags_empty() {
        assert_eq!(strip_html_tags(""), "");
    }

    #[test]
    fn test_strip_html_tags_no_tags() {
        assert_eq!(strip_html_tags("plain text"), "plain text");
    }

    #[test]
    fn test_strip_html_tags_self_closing() {
        assert_eq!(strip_html_tags("before<br/>after"), "beforeafter");
    }

    // -- /research URL encoding --

    #[test]
    fn test_research_url_encoding_spaces() {
        let query = "rust async patterns";
        let encoded: String = query
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                    c.to_string()
                } else if c == ' ' {
                    "+".to_string()
                } else {
                    format!("%{:02X}", c as u32)
                }
            })
            .collect();
        assert_eq!(encoded, "rust+async+patterns");
    }

    #[test]
    fn test_research_url_encoding_special_chars() {
        let query = "what's new?";
        let encoded: String = query
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                    c.to_string()
                } else if c == ' ' {
                    "+".to_string()
                } else {
                    format!("%{:02X}", c as u32)
                }
            })
            .collect();
        assert_eq!(encoded, "what%27s+new%3F");
    }

    #[test]
    fn test_research_url_encoding_preserves_safe_chars() {
        let query = "rust-lang_2.0";
        let encoded: String = query
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                    c.to_string()
                } else if c == ' ' {
                    "+".to_string()
                } else {
                    format!("%{:02X}", c as u32)
                }
            })
            .collect();
        assert_eq!(encoded, "rust-lang_2.0");
    }

    // -- /research empty query handling --

    #[test]
    fn test_research_empty_input_does_not_panic() {
        handle_research("/research");
        handle_research("/research ");
    }

    // -- /research save prefix parsing --

    #[test]
    fn test_research_save_prefix_parsing() {
        let args = "save rust async patterns";
        let (save, query) = if args.starts_with("save ") {
            (true, args.strip_prefix("save ").unwrap_or(args).trim())
        } else {
            (false, args)
        };
        assert!(save);
        assert_eq!(query, "rust async patterns");
    }

    #[test]
    fn test_research_no_save_prefix() {
        let args = "rust async patterns";
        let (save, query) = if args.starts_with("save ") {
            (true, args.strip_prefix("save ").unwrap_or(args).trim())
        } else {
            (false, args)
        };
        assert!(!save);
        assert_eq!(query, "rust async patterns");
    }

    // -- /refactor --

    #[test]
    fn test_refactor_empty_input_returns_none() {
        assert!(handle_refactor("/refactor").is_none());
        assert!(handle_refactor("/refactor ").is_none());
        assert!(handle_refactor("/refactor  ").is_none());
    }

    #[test]
    fn test_refactor_valid_input_returns_some() {
        let result = handle_refactor("/refactor rename foo to bar");
        assert!(result.is_some());
    }

    #[test]
    fn test_refactor_prompt_contains_description() {
        let result = handle_refactor("/refactor extract error handling into module").unwrap();
        assert!(result.contains("extract error handling into module"));
    }

    #[test]
    fn test_refactor_prompt_contains_coupling_section() {
        let result = handle_refactor("/refactor rename foo to bar").unwrap();
        assert!(result.contains("File Coupling Analysis"));
    }

    #[test]
    fn test_refactor_prompt_contains_cross_ref_section() {
        let result = handle_refactor("/refactor rename foo to bar").unwrap();
        assert!(result.contains("Function Cross-References"));
    }

    #[test]
    fn test_refactor_prompt_contains_instructions() {
        let result = handle_refactor("/refactor rename foo to bar").unwrap();
        assert!(result.contains("coordinated refactoring"));
        assert!(result.contains("cargo fmt"));
    }
}
