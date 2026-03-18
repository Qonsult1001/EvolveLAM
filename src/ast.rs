//! AST-aware symbol search for yoyo.
//!
//! Uses regex patterns to extract structural symbols (functions, structs, enums,
//! traits, impls, mods) from source files. Language-aware: supports Rust, Python,
//! TypeScript/JavaScript, Go, and C/C++.
//!
//! Usage: `/ast <pattern>` searches for symbols matching the pattern.

use std::path::Path;

/// A single symbol found in source code.
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub kind: SymbolKind,
    pub name: String,
    pub file: String,
    pub line: usize,
    pub signature: String,
}

/// The kind of symbol found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Module,
    Class,
    Interface,
    Type,
    Const,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolKind::Function => write!(f, "fn"),
            SymbolKind::Struct => write!(f, "struct"),
            SymbolKind::Enum => write!(f, "enum"),
            SymbolKind::Trait => write!(f, "trait"),
            SymbolKind::Impl => write!(f, "impl"),
            SymbolKind::Module => write!(f, "mod"),
            SymbolKind::Class => write!(f, "class"),
            SymbolKind::Interface => write!(f, "interface"),
            SymbolKind::Type => write!(f, "type"),
            SymbolKind::Const => write!(f, "const"),
        }
    }
}

/// Language-specific symbol extraction patterns.
struct LangPatterns {
    extensions: &'static [&'static str],
    patterns: &'static [(&'static str, SymbolKind)],
}

/// Rust patterns.
const RUST_PATTERNS: LangPatterns = LangPatterns {
    extensions: &["rs"],
    patterns: &[
        (
            r"^\s*(?:pub(?:\([\w:]+\))?\s+)?(?:async\s+)?fn\s+(\w+)",
            SymbolKind::Function,
        ),
        (
            r"^\s*(?:pub(?:\([\w:]+\))?\s+)?struct\s+(\w+)",
            SymbolKind::Struct,
        ),
        (
            r"^\s*(?:pub(?:\([\w:]+\))?\s+)?enum\s+(\w+)",
            SymbolKind::Enum,
        ),
        (
            r"^\s*(?:pub(?:\([\w:]+\))?\s+)?trait\s+(\w+)",
            SymbolKind::Trait,
        ),
        (r"^\s*impl(?:<[^>]*>)?\s+(\w+)", SymbolKind::Impl),
        (
            r"^\s*(?:pub(?:\([\w:]+\))?\s+)?mod\s+(\w+)",
            SymbolKind::Module,
        ),
        (
            r"^\s*(?:pub(?:\([\w:]+\))?\s+)?type\s+(\w+)",
            SymbolKind::Type,
        ),
        (
            r"^\s*(?:pub(?:\([\w:]+\))?\s+)?const\s+(\w+)",
            SymbolKind::Const,
        ),
    ],
};

/// Python patterns.
const PYTHON_PATTERNS: LangPatterns = LangPatterns {
    extensions: &["py"],
    patterns: &[
        (r"^\s*(?:async\s+)?def\s+(\w+)", SymbolKind::Function),
        (r"^\s*class\s+(\w+)", SymbolKind::Class),
    ],
};

/// TypeScript/JavaScript patterns.
const TS_JS_PATTERNS: LangPatterns = LangPatterns {
    extensions: &["ts", "tsx", "js", "jsx"],
    patterns: &[
        (
            r"^\s*(?:export\s+)?(?:async\s+)?function\s+(\w+)",
            SymbolKind::Function,
        ),
        (r"^\s*(?:export\s+)?class\s+(\w+)", SymbolKind::Class),
        (
            r"^\s*(?:export\s+)?interface\s+(\w+)",
            SymbolKind::Interface,
        ),
        (r"^\s*(?:export\s+)?type\s+(\w+)", SymbolKind::Type),
        (
            r"^\s*(?:export\s+)?(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?(?:\([^)]*\)|[^=])\s*=>",
            SymbolKind::Function,
        ),
    ],
};

/// Go patterns.
const GO_PATTERNS: LangPatterns = LangPatterns {
    extensions: &["go"],
    patterns: &[
        (r"^func\s+(?:\([^)]+\)\s+)?(\w+)", SymbolKind::Function),
        (r"^type\s+(\w+)\s+struct", SymbolKind::Struct),
        (r"^type\s+(\w+)\s+interface", SymbolKind::Interface),
        (r"^type\s+(\w+)", SymbolKind::Type),
    ],
};

/// C/C++ patterns.
const C_CPP_PATTERNS: LangPatterns = LangPatterns {
    extensions: &["c", "h", "cpp", "hpp", "cc", "cxx"],
    patterns: &[
        (
            r"^\s*(?:static\s+|inline\s+|virtual\s+|extern\s+)*(?:[\w:*&<>]+\s+)+(\w+)\s*\(",
            SymbolKind::Function,
        ),
        (r"^\s*(?:typedef\s+)?struct\s+(\w+)", SymbolKind::Struct),
        (r"^\s*(?:typedef\s+)?enum\s+(\w+)", SymbolKind::Enum),
        (r"^\s*class\s+(\w+)", SymbolKind::Class),
    ],
};

const ALL_LANGS: &[&LangPatterns] = &[
    &RUST_PATTERNS,
    &PYTHON_PATTERNS,
    &TS_JS_PATTERNS,
    &GO_PATTERNS,
    &C_CPP_PATTERNS,
];

/// Detect language patterns for a given file extension.
fn patterns_for_ext(ext: &str) -> Option<&'static LangPatterns> {
    let ext_lower = ext.to_lowercase();
    ALL_LANGS
        .iter()
        .copied()
        .find(|lang| lang.extensions.iter().any(|e| *e == ext_lower))
}

/// Extract symbols from a single file's content.
pub fn extract_symbols(file_path: &str, content: &str) -> Vec<Symbol> {
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let lang = match patterns_for_ext(ext) {
        Some(l) => l,
        None => return Vec::new(),
    };

    let mut symbols = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        for (pattern_str, kind) in lang.patterns {
            // Simple regex matching without the regex crate:
            // We use a hand-rolled matcher for the common patterns.
            if let Some(name) = match_pattern(line, pattern_str, *kind) {
                let sig = line.trim().to_string();
                // Truncate signature for display
                let sig = if sig.len() > 120 {
                    format!("{}...", &sig[..117])
                } else {
                    sig
                };
                symbols.push(Symbol {
                    kind: *kind,
                    name,
                    file: file_path.to_string(),
                    line: line_num + 1,
                    signature: sig,
                });
                break; // Only first match per line
            }
        }
    }

    symbols
}

/// Simple pattern matching that extracts the first capture group.
/// Handles the most common regex patterns used in our lang definitions
/// without requiring the regex crate.
fn match_pattern(line: &str, _pattern: &str, kind: SymbolKind) -> Option<String> {
    let trimmed = line.trim_start();

    match kind {
        SymbolKind::Function => match_function(trimmed),
        SymbolKind::Struct => match_keyword_name(trimmed, "struct"),
        SymbolKind::Enum => match_keyword_name(trimmed, "enum"),
        SymbolKind::Trait => match_keyword_name(trimmed, "trait"),
        SymbolKind::Impl => match_impl(trimmed),
        SymbolKind::Module => match_keyword_name(trimmed, "mod"),
        SymbolKind::Class => match_keyword_name(trimmed, "class"),
        SymbolKind::Interface => match_keyword_name(trimmed, "interface"),
        SymbolKind::Type => match_keyword_name(trimmed, "type"),
        SymbolKind::Const => match_keyword_name(trimmed, "const"),
    }
}

/// Match function declarations across languages.
fn match_function(s: &str) -> Option<String> {
    // Rust: pub fn foo, pub async fn foo, fn foo, async fn foo
    // Python: def foo, async def foo
    // JS/TS: function foo, async function foo, export function foo
    // Go: func foo, func (r Receiver) foo

    let s = strip_prefix_words(
        s,
        &[
            "pub", "export", "static", "inline", "virtual", "extern", "async",
        ],
    );
    let s = strip_visibility(&s); // strip pub(crate) etc.
    let s = strip_prefix_words(&s, &["async"]); // might appear after pub

    if let Some(rest) = s
        .strip_prefix("fn ")
        .or_else(|| s.strip_prefix("def "))
        .or_else(|| s.strip_prefix("function "))
    {
        return extract_identifier(rest.trim_start());
    }

    // Go: func (receiver) name or func name
    if let Some(rest) = s.strip_prefix("func ") {
        let rest = rest.trim_start();
        // Skip receiver: func (r *Foo) Bar(...)
        if rest.starts_with('(') {
            if let Some(close) = rest.find(')') {
                let after = rest[close + 1..].trim_start();
                return extract_identifier(after);
            }
        }
        return extract_identifier(rest);
    }

    // Arrow functions: const foo = (...) => or const foo = async (...) =>
    for kw in &["const ", "let ", "var "] {
        if let Some(rest) = s.strip_prefix(kw) {
            let rest = rest.trim_start();
            if let Some(name) = extract_identifier(rest) {
                // Check if remainder has => pattern
                let after_name = &rest[name.len()..].trim_start();
                if let Some(rest) = after_name.strip_prefix('=') {
                    let rhs = rest.trim_start();
                    if rhs.contains("=>") {
                        return Some(name);
                    }
                }
            }
        }
    }

    None
}

/// Match `impl Foo` or `impl<T> Foo`
fn match_impl(s: &str) -> Option<String> {
    let rest = s.strip_prefix("impl")?;
    let rest = if rest.starts_with('<') {
        // Skip generic parameters
        let mut depth = 0;
        let mut end = 0;
        for (i, ch) in rest.char_indices() {
            match ch {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        end = i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        &rest[end..]
    } else {
        rest
    };
    let rest = rest.trim_start();
    extract_identifier(rest)
}

/// Match `keyword Name` with optional prefixes like pub, export, typedef.
fn match_keyword_name(s: &str, keyword: &str) -> Option<String> {
    let s = strip_prefix_words(s, &["pub", "export", "typedef"]);
    let s = strip_visibility(&s);
    let kw_space = format!("{keyword} ");
    let rest = s.strip_prefix(&kw_space)?;
    extract_identifier(rest.trim_start())
}

/// Extract a valid identifier from the start of a string.
fn extract_identifier(s: &str) -> Option<String> {
    if s.is_empty() {
        return None;
    }
    let first = s.chars().next()?;
    if !first.is_alphabetic() && first != '_' {
        return None;
    }
    let name: String = s
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Strip known prefix keywords (e.g., "pub", "export") from the start of a line.
fn strip_prefix_words(s: &str, words: &[&str]) -> String {
    let mut result = s.to_string();
    loop {
        let mut changed = false;
        for word in words {
            let prefix = format!("{word} ");
            if let Some(rest) = result.strip_prefix(&prefix) {
                result = rest.trim_start().to_string();
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    result
}

/// Strip Rust-style visibility modifiers like `pub(crate)`, `pub(super)`.
fn strip_visibility(s: &str) -> String {
    if s.starts_with("pub(") {
        if let Some(close) = s.find(')') {
            return s[close + 1..].trim_start().to_string();
        }
    }
    s.to_string()
}

/// Search project files for symbols matching a pattern.
/// Returns symbols sorted by relevance (exact match > prefix > contains).
pub fn search_symbols(pattern: &str) -> Vec<Symbol> {
    let files = list_searchable_files();
    let pattern_lower = pattern.to_lowercase();
    let mut all_symbols = Vec::new();

    for file in &files {
        let content = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let symbols = extract_symbols(file, &content);
        for sym in symbols {
            let name_lower = sym.name.to_lowercase();
            if name_lower.contains(&pattern_lower) {
                all_symbols.push(sym);
            }
        }
    }

    // Sort: exact match first, then prefix match, then contains
    all_symbols.sort_by(|a, b| {
        let a_lower = a.name.to_lowercase();
        let b_lower = b.name.to_lowercase();
        let a_exact = a_lower == pattern_lower;
        let b_exact = b_lower == pattern_lower;
        let a_prefix = a_lower.starts_with(&pattern_lower);
        let b_prefix = b_lower.starts_with(&pattern_lower);

        match (a_exact, b_exact) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => match (a_prefix, b_prefix) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file.cmp(&b.file).then(a.line.cmp(&b.line)),
            },
        }
    });

    all_symbols
}

/// List files eligible for AST search using git ls-files, falling back to directory walk.
fn list_searchable_files() -> Vec<String> {
    // Try git ls-files first
    if let Ok(output) = std::process::Command::new("git")
        .args(["ls-files", "--cached", "--others", "--exclude-standard"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout
                .lines()
                .filter(|l| !l.is_empty())
                .filter(|l| {
                    Path::new(l)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| patterns_for_ext(e).is_some())
                        .unwrap_or(false)
                })
                .map(|l| l.to_string())
                .collect();
        }
    }

    // Fallback: walk directory (max depth 6)
    let mut files = Vec::new();
    walk_for_sources(Path::new("."), 0, 6, &mut files);
    files
}

fn walk_for_sources(dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<String>) {
    if depth > max_depth {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "node_modules" || name == "target" || name == "vendor" {
            continue;
        }
        if path.is_dir() {
            walk_for_sources(&path, depth + 1, max_depth, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| patterns_for_ext(e).is_some())
            .unwrap_or(false)
        {
            out.push(path.to_string_lossy().to_string());
        }
    }
}

/// Format search results for display.
pub fn format_symbols(symbols: &[Symbol], max_results: usize) -> String {
    if symbols.is_empty() {
        return "  No symbols found.".to_string();
    }

    let shown = symbols.len().min(max_results);
    let mut lines = Vec::new();

    for sym in &symbols[..shown] {
        lines.push(format!(
            "  {:<10} {:<30} {}:{}",
            sym.kind, sym.name, sym.file, sym.line
        ));
        lines.push(format!("             {}", sym.signature));
    }

    if symbols.len() > max_results {
        lines.push(format!(
            "\n  ... and {} more (showing top {max_results})",
            symbols.len() - max_results
        ));
    }

    lines.join("\n")
}

// ============================================================================
// File Coupling Detection
// ============================================================================

/// A coupling edge: file A depends on module B.
#[derive(Debug, Clone, PartialEq)]
pub struct FileCoupling {
    /// The file that contains the `use` statement.
    pub from_file: String,
    /// The module being imported (e.g., "cli", "format", "memory").
    pub to_module: String,
}

/// Parse `use crate::module` statements from Rust source to find which modules a file imports.
/// Returns module names (not full paths) — e.g., `use crate::cli::*` yields "cli".
pub fn parse_rust_imports(content: &str) -> Vec<String> {
    let mut modules = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        // Match: use crate::module_name (with optional ::sub or ;)
        if let Some(rest) = trimmed.strip_prefix("use crate::") {
            // Extract the first path segment (the module name)
            let module: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !module.is_empty() && !modules.contains(&module) {
                modules.push(module);
            }
        }
    }
    modules
}

/// Scan Rust source files under `src/` and build a coupling map.
/// Returns a list of coupling edges (file → module it depends on).
pub fn detect_file_couplings(src_dir: &Path) -> Vec<FileCoupling> {
    let mut couplings = Vec::new();
    let entries = match std::fs::read_dir(src_dir) {
        Ok(e) => e,
        Err(_) => return couplings,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for module in parse_rust_imports(&content) {
            couplings.push(FileCoupling {
                from_file: filename.clone(),
                to_module: module,
            });
        }
    }
    couplings.sort_by(|a, b| {
        a.from_file
            .cmp(&b.from_file)
            .then(a.to_module.cmp(&b.to_module))
    });
    couplings
}

/// Format coupling data for display: group by file, show dependency count.
pub fn format_couplings(couplings: &[FileCoupling]) -> String {
    if couplings.is_empty() {
        return "  No file couplings detected.".to_string();
    }

    // Group by from_file
    let mut by_file: std::collections::BTreeMap<&str, Vec<&str>> =
        std::collections::BTreeMap::new();
    for c in couplings {
        by_file.entry(&c.from_file).or_default().push(&c.to_module);
    }

    // Also count how many files depend on each module (reverse coupling)
    let mut dependents: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for c in couplings {
        *dependents.entry(&c.to_module).or_insert(0) += 1;
    }

    let mut lines = Vec::new();
    lines.push("  File couplings (use crate:: imports):\n".to_string());
    for (file, modules) in &by_file {
        lines.push(format!("  {file} → [{}]", modules.join(", ")));
    }
    lines.push(String::new());
    lines.push("  Most depended-on modules:".to_string());
    let mut dep_list: Vec<(&&str, &usize)> = dependents.iter().collect();
    dep_list.sort_by(|a, b| b.1.cmp(a.1));
    for (module, count) in dep_list.iter().take(10) {
        lines.push(format!("    {module}: {count} dependents"));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_rust_functions() {
        let code = r#"
pub fn hello_world() {
}

fn private_fn() {}

pub async fn async_handler(req: Request) -> Response {
}

pub(crate) fn crate_fn() {}
"#;
        let symbols = extract_symbols("test.rs", code);
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"hello_world"));
        assert!(names.contains(&"private_fn"));
        assert!(names.contains(&"async_handler"));
        assert!(names.contains(&"crate_fn"));
    }

    #[test]
    fn test_extract_rust_structs_enums() {
        let code = r#"
pub struct Foo {
    bar: i32,
}

enum Color {
    Red,
    Green,
    Blue,
}

pub(crate) struct Internal;
"#;
        let symbols = extract_symbols("test.rs", code);
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Foo"));
        assert!(names.contains(&"Color"));
        assert!(names.contains(&"Internal"));
    }

    #[test]
    fn test_extract_rust_traits_impls() {
        let code = r#"
pub trait Display {
    fn fmt(&self) -> String;
}

impl Display for Foo {
    fn fmt(&self) -> String { }
}

impl<T: Clone> MyTrait for Vec<T> {
}
"#;
        let symbols = extract_symbols("test.rs", code);
        let kinds: Vec<(&str, SymbolKind)> =
            symbols.iter().map(|s| (s.name.as_str(), s.kind)).collect();
        assert!(kinds.contains(&("Display", SymbolKind::Trait)));
        assert!(kinds
            .iter()
            .any(|(n, k)| *n == "Display" && *k == SymbolKind::Impl));
        assert!(kinds
            .iter()
            .any(|(n, k)| *n == "fmt" && *k == SymbolKind::Function));
    }

    #[test]
    fn test_extract_python_symbols() {
        let code = r#"
def hello():
    pass

class MyClass:
    def method(self):
        pass

async def async_func():
    pass
"#;
        let symbols = extract_symbols("test.py", code);
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"hello"));
        assert!(names.contains(&"MyClass"));
        assert!(names.contains(&"method"));
        assert!(names.contains(&"async_func"));
    }

    #[test]
    fn test_extract_typescript_symbols() {
        let code = r#"
export function greet(name: string): string {}

class UserService {
}

interface ApiResponse {
}

export type UserId = string;
"#;
        let symbols = extract_symbols("test.ts", code);
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"greet"));
        assert!(names.contains(&"UserService"));
        assert!(names.contains(&"ApiResponse"));
        assert!(names.contains(&"UserId"));
    }

    #[test]
    fn test_extract_go_symbols() {
        let code = r#"
func Hello() {
}

func (s *Server) Start() error {
}

type Config struct {
    Port int
}

type Handler interface {
    Handle() error
}
"#;
        let symbols = extract_symbols("test.go", code);
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Hello"));
        assert!(names.contains(&"Start"));
        assert!(names.contains(&"Config"));
        assert!(names.contains(&"Handler"));
    }

    #[test]
    fn test_unknown_extension_returns_empty() {
        let symbols = extract_symbols("test.xyz", "whatever content");
        assert!(symbols.is_empty());
    }

    #[test]
    fn test_symbol_sorting() {
        let mut symbols = [
            Symbol {
                kind: SymbolKind::Function,
                name: "handle_foo".to_string(),
                file: "b.rs".to_string(),
                line: 10,
                signature: "fn handle_foo()".to_string(),
            },
            Symbol {
                kind: SymbolKind::Function,
                name: "foo".to_string(),
                file: "a.rs".to_string(),
                line: 5,
                signature: "fn foo()".to_string(),
            },
            Symbol {
                kind: SymbolKind::Struct,
                name: "FooBar".to_string(),
                file: "c.rs".to_string(),
                line: 1,
                signature: "struct FooBar".to_string(),
            },
        ];

        // Simulate the sorting from search_symbols
        let pattern_lower = "foo";
        symbols.sort_by(|a, b| {
            let a_lower = a.name.to_lowercase();
            let b_lower = b.name.to_lowercase();
            let a_exact = a_lower == pattern_lower;
            let b_exact = b_lower == pattern_lower;
            let a_prefix = a_lower.starts_with(pattern_lower);
            let b_prefix = b_lower.starts_with(pattern_lower);

            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => match (a_prefix, b_prefix) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.file.cmp(&b.file).then(a.line.cmp(&b.line)),
                },
            }
        });

        assert_eq!(symbols[0].name, "foo"); // exact match first
        assert_eq!(symbols[1].name, "FooBar"); // prefix match second
    }

    #[test]
    fn test_format_symbols_empty() {
        let result = format_symbols(&[], 20);
        assert!(result.contains("No symbols found"));
    }

    #[test]
    fn test_format_symbols_truncates() {
        let symbols: Vec<Symbol> = (0..5)
            .map(|i| Symbol {
                kind: SymbolKind::Function,
                name: format!("fn_{i}"),
                file: "test.rs".to_string(),
                line: i + 1,
                signature: format!("fn fn_{i}()"),
            })
            .collect();
        let result = format_symbols(&symbols, 3);
        assert!(result.contains("fn_0"));
        assert!(result.contains("fn_2"));
        assert!(result.contains("and 2 more"));
    }

    #[test]
    fn test_extract_identifier() {
        assert_eq!(extract_identifier("foo_bar("), Some("foo_bar".to_string()));
        assert_eq!(extract_identifier("_private"), Some("_private".to_string()));
        assert_eq!(extract_identifier("123bad"), None);
        assert_eq!(extract_identifier(""), None);
    }

    #[test]
    fn test_strip_visibility() {
        assert_eq!(strip_visibility("pub(crate) fn foo"), "fn foo");
        assert_eq!(strip_visibility("pub(super) struct Bar"), "struct Bar");
        assert_eq!(strip_visibility("fn foo"), "fn foo");
    }

    #[test]
    fn test_match_impl_generics() {
        let result = match_impl("impl<T: Clone> MyTrait for Vec<T> {");
        assert_eq!(result, Some("MyTrait".to_string()));
    }

    #[test]
    fn test_arrow_function_detection() {
        let result = match_function("const handler = (req) => {");
        assert_eq!(result, Some("handler".to_string()));

        let result = match_function("export const greet = async () => {");
        assert_eq!(result, Some("greet".to_string()));
    }

    // ── File coupling tests ──────────────────────────────────────────────

    #[test]
    fn test_parse_rust_imports_basic() {
        let code = r#"
use crate::cli::*;
use crate::format::*;
use crate::prompt::*;
use std::io;
"#;
        let imports = parse_rust_imports(code);
        assert_eq!(imports, vec!["cli", "format", "prompt"]);
    }

    #[test]
    fn test_parse_rust_imports_nested() {
        let code = "use crate::memory::ConnectionGraph;\nuse crate::memory::ConnectionKind;\n";
        let imports = parse_rust_imports(code);
        // Deduplicates: memory appears only once
        assert_eq!(imports, vec!["memory"]);
    }

    #[test]
    fn test_parse_rust_imports_empty() {
        let code = "use std::collections::HashMap;\nfn main() {}\n";
        let imports = parse_rust_imports(code);
        assert!(imports.is_empty());
    }

    #[test]
    fn test_parse_rust_imports_braced() {
        let code = "use crate::cli::{is_verbose, AUTO_COMPACT_THRESHOLD};\n";
        let imports = parse_rust_imports(code);
        assert_eq!(imports, vec!["cli"]);
    }

    #[test]
    fn test_format_couplings_empty() {
        let result = format_couplings(&[]);
        assert!(result.contains("No file couplings"));
    }

    #[test]
    fn test_format_couplings_basic() {
        let couplings = vec![
            FileCoupling {
                from_file: "main.rs".to_string(),
                to_module: "cli".to_string(),
            },
            FileCoupling {
                from_file: "main.rs".to_string(),
                to_module: "format".to_string(),
            },
            FileCoupling {
                from_file: "repl.rs".to_string(),
                to_module: "cli".to_string(),
            },
        ];
        let result = format_couplings(&couplings);
        assert!(result.contains("main.rs"));
        assert!(result.contains("cli"));
        assert!(result.contains("Most depended-on"));
        // cli has 2 dependents
        assert!(result.contains("cli: 2 dependents"));
    }
}
