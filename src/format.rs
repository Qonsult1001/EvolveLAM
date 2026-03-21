//! Formatting helpers: ANSI colors, cost, duration, tokens, context bar, truncation.

use std::sync::OnceLock;

// --- Color support with NO_COLOR and --no-color ---

/// Whether color output has been disabled (via NO_COLOR env or --no-color flag).
static COLOR_DISABLED: OnceLock<bool> = OnceLock::new();

/// Disable color output. Call before any formatting happens (e.g., from CLI arg parsing).
pub fn disable_color() {
    let _ = COLOR_DISABLED.set(true);
}

/// Check if color output is enabled. Cached after first call.
/// Respects the NO_COLOR environment variable (https://no-color.org/).
fn color_enabled() -> bool {
    !*COLOR_DISABLED.get_or_init(|| std::env::var("NO_COLOR").is_ok())
}

/// A color code that respects the NO_COLOR convention.
/// When color is disabled, formats as an empty string.
pub struct Color(pub &'static str);

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if color_enabled() {
            f.write_str(self.0)
        } else {
            Ok(())
        }
    }
}

// ANSI color helpers — respect NO_COLOR env var and --no-color flag
pub static RESET: Color = Color("\x1b[0m");
pub static BOLD: Color = Color("\x1b[1m");
pub static DIM: Color = Color("\x1b[2m");
pub static GREEN: Color = Color("\x1b[32m");
pub static YELLOW: Color = Color("\x1b[33m");
pub static CYAN: Color = Color("\x1b[36m");
pub static RED: Color = Color("\x1b[31m");
pub static MAGENTA: Color = Color("\x1b[35m");
pub static BOLD_CYAN: Color = Color("\x1b[1;36m");
pub static BOLD_YELLOW: Color = Color("\x1b[1;33m");

// --- Syntax highlighting for code blocks ---

/// Languages recognized for syntax highlighting.
fn normalize_lang(lang: &str) -> Option<&'static str> {
    match lang.to_lowercase().as_str() {
        "rust" | "rs" => Some("rust"),
        "python" | "py" => Some("python"),
        "javascript" | "js" | "typescript" | "ts" | "jsx" | "tsx" => Some("js"),
        "go" | "golang" => Some("go"),
        "sh" | "bash" | "shell" | "zsh" => Some("shell"),
        "c" | "cpp" | "c++" | "cc" | "cxx" | "h" | "hpp" => Some("c"),
        "json" | "jsonc" => Some("json"),
        "yaml" | "yml" => Some("yaml"),
        "toml" => Some("toml"),
        _ => None,
    }
}

/// Get the keyword list for a normalized language.
fn lang_keywords(lang: &str) -> &'static [&'static str] {
    match lang {
        "rust" => &[
            "fn",
            "let",
            "mut",
            "if",
            "else",
            "for",
            "while",
            "loop",
            "match",
            "return",
            "use",
            "mod",
            "pub",
            "struct",
            "enum",
            "impl",
            "trait",
            "where",
            "async",
            "await",
            "move",
            "self",
            "super",
            "crate",
            "const",
            "static",
            "type",
            "as",
            "in",
            "ref",
            "true",
            "false",
            "Some",
            "None",
            "Ok",
            "Err",
            "unsafe",
            "dyn",
            "macro_rules",
        ],
        "python" => &[
            "def", "class", "if", "elif", "else", "for", "while", "return", "import", "from", "as",
            "with", "try", "except", "finally", "raise", "yield", "lambda", "pass", "break",
            "continue", "and", "or", "not", "in", "is", "None", "True", "False", "self", "async",
            "await", "del", "global", "nonlocal", "assert",
        ],
        "js" => &[
            "function",
            "const",
            "let",
            "var",
            "if",
            "else",
            "for",
            "while",
            "return",
            "import",
            "export",
            "from",
            "class",
            "new",
            "this",
            "async",
            "await",
            "try",
            "catch",
            "finally",
            "throw",
            "typeof",
            "instanceof",
            "true",
            "false",
            "null",
            "undefined",
            "switch",
            "case",
            "default",
            "break",
            "continue",
            "interface",
            "type",
            "enum",
            "of",
            "in",
            "yield",
            "delete",
            "void",
            "super",
            "extends",
            "implements",
            "static",
            "get",
            "set",
        ],
        "go" => &[
            "func",
            "var",
            "const",
            "if",
            "else",
            "for",
            "range",
            "return",
            "import",
            "package",
            "type",
            "struct",
            "interface",
            "map",
            "chan",
            "go",
            "defer",
            "select",
            "case",
            "switch",
            "default",
            "break",
            "continue",
            "nil",
            "true",
            "false",
            "fallthrough",
            "goto",
        ],
        "shell" => &[
            "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac",
            "function", "return", "exit", "echo", "export", "local", "readonly", "set", "unset",
            "in", "true", "false", "source", "alias", "cd", "test",
        ],
        "c" => &[
            "if",
            "else",
            "for",
            "while",
            "do",
            "switch",
            "case",
            "default",
            "break",
            "continue",
            "return",
            "goto",
            "struct",
            "union",
            "enum",
            "typedef",
            "sizeof",
            "static",
            "extern",
            "const",
            "volatile",
            "inline",
            "void",
            "int",
            "char",
            "float",
            "double",
            "long",
            "short",
            "unsigned",
            "signed",
            "auto",
            "register",
            "class",
            "public",
            "private",
            "protected",
            "virtual",
            "template",
            "namespace",
            "using",
            "new",
            "delete",
            "try",
            "catch",
            "throw",
            "nullptr",
            "true",
            "false",
            "bool",
            "include",
            "define",
            "ifdef",
            "ifndef",
            "endif",
            "pragma",
        ],
        "toml" | "yaml" => &["true", "false", "null", "yes", "no", "on", "off"],
        _ => &[],
    }
}

/// Get built-in type names for a normalized language (highlighted in magenta).
fn lang_types(lang: &str) -> &'static [&'static str] {
    match lang {
        "rust" => &[
            "String",
            "Vec",
            "Option",
            "Result",
            "Box",
            "Rc",
            "Arc",
            "HashMap",
            "HashSet",
            "BTreeMap",
            "BTreeSet",
            "VecDeque",
            "LinkedList",
            "BinaryHeap",
            "Cell",
            "RefCell",
            "Mutex",
            "RwLock",
            "Cow",
            "Pin",
            "PhantomData",
            "i8",
            "i16",
            "i32",
            "i64",
            "i128",
            "isize",
            "u8",
            "u16",
            "u32",
            "u64",
            "u128",
            "usize",
            "f32",
            "f64",
            "bool",
            "char",
            "str",
            "Self",
        ],
        "go" => &[
            "int",
            "int8",
            "int16",
            "int32",
            "int64",
            "uint",
            "uint8",
            "uint16",
            "uint32",
            "uint64",
            "uintptr",
            "float32",
            "float64",
            "complex64",
            "complex128",
            "string",
            "bool",
            "byte",
            "rune",
            "error",
        ],
        "c" => &[
            "size_t",
            "ssize_t",
            "ptrdiff_t",
            "intptr_t",
            "uintptr_t",
            "int8_t",
            "int16_t",
            "int32_t",
            "int64_t",
            "uint8_t",
            "uint16_t",
            "uint32_t",
            "uint64_t",
            "FILE",
            "string",
            "vector",
            "map",
            "set",
            "pair",
            "tuple",
            "shared_ptr",
            "unique_ptr",
        ],
        _ => &[],
    }
}

/// Get the line-comment prefix for a normalized language.
fn comment_prefix(lang: &str) -> &'static str {
    match lang {
        "python" | "shell" | "yaml" | "toml" => "#",
        "c" | "rust" | "js" | "go" => "//",
        // json has no comments (jsonc uses //) — treat as //
        _ => "//",
    }
}

/// Apply syntax-aware ANSI highlighting to a single code line.
///
/// Colorizes keywords (bold cyan), types (magenta), strings (green),
/// comments (dim), and numbers (yellow).
/// JSON keys are highlighted in cyan, YAML keys in bold yellow.
/// Falls back to DIM when language is unrecognized.
/// Highlight a diff line: + lines green, - lines red, @@ lines cyan.
pub fn highlight_diff_line(line: &str) -> String {
    let trimmed = line.trim_start();
    if trimmed.starts_with('+') && !trimmed.starts_with("+++") {
        format!("{GREEN}{line}{RESET}")
    } else if trimmed.starts_with('-') && !trimmed.starts_with("---") {
        format!("{RED}{line}{RESET}")
    } else if trimmed.starts_with("@@") {
        format!("{CYAN}{line}{RESET}")
    } else if trimmed.starts_with("---") || trimmed.starts_with("+++") {
        format!("{BOLD}{line}{RESET}")
    } else {
        format!("{DIM}{line}{RESET}")
    }
}

pub fn highlight_code_line(lang: &str, line: &str) -> String {
    let norm = match normalize_lang(lang) {
        Some(n) => n,
        None => return format!("{DIM}{line}{RESET}"),
    };

    let cp = comment_prefix(norm);
    let trimmed = line.trim_start();

    // Full-line comment detection
    if trimmed.starts_with(cp) {
        return format!("{DIM}{line}{RESET}");
    }

    // JSON: highlight keys and string values with simple heuristic
    if norm == "json" {
        return highlight_json_line(line);
    }

    // YAML: highlight keys (word before colon) and values
    if norm == "yaml" {
        return highlight_yaml_line(line);
    }

    // TOML: highlight keys and values
    if norm == "toml" {
        return highlight_toml_line(line);
    }

    let keywords = lang_keywords(norm);
    let types = lang_types(norm);
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut result = String::with_capacity(line.len() + 64);
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        // Check for inline comment: // or # (at current position)
        if i + 1 < len && chars[i] == '/' && chars[i + 1] == '/' && cp == "//" {
            // Rest of line is a comment
            let rest: String = chars[i..].iter().collect();
            result.push_str(&format!("{DIM}{rest}{RESET}"));
            break;
        }
        if ch == '#' && cp == "#" {
            let rest: String = chars[i..].iter().collect();
            result.push_str(&format!("{DIM}{rest}{RESET}"));
            break;
        }

        // String literals: "..." or '...'
        if ch == '"' || ch == '\'' {
            let quote = ch;
            let mut s = String::new();
            s.push(ch);
            i += 1;
            while i < len {
                let c = chars[i];
                s.push(c);
                i += 1;
                if c == '\\' && i < len {
                    s.push(chars[i]);
                    i += 1;
                } else if c == quote {
                    break;
                }
            }
            result.push_str(&format!("{GREEN}{s}{RESET}"));
            continue;
        }

        // Numbers: digit sequences (possibly with . for floats)
        if ch.is_ascii_digit()
            && (i == 0 || !chars[i - 1].is_ascii_alphanumeric() && chars[i - 1] != '_')
        {
            let mut num = String::new();
            while i < len && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == '_') {
                num.push(chars[i]);
                i += 1;
            }
            // Don't color if followed by an alpha char (it's part of an identifier)
            if i < len && (chars[i].is_ascii_alphabetic() || chars[i] == '_') {
                result.push_str(&num);
            } else {
                result.push_str(&format!("{YELLOW}{num}{RESET}"));
            }
            continue;
        }

        // Word: check for keyword or type
        if ch.is_ascii_alphabetic() || ch == '_' {
            let mut word = String::new();
            let start = i;
            while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                word.push(chars[i]);
                i += 1;
            }
            // Only highlight if it's a standalone word (not part of a larger identifier)
            let before_ok = start == 0
                || (!chars[start - 1].is_ascii_alphanumeric() && chars[start - 1] != '_');
            let after_ok = i >= len || (!chars[i].is_ascii_alphanumeric() && chars[i] != '_');
            if before_ok && after_ok {
                if keywords.contains(&word.as_str()) {
                    result.push_str(&format!("{BOLD_CYAN}{word}{RESET}"));
                } else if types.contains(&word.as_str()) {
                    result.push_str(&format!("{MAGENTA}{word}{RESET}"));
                } else {
                    result.push_str(&word);
                }
            } else {
                result.push_str(&word);
            }
            continue;
        }

        result.push(ch);
        i += 1;
    }

    result
}

/// Highlight a JSON line: keys in cyan, strings in green, numbers in yellow.
fn highlight_json_line(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut result = String::with_capacity(line.len() + 64);
    let mut i = 0;
    let mut expecting_value = false;

    while i < len {
        let ch = chars[i];

        // String literal
        if ch == '"' {
            let mut s = String::new();
            s.push(ch);
            i += 1;
            while i < len {
                let c = chars[i];
                s.push(c);
                i += 1;
                if c == '\\' && i < len {
                    s.push(chars[i]);
                    i += 1;
                } else if c == '"' {
                    break;
                }
            }
            // Check if this string is followed by a colon (it's a key)
            let rest_trimmed: String = chars[i..].iter().collect();
            if !expecting_value && rest_trimmed.trim_start().starts_with(':') {
                result.push_str(&format!("{CYAN}{s}{RESET}"));
            } else {
                result.push_str(&format!("{GREEN}{s}{RESET}"));
            }
            continue;
        }

        if ch == ':' {
            expecting_value = true;
            result.push(ch);
            i += 1;
            continue;
        }

        if ch == ',' || ch == '{' || ch == '[' {
            expecting_value = false;
            result.push(ch);
            i += 1;
            continue;
        }

        // Numbers
        if ch.is_ascii_digit() || (ch == '-' && i + 1 < len && chars[i + 1].is_ascii_digit()) {
            let mut num = String::new();
            num.push(ch);
            i += 1;
            while i < len
                && (chars[i].is_ascii_digit()
                    || chars[i] == '.'
                    || chars[i] == 'e'
                    || chars[i] == 'E'
                    || chars[i] == '+'
                    || chars[i] == '-')
            {
                num.push(chars[i]);
                i += 1;
            }
            result.push_str(&format!("{YELLOW}{num}{RESET}"));
            continue;
        }

        // true/false/null
        if ch.is_ascii_alphabetic() {
            let mut word = String::new();
            while i < len && chars[i].is_ascii_alphabetic() {
                word.push(chars[i]);
                i += 1;
            }
            match word.as_str() {
                "true" | "false" | "null" => {
                    result.push_str(&format!("{BOLD_CYAN}{word}{RESET}"));
                }
                _ => result.push_str(&word),
            }
            continue;
        }

        result.push(ch);
        i += 1;
    }

    result
}

/// Highlight a YAML line: keys in bold yellow, strings in green, numbers in yellow.
fn highlight_yaml_line(line: &str) -> String {
    let trimmed = line.trim_start();

    // Comment
    if trimmed.starts_with('#') {
        return format!("{DIM}{line}{RESET}");
    }

    // Section header [section]
    if trimmed.starts_with("---") || trimmed.starts_with("...") {
        return format!("{DIM}{line}{RESET}");
    }

    // Key-value pair: look for "key:" pattern
    if let Some(colon_pos) = trimmed.find(':') {
        let key_part = &trimmed[..colon_pos];
        // Only treat as key if it doesn't start with - (list item) and key is simple
        if !key_part.contains(' ') || key_part.starts_with("- ") || key_part.starts_with('-') {
            let indent = &line[..line.len() - trimmed.len()];
            let value_part = &trimmed[colon_pos + 1..];
            let value_highlighted = highlight_yaml_value(value_part);
            return format!("{indent}{BOLD_YELLOW}{key_part}{RESET}:{value_highlighted}");
        }
    }

    // List item
    if let Some(rest) = trimmed.strip_prefix("- ") {
        let indent = &line[..line.len() - trimmed.len()];
        return format!("{indent}- {}", highlight_yaml_value(rest));
    }

    line.to_string()
}

/// Highlight a YAML value (strings, numbers, booleans).
fn highlight_yaml_value(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return value.to_string();
    }

    // Inline comment
    if let Some(comment_pos) = trimmed.find(" #") {
        let before = &trimmed[..comment_pos];
        let after = &trimmed[comment_pos..];
        return format!(" {}{DIM}{after}{RESET}", highlight_yaml_value_inner(before));
    }

    format!(" {}", highlight_yaml_value_inner(trimmed))
}

fn highlight_yaml_value_inner(value: &str) -> String {
    // Quoted string
    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        return format!("{GREEN}{value}{RESET}");
    }

    // Boolean/null keywords
    match value {
        "true" | "false" | "yes" | "no" | "on" | "off" | "null" | "~" => {
            return format!("{BOLD_CYAN}{value}{RESET}");
        }
        _ => {}
    }

    // Number
    if value.parse::<f64>().is_ok() {
        return format!("{YELLOW}{value}{RESET}");
    }

    // Plain string — leave as-is
    value.to_string()
}

/// Highlight a TOML line: section headers in bold, keys in bold yellow.
fn highlight_toml_line(line: &str) -> String {
    let trimmed = line.trim_start();

    // Comment
    if trimmed.starts_with('#') {
        return format!("{DIM}{line}{RESET}");
    }

    // Section header [section] or [[array]]
    if trimmed.starts_with('[') {
        return format!("{BOLD}{CYAN}{line}{RESET}");
    }

    // Key = value
    if let Some(eq_pos) = trimmed.find('=') {
        let key_part = trimmed[..eq_pos].trim();
        let value_part = trimmed[eq_pos + 1..].trim();
        let indent = &line[..line.len() - trimmed.len()];
        let value_highlighted = highlight_toml_value(value_part);
        return format!("{indent}{BOLD_YELLOW}{key_part}{RESET} = {value_highlighted}");
    }

    line.to_string()
}

fn highlight_toml_value(value: &str) -> String {
    // String
    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        return format!("{GREEN}{value}{RESET}");
    }

    // Boolean
    match value {
        "true" | "false" => return format!("{BOLD_CYAN}{value}{RESET}"),
        _ => {}
    }

    // Number
    if value.parse::<f64>().is_ok() {
        return format!("{YELLOW}{value}{RESET}");
    }

    // Array or inline table — leave as-is for simplicity
    value.to_string()
}

/// Get pricing rates (per MTok) for a model.
/// Returns (input, cache_write, cache_read, output) or None if model is unknown.
fn model_pricing(model: &str) -> Option<(f64, f64, f64, f64)> {
    // Returns (input_per_MTok, cache_write_per_MTok, cache_read_per_MTok, output_per_MTok)
    // For providers without caching, cache_write and cache_read are set to 0.0.

    // Strip common OpenRouter prefixes (e.g. "anthropic/claude-sonnet-4-20250514")
    let model = model
        .strip_prefix("anthropic/")
        .or_else(|| model.strip_prefix("openai/"))
        .or_else(|| model.strip_prefix("google/"))
        .or_else(|| model.strip_prefix("deepseek/"))
        .or_else(|| model.strip_prefix("mistralai/"))
        .or_else(|| model.strip_prefix("x-ai/"))
        .or_else(|| model.strip_prefix("meta-llama/"))
        .unwrap_or(model);

    // ── Anthropic ─────────────────────────────────────────────────────
    // https://docs.anthropic.com/en/about-claude/pricing
    if model.contains("opus") {
        if model.contains("4-6")
            || model.contains("4-5")
            || model.contains("4.6")
            || model.contains("4.5")
        {
            return Some((5.0, 6.25, 0.50, 25.0));
        } else {
            return Some((15.0, 18.75, 1.50, 75.0));
        }
    }
    if model.contains("sonnet") {
        return Some((3.0, 3.75, 0.30, 15.0));
    }
    if model.contains("haiku") {
        if model.contains("4-5") || model.contains("4.5") {
            return Some((1.0, 1.25, 0.10, 5.0));
        } else {
            return Some((0.80, 1.0, 0.08, 4.0));
        }
    }

    // ── OpenAI ────────────────────────────────────────────────────────
    // https://platform.openai.com/docs/pricing
    if model.starts_with("gpt-4.1") {
        if model.contains("mini") {
            return Some((0.40, 0.0, 0.0, 1.60)); // gpt-4.1-mini
        } else if model.contains("nano") {
            return Some((0.10, 0.0, 0.0, 0.40)); // gpt-4.1-nano
        } else {
            return Some((2.00, 0.0, 0.0, 8.00)); // gpt-4.1
        }
    }
    if model.starts_with("gpt-4o") {
        if model.contains("mini") {
            return Some((0.15, 0.0, 0.0, 0.60)); // gpt-4o-mini
        } else {
            return Some((2.50, 0.0, 0.0, 10.00)); // gpt-4o
        }
    }
    if model.starts_with("o4-mini") {
        return Some((1.10, 0.0, 0.0, 4.40));
    }
    if model.starts_with("o3-mini") {
        return Some((1.10, 0.0, 0.0, 4.40));
    }
    if model == "o3" {
        return Some((2.00, 0.0, 0.0, 8.00));
    }

    // ── Google Gemini ─────────────────────────────────────────────────
    // https://ai.google.dev/pricing
    if model.contains("gemini-2.5-pro") {
        return Some((1.25, 0.0, 0.0, 10.00));
    }
    if model.contains("gemini-2.5-flash") {
        return Some((0.15, 0.0, 0.0, 0.60));
    }
    if model.contains("gemini-2.0-flash") {
        return Some((0.10, 0.0, 0.0, 0.40));
    }

    // ── DeepSeek ──────────────────────────────────────────────────────
    // https://platform.deepseek.com/api-docs/pricing/
    if model.contains("deepseek-chat") || model.contains("deepseek-v3") {
        return Some((0.27, 0.0, 0.0, 1.10));
    }
    if model.contains("deepseek-reasoner") || model.contains("deepseek-r1") {
        return Some((0.55, 0.0, 0.0, 2.19));
    }

    // ── Mistral ───────────────────────────────────────────────────────
    // https://mistral.ai/products#pricing
    if model.contains("mistral-large") {
        return Some((2.00, 0.0, 0.0, 6.00));
    }
    if model.contains("mistral-small") || model.contains("mistral-latest") {
        return Some((0.10, 0.0, 0.0, 0.30));
    }
    if model.contains("codestral") {
        return Some((0.30, 0.0, 0.0, 0.90));
    }

    // ── xAI (Grok) ───────────────────────────────────────────────────
    // https://docs.x.ai/docs/models#models-and-pricing
    if model.contains("grok-3") {
        if model.contains("mini") {
            return Some((0.30, 0.0, 0.0, 0.50));
        } else {
            return Some((3.00, 0.0, 0.0, 15.00));
        }
    }
    if model.contains("grok-2") {
        return Some((2.00, 0.0, 0.0, 10.00));
    }

    // ── Groq (hosted models) ─────────────────────────────────────────
    // https://groq.com/pricing/
    if model.contains("llama-3.3-70b") || model.contains("llama3-70b") {
        return Some((0.59, 0.0, 0.0, 0.79));
    }
    if model.contains("llama-3.1-8b") || model.contains("llama3-8b") {
        return Some((0.05, 0.0, 0.0, 0.08));
    }
    if model.contains("mixtral-8x7b") {
        return Some((0.24, 0.0, 0.0, 0.24));
    }
    if model.contains("gemma2-9b") {
        return Some((0.20, 0.0, 0.0, 0.20));
    }

    None
}

/// Estimate cost in USD for a given usage and model.
/// Returns None if the model pricing is unknown.
pub fn estimate_cost(usage: &yoagent::Usage, model: &str) -> Option<f64> {
    let (input_cost, cw_cost, cr_cost, output_cost) = cost_breakdown(usage, model)?;
    Some(input_cost + cw_cost + cr_cost + output_cost)
}

/// Get individual cost components for a usage and model.
/// Returns (input_cost, cache_write_cost, cache_read_cost, output_cost) or None if model unknown.
pub fn cost_breakdown(usage: &yoagent::Usage, model: &str) -> Option<(f64, f64, f64, f64)> {
    let (input_per_m, cache_write_per_m, cache_read_per_m, output_per_m) = model_pricing(model)?;

    let input_cost = usage.input as f64 * input_per_m / 1_000_000.0;
    let cache_write_cost = usage.cache_write as f64 * cache_write_per_m / 1_000_000.0;
    let cache_read_cost = usage.cache_read as f64 * cache_read_per_m / 1_000_000.0;
    let output_cost = usage.output as f64 * output_per_m / 1_000_000.0;

    Some((input_cost, cache_write_cost, cache_read_cost, output_cost))
}

/// Format a cost in USD for display (e.g., "$0.0042", "$1.23").
pub fn format_cost(cost: f64) -> String {
    if cost < 0.01 {
        format!("${:.4}", cost)
    } else if cost < 1.0 {
        format!("${:.3}", cost)
    } else {
        format!("${:.2}", cost)
    }
}

/// Format a duration for display (e.g., "1.2s", "350ms", "2m 15s").
pub fn format_duration(d: std::time::Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{ms}ms")
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        format!("{mins}m {secs}s")
    }
}

/// Format a token count for display (e.g., 1500 -> "1.5k", 1000000 -> "1.0M").
pub fn format_token_count(count: u64) -> String {
    if count < 1000 {
        format!("{count}")
    } else if count < 1_000_000 {
        format!("{:.1}k", count as f64 / 1000.0)
    } else {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    }
}

/// Build a context usage bar (e.g., "████████░░░░░░░░░░░░ 40%").
pub fn context_bar(used: u64, max: u64) -> String {
    let pct = if max == 0 {
        0.0
    } else {
        (used as f64 / max as f64).min(1.0)
    };
    let width = 20;
    let filled = (pct * width as f64).round() as usize;
    let empty = width - filled;
    let bar: String = "█".repeat(filled) + &"░".repeat(empty);
    format!("{bar} {:.0}%", pct * 100.0)
}

/// Truncate a string with an ellipsis if it exceeds `max` characters.
pub fn truncate_with_ellipsis(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((idx, _)) => format!("{}…", &s[..idx]),
        None => s.to_string(),
    }
}

/// Truncate a string to `max` characters (no ellipsis).
#[cfg(test)]
pub fn truncate(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

/// Maximum number of diff lines to display before truncating.
const MAX_DIFF_LINES: usize = 20;

/// Format a colored unified diff between old_text and new_text.
/// Removed lines are shown in red with `- ` prefix, added lines in green with `+ ` prefix.
/// If the diff exceeds `MAX_DIFF_LINES`, it is truncated with an ellipsis note.
pub fn format_edit_diff(old_text: &str, new_text: &str) -> String {
    let mut lines: Vec<String> = Vec::new();

    // Show removed lines (old_text)
    if !old_text.is_empty() {
        for line in old_text.lines() {
            lines.push(format!("{RED}  - {line}{RESET}"));
        }
    }

    // Show added lines (new_text)
    if !new_text.is_empty() {
        for line in new_text.lines() {
            lines.push(format!("{GREEN}  + {line}{RESET}"));
        }
    }

    if lines.is_empty() {
        return String::new();
    }

    // Truncate if too many lines
    if lines.len() > MAX_DIFF_LINES {
        let remaining = lines.len() - MAX_DIFF_LINES;
        lines.truncate(MAX_DIFF_LINES);
        lines.push(format!("{DIM}  ... ({remaining} more lines){RESET}"));
    }

    lines.join("\n")
}

/// Format a human-readable summary for a tool execution.
pub fn format_tool_summary(tool_name: &str, args: &serde_json::Value) -> String {
    match tool_name {
        "bash" => {
            let cmd = args
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("...");
            format!("$ {}", truncate_with_ellipsis(cmd, 80))
        }
        "read_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            format!("read {}", path)
        }
        "write_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            let line_info = args
                .get("content")
                .and_then(|v| v.as_str())
                .map(|c| {
                    let count = c.lines().count();
                    format!(" ({count} lines)")
                })
                .unwrap_or_default();
            format!("write {path}{line_info}")
        }
        "edit_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            format!("edit {}", path)
        }
        "list_files" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            format!("ls {}", path)
        }
        "search" => {
            let pat = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("?");
            format!("search '{}'", truncate_with_ellipsis(pat, 60))
        }
        _ => tool_name.to_string(),
    }
}

/// Print usage stats after a prompt response.
pub fn print_usage(
    usage: &yoagent::Usage,
    total: &yoagent::Usage,
    model: &str,
    elapsed: std::time::Duration,
) {
    if usage.input > 0 || usage.output > 0 {
        let cache_info = if usage.cache_read > 0 || usage.cache_write > 0 {
            format!(
                "  [cache: {} read, {} write]",
                usage.cache_read, usage.cache_write
            )
        } else {
            String::new()
        };
        let cost_info = estimate_cost(usage, model)
            .map(|c| format!("  cost: {}", format_cost(c)))
            .unwrap_or_default();
        let total_cost_info = estimate_cost(total, model)
            .map(|c| format!("  total: {}", format_cost(c)))
            .unwrap_or_default();
        let elapsed_str = format_duration(elapsed);
        println!(
            "\n{DIM}  tokens: {} in / {} out{cache_info}  (session: {} in / {} out){cost_info}{total_cost_info}  ⏱ {elapsed_str}{RESET}",
            usage.input, usage.output, total.input, total.output
        );
    }
}

/// Incremental markdown renderer for streamed text output.
/// Tracks state across partial deltas to apply ANSI formatting for
/// code blocks, inline code, bold text, and headers.
///
/// Designed for LLM streaming: mid-line tokens are rendered immediately
/// with inline formatting. Only line boundaries buffer briefly to detect
/// code fences (`` ``` ``) and headers (`#`).
pub struct MarkdownRenderer {
    in_code_block: bool,
    code_lang: Option<String>,
    line_buffer: String,
    /// Whether we're at the start of a new line (need to detect fence/header).
    line_start: bool,
}

/// Minimum characters needed at line start before we can determine
/// the line is NOT a code fence or header and flush it immediately.
const LINE_START_RESOLVE_THRESHOLD: usize = 4;

impl MarkdownRenderer {
    /// Create a new renderer with empty state.
    pub fn new() -> Self {
        Self {
            in_code_block: false,
            code_lang: None,
            line_buffer: String::new(),
            line_start: true,
        }
    }

    /// Process a delta chunk and return ANSI-formatted output.
    ///
    /// **Streaming behavior:**
    /// - At line start, buffers briefly to detect code fences/headers (typically 1–4 chars)
    /// - Mid-line, renders immediately with inline formatting (bold, inline code)
    /// - Complete lines (ending with `\n`) are always processed immediately
    pub fn render_delta(&mut self, delta: &str) -> String {
        let mut output = String::new();

        // If we're mid-line and NOT in a code block, we can render tokens immediately
        // without buffering (code fences and headers only matter at line start).
        if !self.line_start && !self.in_code_block {
            // Split on newlines: everything before the first \n is mid-line
            if let Some(newline_pos) = delta.find('\n') {
                // Render the mid-line portion immediately
                let mid_line_part = &delta[..newline_pos];
                if !mid_line_part.is_empty() {
                    output.push_str(&self.render_inline(mid_line_part));
                }
                output.push('\n');
                self.line_start = true;

                // Process the rest (after the first \n) by buffering
                let rest = &delta[newline_pos + 1..];
                if !rest.is_empty() {
                    output.push_str(&self.render_delta_buffered(rest));
                }
            } else {
                // No newline — pure mid-line content, render immediately
                output.push_str(&self.render_inline(delta));
            }
            return output;
        }

        // We're at line start (or in a code block) — use buffered approach
        output.push_str(&self.render_delta_buffered(delta));
        output
    }

    /// Buffered rendering: adds delta to line_buffer, processes complete lines,
    /// and attempts early flush of line-start content when safe.
    fn render_delta_buffered(&mut self, delta: &str) -> String {
        let mut output = String::new();
        self.line_buffer.push_str(delta);

        // Process all complete lines (those ending with \n)
        while let Some(newline_pos) = self.line_buffer.find('\n') {
            let line = self.line_buffer[..newline_pos].to_string();
            self.line_buffer = self.line_buffer[newline_pos + 1..].to_string();
            output.push_str(&self.render_line(&line));
            output.push('\n');
            self.line_start = true;
        }

        // Try to resolve the line-start buffer early:
        // If we have enough characters to determine it's NOT a fence or header,
        // flush the buffer as inline text and switch to mid-line mode.
        if self.line_start && !self.line_buffer.is_empty() && !self.in_code_block {
            let trimmed = self.line_buffer.trim_start();
            let could_be_fence =
                trimmed.is_empty() || trimmed.starts_with('`') || "`".starts_with(trimmed);
            let could_be_header =
                trimmed.is_empty() || trimmed.starts_with('#') || "#".starts_with(trimmed);

            if trimmed.len() >= LINE_START_RESOLVE_THRESHOLD && !could_be_fence && !could_be_header
            {
                // Definitely not a fence or header — flush as inline text
                let buf = std::mem::take(&mut self.line_buffer);
                output.push_str(&self.render_inline(&buf));
                self.line_start = false;
            } else if !could_be_fence && !could_be_header {
                // Even with fewer chars, if it can't possibly be a fence/header, flush
                let buf = std::mem::take(&mut self.line_buffer);
                output.push_str(&self.render_inline(&buf));
                self.line_start = false;
            }
        }

        output
    }

    /// Flush any remaining buffered content (call after stream ends).
    pub fn flush(&mut self) -> String {
        if self.line_buffer.is_empty() {
            return String::new();
        }
        let line = std::mem::take(&mut self.line_buffer);
        self.line_start = true;
        self.render_line(&line)
    }

    /// Render a single complete line, updating state for code fences.
    fn render_line(&mut self, line: &str) -> String {
        let trimmed = line.trim();
        // After rendering a complete line, next content will be at line start
        self.line_start = true;

        // Check for code fence (``` with optional language)
        if let Some(after_fence) = trimmed.strip_prefix("```") {
            if self.in_code_block {
                // Closing fence
                self.in_code_block = false;
                self.code_lang = None;
                return format!("{DIM}  └─{RESET}");
            } else {
                // Opening fence — capture language if present
                self.in_code_block = true;
                let lang = after_fence.trim();
                self.code_lang = if lang.is_empty() {
                    None
                } else {
                    Some(lang.to_string())
                };
                let lang_label = if lang.is_empty() {
                    String::new()
                } else {
                    format!(" {CYAN}{lang}{RESET}")
                };
                return format!("{DIM}  ┌─{lang_label}{RESET}");
            }
        }

        if self.in_code_block {
            // Code block content with left border
            let is_diff = self.code_lang.as_deref() == Some("diff");
            let content = if is_diff || trimmed.is_empty() {
                // Diff mode or empty line — handle +/- highlighting
                highlight_diff_line(line)
            } else if let Some(ref lang) = self.code_lang {
                // Check for inline diff markers even in non-diff code blocks
                if trimmed.starts_with('+') && !trimmed.starts_with("++") {
                    format!("{GREEN}{line}{RESET}")
                } else if trimmed.starts_with('-') && !trimmed.starts_with("--") {
                    format!("{RED}{line}{RESET}")
                } else {
                    highlight_code_line(lang, line)
                }
            } else {
                format!("{DIM}{line}{RESET}")
            };
            return format!("{DIM}  │{RESET} {content}");
        }

        // Header: # at line start → BOLD+CYAN
        if trimmed.starts_with('#') {
            return format!("{BOLD}{CYAN}{line}{RESET}");
        }

        // Horizontal rule: --- or *** or ___
        if trimmed.len() >= 3
            && (trimmed.chars().all(|c| c == '-' || c == ' ')
                || trimmed.chars().all(|c| c == '*' || c == ' ')
                || trimmed.chars().all(|c| c == '_' || c == ' '))
            && trimmed.chars().filter(|c| !c.is_whitespace()).count() >= 3
        {
            return format!("{DIM}  ────────────────────────────────{RESET}");
        }

        // Blockquote: > at line start
        if let Some(quote_content) = trimmed.strip_prefix('>') {
            let inner = quote_content.trim_start();
            return format!("{DIM}  │ {inner}{RESET}");
        }

        // Bullet lists: -, *, or numbered (1.)
        if (trimmed.starts_with("- ") || trimmed.starts_with("* ")) && trimmed.len() > 2 {
            let rest = &trimmed[2..];
            let formatted_rest = self.render_inline(rest);
            let indent = &line[..line.len() - trimmed.len()]; // preserve leading whitespace
            return format!("{indent}  • {formatted_rest}");
        }
        // Numbered lists: "1. ", "2. ", etc.
        if let Some(dot_pos) = trimmed.find(". ") {
            if dot_pos <= 3 && trimmed[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
                let num = &trimmed[..dot_pos];
                let rest = &trimmed[dot_pos + 2..];
                let formatted_rest = self.render_inline(rest);
                let indent = &line[..line.len() - trimmed.len()];
                return format!("{indent}  {DIM}{num}.{RESET} {formatted_rest}");
            }
        }

        // Apply inline formatting for normal text
        self.render_inline(line)
    }

    /// Apply inline formatting (bold, inline code) to a line of normal text.
    fn render_inline(&self, line: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = line.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            // Check for bold: **text**
            if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
                // Find closing **
                if let Some(close) = self.find_double_star(&chars, i + 2) {
                    let inner: String = chars[i + 2..close].iter().collect();
                    result.push_str(&format!("{BOLD}{inner}{RESET}"));
                    i = close + 2;
                    continue;
                }
            }

            // Check for inline code: `text`
            if chars[i] == '`' {
                // Find closing backtick (not another opening fence)
                if let Some(close) = self.find_backtick(&chars, i + 1) {
                    let inner: String = chars[i + 1..close].iter().collect();
                    result.push_str(&format!("{CYAN}{inner}{RESET}"));
                    i = close + 1;
                    continue;
                }
            }

            result.push(chars[i]);
            i += 1;
        }

        result
    }

    /// Find closing ** starting from position `from` in char slice.
    fn find_double_star(&self, chars: &[char], from: usize) -> Option<usize> {
        let len = chars.len();
        let mut j = from;
        while j + 1 < len {
            if chars[j] == '*' && chars[j + 1] == '*' {
                return Some(j);
            }
            j += 1;
        }
        None
    }

    /// Find closing backtick starting from position `from` in char slice.
    fn find_backtick(&self, chars: &[char], from: usize) -> Option<usize> {
        (from..chars.len()).find(|&j| chars[j] == '`')
    }
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// --- Waiting spinner for AI responses ---

/// Braille spinner frames used for the "thinking" animation.
pub const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Get the spinner frame for a given tick index (wraps around).
pub fn spinner_frame(tick: usize) -> char {
    SPINNER_FRAMES[tick % SPINNER_FRAMES.len()]
}

/// A handle to a running spinner task. Dropping or calling `stop()` cancels it.
pub struct Spinner {
    cancel: tokio::sync::watch::Sender<bool>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl Spinner {
    /// Start a spinner that prints frames to stderr every 100ms.
    /// The spinner shows `⠋ thinking...` cycling through braille characters.
    pub fn start() -> Self {
        let (cancel_tx, mut cancel_rx) = tokio::sync::watch::channel(false);
        let handle = tokio::spawn(async move {
            let mut tick: usize = 0;
            loop {
                // Check cancellation before printing
                if *cancel_rx.borrow() {
                    // Clear the spinner line
                    eprint!("\r\x1b[K");
                    break;
                }
                let frame = spinner_frame(tick);
                eprint!("\r{DIM}  {frame} thinking...{RESET}");
                tick = tick.wrapping_add(1);

                // Wait 100ms or until cancelled
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
                    _ = cancel_rx.changed() => {
                        // Clear the spinner line
                        eprint!("\r\x1b[K");
                        break;
                    }
                }
            }
        });
        Self {
            cancel: cancel_tx,
            handle: Some(handle),
        }
    }

    /// Stop the spinner and clear its output.
    pub fn stop(mut self) {
        let _ = self.cancel.send(true);
        // Take the handle so Drop doesn't try to stop again
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        let _ = self.cancel.send(true);
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("hello world", 5), "hello");
    }

    #[test]
    fn test_truncate_unicode() {
        assert_eq!(truncate("héllo wörld", 5), "héllo");
    }

    #[test]
    fn test_truncate_empty() {
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn test_truncate_adds_ellipsis() {
        assert_eq!(truncate_with_ellipsis("hello world", 5), "hello…");
        assert_eq!(truncate_with_ellipsis("hi", 5), "hi");
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
    }

    #[test]
    fn test_format_token_count() {
        assert_eq!(format_token_count(0), "0");
        assert_eq!(format_token_count(999), "999");
        assert_eq!(format_token_count(1000), "1.0k");
        assert_eq!(format_token_count(1500), "1.5k");
        assert_eq!(format_token_count(10000), "10.0k");
        assert_eq!(format_token_count(150000), "150.0k");
        assert_eq!(format_token_count(1000000), "1.0M");
        assert_eq!(format_token_count(2500000), "2.5M");
    }

    #[test]
    fn test_context_bar() {
        let bar = context_bar(50000, 200000);
        assert!(bar.contains('█'));
        assert!(bar.contains("25%"));

        let bar_empty = context_bar(0, 200000);
        assert!(bar_empty.contains("0%"));

        let bar_full = context_bar(200000, 200000);
        assert!(bar_full.contains("100%"));
    }

    #[test]
    fn test_format_cost() {
        assert_eq!(format_cost(0.0001), "$0.0001");
        assert_eq!(format_cost(0.0042), "$0.0042");
        assert_eq!(format_cost(0.05), "$0.050");
        assert_eq!(format_cost(0.123), "$0.123");
        assert_eq!(format_cost(1.5), "$1.50");
        assert_eq!(format_cost(12.345), "$12.35");
    }

    #[test]
    fn test_format_duration_ms() {
        assert_eq!(
            format_duration(std::time::Duration::from_millis(50)),
            "50ms"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_millis(999)),
            "999ms"
        );
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(
            format_duration(std::time::Duration::from_millis(1000)),
            "1.0s"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_millis(1500)),
            "1.5s"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_millis(30000)),
            "30.0s"
        );
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(
            format_duration(std::time::Duration::from_millis(60000)),
            "1m 0s"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_millis(90000)),
            "1m 30s"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_millis(125000)),
            "2m 5s"
        );
    }

    #[test]
    fn test_estimate_cost_opus() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 100_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        let cost = estimate_cost(&usage, "claude-opus-4-6").unwrap();
        assert!((cost - 7.5).abs() < 0.001);
    }

    #[test]
    fn test_estimate_cost_sonnet() {
        let usage = yoagent::Usage {
            input: 500_000,
            output: 50_000,
            cache_read: 200_000,
            cache_write: 100_000,
            total_tokens: 0,
        };
        let cost = estimate_cost(&usage, "claude-sonnet-4-6").unwrap();
        assert!((cost - 2.685).abs() < 0.001);
    }

    #[test]
    fn test_estimate_cost_haiku() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 500_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        let cost = estimate_cost(&usage, "claude-haiku-4-5").unwrap();
        assert!((cost - 3.5).abs() < 0.001);
    }

    #[test]
    fn test_estimate_cost_unknown_model() {
        let usage = yoagent::Usage {
            input: 1000,
            output: 1000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // A truly unknown model should return None
        assert!(estimate_cost(&usage, "unknown-model-xyz").is_none());
    }

    #[test]
    fn test_cost_breakdown_opus() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 100_000,
            cache_read: 500_000,
            cache_write: 200_000,
            total_tokens: 0,
        };
        let (input, cw, cr, output) = cost_breakdown(&usage, "claude-opus-4-6").unwrap();
        // input: 1M * 5/M = 5.0
        assert!((input - 5.0).abs() < 0.001);
        // output: 100k * 25/M = 2.5
        assert!((output - 2.5).abs() < 0.001);
        // cache_read: 500k * 0.50/M = 0.25
        assert!((cr - 0.25).abs() < 0.001);
        // cache_write: 200k * 6.25/M = 1.25
        assert!((cw - 1.25).abs() < 0.001);
        // Total should match estimate_cost
        let total = input + cw + cr + output;
        let expected = estimate_cost(&usage, "claude-opus-4-6").unwrap();
        assert!((total - expected).abs() < 0.001);
    }

    #[test]
    fn test_cost_breakdown_unknown_model() {
        let usage = yoagent::Usage {
            input: 1000,
            output: 1000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        assert!(cost_breakdown(&usage, "unknown-model-xyz").is_none());
    }

    // ── OpenAI model pricing tests ───────────────────────────────────

    #[test]
    fn test_estimate_cost_gpt4o() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 100_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // gpt-4o: $2.50/MTok input, $10.00/MTok output
        let cost = estimate_cost(&usage, "gpt-4o").unwrap();
        assert!((cost - 3.5).abs() < 0.001, "gpt-4o cost: {cost}");
    }

    #[test]
    fn test_estimate_cost_gpt4o_mini() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // gpt-4o-mini: $0.15/MTok input, $0.60/MTok output
        let cost = estimate_cost(&usage, "gpt-4o-mini").unwrap();
        assert!((cost - 0.75).abs() < 0.001, "gpt-4o-mini cost: {cost}");
    }

    #[test]
    fn test_estimate_cost_gpt41() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 100_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // gpt-4.1: $2.00/MTok input, $8.00/MTok output
        let cost = estimate_cost(&usage, "gpt-4.1").unwrap();
        assert!((cost - 2.8).abs() < 0.001, "gpt-4.1 cost: {cost}");
    }

    #[test]
    fn test_estimate_cost_gpt41_mini() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // gpt-4.1-mini: $0.40/MTok input, $1.60/MTok output
        let cost = estimate_cost(&usage, "gpt-4.1-mini").unwrap();
        assert!((cost - 2.0).abs() < 0.001, "gpt-4.1-mini cost: {cost}");
    }

    #[test]
    fn test_estimate_cost_o3() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 100_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // o3: $2.00/MTok input, $8.00/MTok output
        let cost = estimate_cost(&usage, "o3").unwrap();
        assert!((cost - 2.8).abs() < 0.001, "o3 cost: {cost}");
    }

    #[test]
    fn test_estimate_cost_o4_mini() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 100_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // o4-mini: $1.10/MTok input, $4.40/MTok output
        let cost = estimate_cost(&usage, "o4-mini").unwrap();
        assert!((cost - 1.54).abs() < 0.001, "o4-mini cost: {cost}");
    }

    // ── Google Gemini pricing tests ──────────────────────────────────

    #[test]
    fn test_estimate_cost_gemini_25_pro() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 100_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // gemini-2.5-pro: $1.25/MTok input, $10.00/MTok output
        let cost = estimate_cost(&usage, "gemini-2.5-pro").unwrap();
        assert!((cost - 2.25).abs() < 0.001, "gemini-2.5-pro cost: {cost}");
    }

    #[test]
    fn test_estimate_cost_gemini_25_flash() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // gemini-2.5-flash: $0.15/MTok input, $0.60/MTok output
        let cost = estimate_cost(&usage, "gemini-2.5-flash").unwrap();
        assert!((cost - 0.75).abs() < 0.001, "gemini-2.5-flash cost: {cost}");
    }

    #[test]
    fn test_estimate_cost_gemini_20_flash() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // gemini-2.0-flash: $0.10/MTok input, $0.40/MTok output
        let cost = estimate_cost(&usage, "gemini-2.0-flash").unwrap();
        assert!((cost - 0.50).abs() < 0.001, "gemini-2.0-flash cost: {cost}");
    }

    // ── DeepSeek pricing tests ───────────────────────────────────────

    #[test]
    fn test_estimate_cost_deepseek_chat() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // deepseek-chat: $0.27/MTok input, $1.10/MTok output
        let cost = estimate_cost(&usage, "deepseek-chat").unwrap();
        assert!((cost - 1.37).abs() < 0.001, "deepseek-chat cost: {cost}");
    }

    #[test]
    fn test_estimate_cost_deepseek_reasoner() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // deepseek-reasoner: $0.55/MTok input, $2.19/MTok output
        let cost = estimate_cost(&usage, "deepseek-reasoner").unwrap();
        assert!(
            (cost - 2.74).abs() < 0.001,
            "deepseek-reasoner cost: {cost}"
        );
    }

    // ── Mistral pricing tests ────────────────────────────────────────

    #[test]
    fn test_estimate_cost_mistral_large() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 100_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // mistral-large: $2.00/MTok input, $6.00/MTok output
        let cost = estimate_cost(&usage, "mistral-large-latest").unwrap();
        assert!((cost - 2.6).abs() < 0.001, "mistral-large cost: {cost}");
    }

    #[test]
    fn test_estimate_cost_mistral_small() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // mistral-small: $0.10/MTok input, $0.30/MTok output
        let cost = estimate_cost(&usage, "mistral-small-latest").unwrap();
        assert!((cost - 0.40).abs() < 0.001, "mistral-small cost: {cost}");
    }

    #[test]
    fn test_estimate_cost_codestral() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // codestral: $0.30/MTok input, $0.90/MTok output
        let cost = estimate_cost(&usage, "codestral-latest").unwrap();
        assert!((cost - 1.20).abs() < 0.001, "codestral cost: {cost}");
    }

    // ── xAI (Grok) pricing tests ─────────────────────────────────────

    #[test]
    fn test_estimate_cost_grok3() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 100_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // grok-3: $3.00/MTok input, $15.00/MTok output
        let cost = estimate_cost(&usage, "grok-3").unwrap();
        assert!((cost - 4.5).abs() < 0.001, "grok-3 cost: {cost}");
    }

    #[test]
    fn test_estimate_cost_grok3_mini() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // grok-3-mini: $0.30/MTok input, $0.50/MTok output
        let cost = estimate_cost(&usage, "grok-3-mini").unwrap();
        assert!((cost - 0.80).abs() < 0.001, "grok-3-mini cost: {cost}");
    }

    // ── Groq pricing tests ───────────────────────────────────────────

    #[test]
    fn test_estimate_cost_groq_llama70b() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // llama-3.3-70b on Groq: $0.59/MTok input, $0.79/MTok output
        let cost = estimate_cost(&usage, "llama-3.3-70b-versatile").unwrap();
        assert!((cost - 1.38).abs() < 0.001, "llama-3.3-70b cost: {cost}");
    }

    #[test]
    fn test_estimate_cost_groq_llama8b() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // llama-3.1-8b on Groq: $0.05/MTok input, $0.08/MTok output
        let cost = estimate_cost(&usage, "llama-3.1-8b-instant").unwrap();
        assert!((cost - 0.13).abs() < 0.001, "llama-3.1-8b cost: {cost}");
    }

    // ── OpenRouter prefix stripping tests ────────────────────────────

    #[test]
    fn test_estimate_cost_openrouter_anthropic_prefix() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 100_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        // OpenRouter uses "anthropic/claude-sonnet-4-20250514" format
        let cost = estimate_cost(&usage, "anthropic/claude-sonnet-4-20250514").unwrap();
        let direct_cost = estimate_cost(&usage, "claude-sonnet-4-20250514").unwrap();
        assert!(
            (cost - direct_cost).abs() < 0.001,
            "OpenRouter prefix should resolve to same pricing"
        );
    }

    #[test]
    fn test_estimate_cost_openrouter_openai_prefix() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 100_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        let cost = estimate_cost(&usage, "openai/gpt-4o").unwrap();
        let direct_cost = estimate_cost(&usage, "gpt-4o").unwrap();
        assert!(
            (cost - direct_cost).abs() < 0.001,
            "OpenRouter openai/ prefix should resolve to same pricing"
        );
    }

    #[test]
    fn test_estimate_cost_openrouter_google_prefix() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
        };
        let cost = estimate_cost(&usage, "google/gemini-2.0-flash").unwrap();
        let direct_cost = estimate_cost(&usage, "gemini-2.0-flash").unwrap();
        assert!(
            (cost - direct_cost).abs() < 0.001,
            "OpenRouter google/ prefix should resolve to same pricing"
        );
    }

    // ── Non-caching provider zero cache costs ────────────────────────

    #[test]
    fn test_non_anthropic_providers_zero_cache_costs() {
        let usage = yoagent::Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 500_000,
            cache_write: 200_000,
            total_tokens: 0,
        };
        // For non-Anthropic models, cache_write and cache_read rates are 0
        // so even with cache_read/cache_write tokens, those don't add to cost
        let (_, cw, cr, _) = cost_breakdown(&usage, "gpt-4o").unwrap();
        assert!(
            cw.abs() < 0.001 && cr.abs() < 0.001,
            "Non-Anthropic models should have zero cache costs: cw={cw}, cr={cr}"
        );
    }

    #[test]
    fn test_format_tool_summary_bash() {
        let args = serde_json::json!({"command": "echo hello"});
        assert_eq!(format_tool_summary("bash", &args), "$ echo hello");
    }

    #[test]
    fn test_format_tool_summary_bash_long_command() {
        let long_cmd = "a".repeat(100);
        let args = serde_json::json!({"command": long_cmd});
        let result = format_tool_summary("bash", &args);
        assert!(result.starts_with("$ "));
        assert!(result.ends_with('…'));
        assert!(result.len() < 100);
    }

    #[test]
    fn test_format_tool_summary_read_file() {
        let args = serde_json::json!({"path": "src/main.rs"});
        assert_eq!(format_tool_summary("read_file", &args), "read src/main.rs");
    }

    #[test]
    fn test_format_tool_summary_write_file() {
        let args = serde_json::json!({"path": "out.txt"});
        assert_eq!(format_tool_summary("write_file", &args), "write out.txt");
    }

    #[test]
    fn test_format_tool_summary_edit_file() {
        let args = serde_json::json!({"path": "foo.rs"});
        assert_eq!(format_tool_summary("edit_file", &args), "edit foo.rs");
    }

    #[test]
    fn test_format_tool_summary_list_files() {
        let args = serde_json::json!({"path": "src/"});
        assert_eq!(format_tool_summary("list_files", &args), "ls src/");
    }

    #[test]
    fn test_format_tool_summary_list_files_no_path() {
        let args = serde_json::json!({});
        assert_eq!(format_tool_summary("list_files", &args), "ls .");
    }

    #[test]
    fn test_format_tool_summary_search() {
        let args = serde_json::json!({"pattern": "TODO"});
        assert_eq!(format_tool_summary("search", &args), "search 'TODO'");
    }

    #[test]
    fn test_format_tool_summary_unknown_tool() {
        let args = serde_json::json!({});
        assert_eq!(format_tool_summary("custom_tool", &args), "custom_tool");
    }

    #[test]
    fn test_color_struct_display_outputs_ansi() {
        // Color struct should produce the ANSI code when color is enabled
        let c = Color("\x1b[1m");
        let formatted = format!("{c}");
        // We can't guarantee NO_COLOR isn't set in the test environment,
        // but the type itself should compile and format correctly.
        assert!(formatted == "\x1b[1m" || formatted.is_empty());
    }

    #[test]
    fn test_color_struct_display_consistency() {
        // All color constants should be the same type and format without panic
        let result = format!("{BOLD}{DIM}{GREEN}{YELLOW}{CYAN}{RED}{RESET}");
        // Should either have all codes or be empty (if NO_COLOR is set)
        assert!(result.contains('\x1b') || result.is_empty());
    }

    // --- MarkdownRenderer tests ---

    /// Helper: render a full string through the renderer (not streamed).
    fn render_full(input: &str) -> String {
        let mut r = MarkdownRenderer::new();
        let mut out = r.render_delta(input);
        out.push_str(&r.flush());
        out
    }

    #[test]
    fn test_md_code_block_detection() {
        let input = "before\n```\ncode line\n```\nafter\n";
        let out = render_full(input);
        // "code line" should be wrapped in DIM
        assert!(out.contains(&format!("{DIM}code line{RESET}")));
        // "before" and "after" should NOT be dim
        assert!(out.contains("before"));
        assert!(out.contains("after"));
    }

    #[test]
    fn test_md_code_block_with_language() {
        let input = "```rust\nlet x = 1;\n```\n";
        let mut r = MarkdownRenderer::new();
        let out = r.render_delta(input);
        let flushed = r.flush();
        let full = format!("{out}{flushed}");
        // Opening fence shows ┌─ with language label
        assert!(full.contains("┌─"), "should have opening border");
        assert!(full.contains(&format!("{CYAN}rust{RESET}")));
        // Code lines have │ left border
        assert!(full.contains("│"), "should have left border");
        // "let" should be keyword-highlighted, not just DIM
        assert!(full.contains(&format!("{BOLD_CYAN}let{RESET}")));
        // Number should be yellow
        assert!(full.contains(&format!("{YELLOW}1{RESET}")));
        // Closing fence shows └─
        assert!(full.contains("└─"), "should have closing border");
    }

    #[test]
    fn test_md_inline_code() {
        let out = render_full("use `Option<T>` here\n");
        assert!(out.contains(&format!("{CYAN}Option<T>{RESET}")));
    }

    #[test]
    fn test_md_bold_text() {
        let out = render_full("this is **important** stuff\n");
        assert!(out.contains(&format!("{BOLD}important{RESET}")));
    }

    #[test]
    fn test_md_header_rendering() {
        let out = render_full("# Hello World\n");
        assert!(out.contains(&format!("{BOLD}{CYAN}# Hello World{RESET}")));
    }

    #[test]
    fn test_md_header_h2() {
        let out = render_full("## Section Two\n");
        assert!(out.contains(&format!("{BOLD}{CYAN}## Section Two{RESET}")));
    }

    #[test]
    fn test_md_bullet_list() {
        let out = render_full("- first item\n- second item\n");
        assert!(out.contains("•"), "bullet list should use • marker");
        assert!(out.contains("first item"));
        assert!(out.contains("second item"));
    }

    #[test]
    fn test_md_numbered_list() {
        let out = render_full("1. step one\n2. step two\n");
        assert!(out.contains("1."));
        assert!(out.contains("step one"));
        assert!(out.contains("step two"));
    }

    #[test]
    fn test_md_blockquote() {
        let out = render_full("> some quoted text\n");
        assert!(out.contains("│"), "blockquote should have │ border");
        assert!(out.contains("some quoted text"));
    }

    #[test]
    fn test_md_horizontal_rule() {
        let out = render_full("---\n");
        assert!(out.contains("──"), "horizontal rule should render as line");
    }

    #[test]
    fn test_md_diff_code_block() {
        let input = "```diff\n+added line\n-removed line\n normal line\n```\n";
        let out = render_full(input);
        assert!(out.contains(&format!("{GREEN}+added line{RESET}")));
        assert!(out.contains(&format!("{RED}-removed line{RESET}")));
    }

    #[test]
    fn test_highlight_diff_line_added() {
        let out = highlight_diff_line("+new code");
        assert!(out.contains(&format!("{GREEN}+new code{RESET}")));
    }

    #[test]
    fn test_highlight_diff_line_removed() {
        let out = highlight_diff_line("-old code");
        assert!(out.contains(&format!("{RED}-old code{RESET}")));
    }

    #[test]
    fn test_highlight_diff_line_hunk_header() {
        let out = highlight_diff_line("@@ -1,3 +1,4 @@");
        assert!(out.contains(&format!("{CYAN}@@ -1,3 +1,4 @@{RESET}")));
    }

    #[test]
    fn test_md_partial_delta_fence() {
        // Fence marker split across multiple deltas
        let mut r = MarkdownRenderer::new();
        let out1 = r.render_delta("``");
        // Nothing emitted yet — still buffered (no newline)
        assert_eq!(out1, "");
        let out2 = r.render_delta("`\n");
        // Now the fence line is complete — shows ┌─ border
        assert!(out2.contains("┌─"));
        let out3 = r.render_delta("code here\n");
        // Code lines show with │ border
        assert!(out3.contains("code here"));
        assert!(out3.contains("│"));
        let out4 = r.render_delta("```\n");
        // Closing fence shows └─
        assert!(out4.contains("└─"));
        // After closing, normal text again
        let out5 = r.render_delta("normal\n");
        assert!(out5.contains("normal"));
        // When colors are enabled, DIM should not leak into normal text
        let dim_str = format!("{DIM}");
        if !dim_str.is_empty() {
            assert!(!out5.contains(&dim_str));
        }
    }

    #[test]
    fn test_md_empty_delta() {
        let mut r = MarkdownRenderer::new();
        let out = r.render_delta("");
        assert_eq!(out, "");
        let flushed = r.flush();
        assert_eq!(flushed, "");
    }

    #[test]
    fn test_md_multiple_code_blocks() {
        let input = "text\n```\nblock1\n```\nmiddle\n```python\nblock2\n```\nend\n";
        let out = render_full(input);
        // Untagged code block → DIM fallback
        assert!(out.contains(&format!("{DIM}block1{RESET}")));
        assert!(out.contains("middle"));
        // Python-tagged code block → syntax highlighted (no keyword match, plain output)
        assert!(out.contains("block2"));
        assert!(out.contains("end"));
    }

    #[test]
    fn test_md_inline_code_inside_bold() {
        // Inline code backticks inside bold — bold wraps, code is separate
        let out = render_full("**bold** and `code`\n");
        assert!(out.contains(&format!("{BOLD}bold{RESET}")));
        assert!(out.contains(&format!("{CYAN}code{RESET}")));
    }

    #[test]
    fn test_md_unmatched_backtick() {
        // Single backtick without closing — should pass through literally
        let out = render_full("it's a `partial\n");
        assert!(out.contains('`'));
        assert!(out.contains("partial"));
    }

    #[test]
    fn test_md_unmatched_bold() {
        // Unmatched ** should pass through literally
        let out = render_full("star **power\n");
        assert!(out.contains("**"));
        assert!(out.contains("power"));
    }

    #[test]
    fn test_md_flush_partial_line() {
        let mut r = MarkdownRenderer::new();
        // "no" at line start — can't be fence/header, resolves immediately
        let out = r.render_delta("no");
        assert!(
            out.contains("no"),
            "Short non-fence/non-header text resolves immediately"
        );
        // Continue adding text — mid-line now, immediate output
        let out2 = r.render_delta(" newline here");
        assert!(out2.contains(" newline here"));
    }

    #[test]
    fn test_md_flush_with_inline_formatting() {
        let mut r = MarkdownRenderer::new();
        // "hello **world**" — resolves as non-fence at line start, then renders inline
        let out = r.render_delta("hello **world**");
        let flushed = r.flush();
        let total = format!("{out}{flushed}");
        assert!(total.contains(&format!("{BOLD}world{RESET}")));
    }

    #[test]
    fn test_md_default_trait() {
        let r = MarkdownRenderer::default();
        assert!(!r.in_code_block);
        assert!(r.code_lang.is_none());
        assert!(r.line_buffer.is_empty());
        assert!(r.line_start);
    }

    // --- Streaming output tests (mid-line tokens should render immediately) ---

    #[test]
    fn test_md_streaming_mid_line_immediate_output() {
        // Simulate LLM streaming: first token starts a line, subsequent tokens mid-line
        let mut r = MarkdownRenderer::new();
        // First token: "Hello " — at line start, long enough to resolve as normal text
        let out1 = r.render_delta("Hello ");
        // Should produce output (6 chars, clearly not a fence or header)
        assert!(
            out1.contains("Hello "),
            "Expected immediate output for non-fence/non-header text, got: '{out1}'"
        );

        // Second token: "world" — mid-line, should be immediate
        let out2 = r.render_delta("world");
        assert!(
            out2.contains("world"),
            "Mid-line delta should produce immediate output, got: '{out2}'"
        );

        // Third token: " how" — still mid-line
        let out3 = r.render_delta(" how");
        assert!(
            out3.contains(" how"),
            "Mid-line delta should produce immediate output, got: '{out3}'"
        );
    }

    #[test]
    fn test_md_streaming_newline_resets_to_line_start() {
        let mut r = MarkdownRenderer::new();
        // Start with text that resolves line start
        let _ = r.render_delta("Hello world");
        // Now a newline — next delta should be at line start again
        let _ = r.render_delta("\n");
        // Short text at start of new line — should buffer briefly
        let out = r.render_delta("``");
        // Two backticks could be start of a fence — should buffer
        assert_eq!(
            out, "",
            "Short ambiguous text at line start should be buffered"
        );
    }

    #[test]
    fn test_md_streaming_code_fence_detected_at_line_start() {
        let mut r = MarkdownRenderer::new();
        // Send a code fence at line start — shows ┌─ border
        let out1 = r.render_delta("```\n");
        assert!(out1.contains("┌─"), "opening fence should show ┌─");
        assert!(r.in_code_block);

        // Content inside code block — has │ left border
        let out2 = r.render_delta("some code\n");
        assert!(out2.contains("some code"));
        assert!(out2.contains("│"), "code content should have │ border");

        // Closing fence — shows └─
        let out3 = r.render_delta("```\n");
        assert!(out3.contains("└─"), "closing fence should show └─");
        assert!(!r.in_code_block);
    }

    #[test]
    fn test_md_streaming_header_detected_at_line_start() {
        let mut r = MarkdownRenderer::new();
        // Header at line start
        let out = r.render_delta("# My Header\n");
        assert!(out.contains(&format!("{BOLD}{CYAN}# My Header{RESET}")));
    }

    #[test]
    fn test_md_streaming_bold_mid_line() {
        let mut r = MarkdownRenderer::new();
        // Start a line with enough text to resolve
        let out1 = r.render_delta("This is ");
        assert!(out1.contains("This is "));
        // Now bold text mid-line
        let out2 = r.render_delta("**important**");
        assert!(
            out2.contains(&format!("{BOLD}important{RESET}")),
            "Bold formatting should work in mid-line streaming, got: '{out2}'"
        );
    }

    #[test]
    fn test_md_streaming_inline_code_mid_line() {
        let mut r = MarkdownRenderer::new();
        // Start a line
        let out1 = r.render_delta("Use the ");
        assert!(out1.contains("Use the "));
        // Inline code mid-line
        let out2 = r.render_delta("`Option`");
        assert!(
            out2.contains(&format!("{CYAN}Option{RESET}")),
            "Inline code should work in mid-line streaming, got: '{out2}'"
        );
    }

    #[test]
    fn test_md_streaming_word_by_word_paragraph() {
        // Simulate typical LLM streaming: word by word
        let mut r = MarkdownRenderer::new();
        let words = ["The ", "quick ", "brown ", "fox ", "jumps"];
        let mut got_output = false;
        for word in &words[..] {
            let out = r.render_delta(word);
            if !out.is_empty() {
                got_output = true;
            }
        }
        // We should have gotten SOME output before the line ends
        assert!(
            got_output,
            "Word-by-word streaming should produce output before newline"
        );

        // Flush remainder
        let _flushed = r.flush();
        // Total output should contain all words
        let mut total = String::new();
        let mut r2 = MarkdownRenderer::new();
        for word in &words[..] {
            total.push_str(&r2.render_delta(word));
        }
        total.push_str(&r2.flush());
        assert!(total.contains("The "));
        assert!(total.contains("fox "));
    }

    #[test]
    fn test_md_streaming_line_start_buffer_short_text() {
        // At line start, very short text (1-3 chars) that could be start of fence/header
        // should be buffered
        let mut r = MarkdownRenderer::new();
        let out = r.render_delta("#");
        // Single '#' could be a header — should buffer
        assert_eq!(out, "", "Single '#' at line start should be buffered");

        // Now add more to reveal it's a header
        let out2 = r.render_delta(" Title\n");
        assert!(out2.contains(&format!("{BOLD}{CYAN}# Title{RESET}")));
    }

    #[test]
    fn test_md_streaming_line_start_resolves_normal() {
        // At line start, text that quickly resolves as not a fence/header
        let mut r = MarkdownRenderer::new();
        let out = r.render_delta("Normal text");
        // "Normal" is 11 chars, clearly not a fence or header — should output
        assert!(
            out.contains("Normal text"),
            "Non-fence/non-header text should be output once resolved, got: '{out}'"
        );
    }

    #[test]
    fn test_md_streaming_existing_tests_still_pass() {
        // Ensure the full-line render_full helper still works exactly as before
        let out = render_full("Hello **world** and `code`\n");
        assert!(out.contains("Hello "));
        assert!(out.contains(&format!("{BOLD}world{RESET}")));
        assert!(out.contains(&format!("{CYAN}code{RESET}")));
    }

    #[test]
    fn test_md_streaming_in_code_block_immediate() {
        // Inside a code block, tokens should still stream
        let mut r = MarkdownRenderer::new();
        let _ = r.render_delta("```rust\n");
        assert!(r.in_code_block);
        // Now send code tokens — inside code block, line-start buffering still applies
        // because we need complete lines for syntax highlighting
        let out = r.render_delta("let x");
        // In a code block we still want to buffer for syntax highlighting accuracy
        // but mid-line tokens should still be immediate when possible
        let flushed = r.flush();
        let total = format!("{out}{flushed}");
        assert!(
            total.contains("let") || total.contains("x"),
            "Code block content should eventually render, got: '{total}'"
        );
    }

    #[test]
    fn test_md_plain_text_unchanged() {
        let out = render_full("just plain text\n");
        assert!(out.contains("just plain text"));
    }

    #[test]
    fn test_md_multiple_inline_codes_one_line() {
        let out = render_full("use `foo` and `bar` here\n");
        assert!(out.contains(&format!("{CYAN}foo{RESET}")));
        assert!(out.contains(&format!("{CYAN}bar{RESET}")));
    }

    #[test]
    fn test_md_code_block_preserves_content() {
        let input = "```\nfn main() {\n    println!(\"hello\");\n}\n```\n";
        let out = render_full(input);
        assert!(out.contains("fn main()"));
        assert!(out.contains("println!"));
    }

    // --- Syntax highlighting tests ---

    #[test]
    fn test_highlight_rust_keywords() {
        let out = highlight_code_line("rust", "    let mut x = 42;");
        assert!(out.contains(&format!("{BOLD_CYAN}let{RESET}")));
        assert!(out.contains(&format!("{BOLD_CYAN}mut{RESET}")));
        assert!(out.contains(&format!("{YELLOW}42{RESET}")));
    }

    #[test]
    fn test_highlight_rust_fn() {
        let out = highlight_code_line("rust", "fn main() {");
        assert!(out.contains(&format!("{BOLD_CYAN}fn{RESET}")));
        assert!(out.contains("main"));
    }

    #[test]
    fn test_highlight_rust_string() {
        let out = highlight_code_line("rs", r#"let s = "hello world";"#);
        assert!(out.contains(&format!("{GREEN}\"hello world\"{RESET}")));
    }

    #[test]
    fn test_highlight_rust_comment() {
        let out = highlight_code_line("rust", "    // this is a comment");
        assert!(out.contains(&format!("{DIM}")));
        assert!(out.contains("this is a comment"));
    }

    #[test]
    fn test_highlight_rust_full_line_comment() {
        let out = highlight_code_line("rust", "// full line comment");
        assert_eq!(out, format!("{DIM}// full line comment{RESET}"));
    }

    #[test]
    fn test_highlight_python_keywords() {
        let out = highlight_code_line("python", "def hello(self):");
        assert!(out.contains(&format!("{BOLD_CYAN}def{RESET}")));
        assert!(out.contains(&format!("{BOLD_CYAN}self{RESET}")));
    }

    #[test]
    fn test_highlight_python_comment() {
        let out = highlight_code_line("py", "# a comment");
        assert_eq!(out, format!("{DIM}# a comment{RESET}"));
    }

    #[test]
    fn test_highlight_js_keywords() {
        let out = highlight_code_line("javascript", "const x = async () => {");
        assert!(out.contains(&format!("{BOLD_CYAN}const{RESET}")));
        assert!(out.contains(&format!("{BOLD_CYAN}async{RESET}")));
    }

    #[test]
    fn test_highlight_ts_alias() {
        let out = highlight_code_line("ts", "let y = 10;");
        assert!(out.contains(&format!("{BOLD_CYAN}let{RESET}")));
        assert!(out.contains(&format!("{YELLOW}10{RESET}")));
    }

    #[test]
    fn test_highlight_go_keywords() {
        let out = highlight_code_line("go", "func main() {");
        assert!(out.contains(&format!("{BOLD_CYAN}func{RESET}")));
    }

    #[test]
    fn test_highlight_shell_keywords() {
        let out = highlight_code_line("bash", "if [ -f file ]; then");
        assert!(out.contains(&format!("{BOLD_CYAN}if{RESET}")));
        assert!(out.contains(&format!("{BOLD_CYAN}then{RESET}")));
    }

    #[test]
    fn test_highlight_shell_comment() {
        let out = highlight_code_line("sh", "# shell comment");
        assert_eq!(out, format!("{DIM}# shell comment{RESET}"));
    }

    #[test]
    fn test_highlight_unknown_lang_falls_back_to_dim() {
        let out = highlight_code_line("haskell", "main = putStrLn");
        assert_eq!(out, format!("{DIM}main = putStrLn{RESET}"));
    }

    #[test]
    fn test_highlight_empty_line() {
        let out = highlight_code_line("rust", "");
        assert_eq!(out, "");
    }

    #[test]
    fn test_highlight_no_false_keyword_in_identifier() {
        // "letter" contains "let" but should NOT be highlighted
        let out = highlight_code_line("rust", "let letter = 1;");
        // "letter" should appear in output
        assert!(out.contains("letter"));
        // When colors are enabled, "let" should be keyword-highlighted
        // but "letter" should NOT be
        let bold_cyan_str = format!("{BOLD_CYAN}");
        if !bold_cyan_str.is_empty() {
            assert!(out.contains(&format!("{BOLD_CYAN}let{RESET}")));
            let letter_highlighted = format!("{BOLD_CYAN}letter{RESET}");
            assert!(!out.contains(&letter_highlighted));
        }
    }

    #[test]
    fn test_highlight_string_with_escape() {
        let out = highlight_code_line("rust", r#"let s = "he\"llo";"#);
        assert!(out.contains(&format!("{GREEN}")));
        assert!(out.contains(&format!("{BOLD_CYAN}let{RESET}")));
    }

    #[test]
    fn test_highlight_inline_comment_after_code() {
        let out = highlight_code_line("rust", "let x = 1; // comment");
        assert!(out.contains(&format!("{BOLD_CYAN}let{RESET}")));
        assert!(out.contains(&format!("{DIM}// comment{RESET}")));
    }

    #[test]
    fn test_highlight_number_float() {
        let out = highlight_code_line("rust", "let pi = 3.14;");
        assert!(out.contains(&format!("{YELLOW}3.14{RESET}")));
    }

    #[test]
    fn test_normalize_lang_aliases() {
        assert_eq!(normalize_lang("rust"), Some("rust"));
        assert_eq!(normalize_lang("rs"), Some("rust"));
        assert_eq!(normalize_lang("Python"), Some("python"));
        assert_eq!(normalize_lang("JS"), Some("js"));
        assert_eq!(normalize_lang("typescript"), Some("js"));
        assert_eq!(normalize_lang("tsx"), Some("js"));
        assert_eq!(normalize_lang("golang"), Some("go"));
        assert_eq!(normalize_lang("zsh"), Some("shell"));
        assert_eq!(normalize_lang("haskell"), None);
    }

    #[test]
    fn test_highlight_renders_through_markdown() {
        // End-to-end: markdown renderer should use highlighting for tagged blocks
        let input = "```rust\nfn main() {\n    return 42;\n}\n```\n";
        let out = render_full(input);
        assert!(out.contains(&format!("{BOLD_CYAN}fn{RESET}")));
        assert!(out.contains(&format!("{BOLD_CYAN}return{RESET}")));
        assert!(out.contains(&format!("{YELLOW}42{RESET}")));
    }

    // --- Rust highlighting: types ---

    #[test]
    fn test_highlight_rust_types() {
        let out = highlight_code_line("rust", "let v: Vec<String> = Vec::new();");
        assert!(out.contains(&format!("{MAGENTA}Vec{RESET}")));
        assert!(out.contains(&format!("{MAGENTA}String{RESET}")));
    }

    #[test]
    fn test_highlight_rust_option_result() {
        let out = highlight_code_line("rust", "fn foo() -> Option<Result<u32, String>> {");
        assert!(out.contains(&format!("{MAGENTA}Option{RESET}")));
        assert!(out.contains(&format!("{MAGENTA}Result{RESET}")));
        assert!(out.contains(&format!("{MAGENTA}u32{RESET}")));
    }

    #[test]
    fn test_highlight_rust_primitive_types() {
        let out = highlight_code_line("rust", "let x: i32 = 0;");
        assert!(out.contains(&format!("{MAGENTA}i32{RESET}")));
        assert!(out.contains(&format!("{YELLOW}0{RESET}")));
    }

    #[test]
    fn test_highlight_rust_self_type() {
        let out = highlight_code_line("rust", "impl Self {");
        assert!(out.contains(&format!("{MAGENTA}Self{RESET}")));
        assert!(out.contains(&format!("{BOLD_CYAN}impl{RESET}")));
    }

    // --- Python highlighting: comprehensive ---

    #[test]
    fn test_highlight_python_string() {
        let out = highlight_code_line("python", "name = \"hello world\"");
        assert!(out.contains(&format!("{GREEN}\"hello world\"{RESET}")));
    }

    #[test]
    fn test_highlight_python_single_quote_string() {
        let out = highlight_code_line("python", "name = 'hello'");
        assert!(out.contains(&format!("{GREEN}'hello'{RESET}")));
    }

    #[test]
    fn test_highlight_python_inline_comment() {
        let out = highlight_code_line("python", "x = 1  # set x");
        assert!(out.contains(&format!("{YELLOW}1{RESET}")));
        assert!(out.contains(&format!("{DIM}")));
        assert!(out.contains("set x"));
    }

    #[test]
    fn test_highlight_python_class_def() {
        let out = highlight_code_line("python", "class MyClass(Base):");
        assert!(out.contains(&format!("{BOLD_CYAN}class{RESET}")));
        assert!(out.contains("MyClass"));
    }

    #[test]
    fn test_highlight_python_boolean_none() {
        let out = highlight_code_line("python", "if True and not None:");
        assert!(out.contains(&format!("{BOLD_CYAN}True{RESET}")));
        assert!(out.contains(&format!("{BOLD_CYAN}None{RESET}")));
        assert!(out.contains(&format!("{BOLD_CYAN}not{RESET}")));
    }

    #[test]
    fn test_highlight_python_import() {
        let out = highlight_code_line("python", "from os import path");
        assert!(out.contains(&format!("{BOLD_CYAN}from{RESET}")));
        assert!(out.contains(&format!("{BOLD_CYAN}import{RESET}")));
    }

    // --- JavaScript/TypeScript highlighting: comprehensive ---

    #[test]
    fn test_highlight_js_function_declaration() {
        let out = highlight_code_line("js", "function hello() {");
        assert!(out.contains(&format!("{BOLD_CYAN}function{RESET}")));
    }

    #[test]
    fn test_highlight_js_string_template() {
        let out = highlight_code_line("javascript", "const msg = \"hello\";");
        assert!(out.contains(&format!("{BOLD_CYAN}const{RESET}")));
        assert!(out.contains(&format!("{GREEN}\"hello\"{RESET}")));
    }

    #[test]
    fn test_highlight_js_null_undefined() {
        let out = highlight_code_line("js", "if (x === null || y === undefined) {");
        assert!(out.contains(&format!("{BOLD_CYAN}null{RESET}")));
        assert!(out.contains(&format!("{BOLD_CYAN}undefined{RESET}")));
    }

    #[test]
    fn test_highlight_js_comment() {
        let out = highlight_code_line("js", "// this is a JS comment");
        assert_eq!(out, format!("{DIM}// this is a JS comment{RESET}"));
    }

    #[test]
    fn test_highlight_tsx_recognized() {
        let out = highlight_code_line("tsx", "const App = () => {");
        assert!(out.contains(&format!("{BOLD_CYAN}const{RESET}")));
    }

    // --- Shell highlighting: comprehensive ---

    #[test]
    fn test_highlight_shell_for_loop() {
        let out = highlight_code_line("bash", "for f in *.txt; do");
        assert!(out.contains(&format!("{BOLD_CYAN}for{RESET}")));
        assert!(out.contains(&format!("{BOLD_CYAN}in{RESET}")));
        assert!(out.contains(&format!("{BOLD_CYAN}do{RESET}")));
    }

    #[test]
    fn test_highlight_shell_string() {
        let out = highlight_code_line("shell", "echo \"hello world\"");
        assert!(out.contains(&format!("{BOLD_CYAN}echo{RESET}")));
        assert!(out.contains(&format!("{GREEN}\"hello world\"{RESET}")));
    }

    #[test]
    fn test_highlight_shell_export() {
        let out = highlight_code_line("bash", "export PATH=\"/usr/bin\"");
        assert!(out.contains(&format!("{BOLD_CYAN}export{RESET}")));
    }

    #[test]
    fn test_highlight_zsh_recognized() {
        let out = highlight_code_line("zsh", "if [ -f file ]; then");
        assert!(out.contains(&format!("{BOLD_CYAN}if{RESET}")));
    }

    // --- C/C++ highlighting ---

    #[test]
    fn test_highlight_c_keywords() {
        let out = highlight_code_line("c", "int main() {");
        assert!(out.contains(&format!("{BOLD_CYAN}int{RESET}")));
        assert!(out.contains("main"));
    }

    #[test]
    fn test_highlight_cpp_keywords() {
        let out = highlight_code_line("cpp", "class Foo : public Bar {");
        assert!(out.contains(&format!("{BOLD_CYAN}class{RESET}")));
        assert!(out.contains(&format!("{BOLD_CYAN}public{RESET}")));
    }

    #[test]
    fn test_highlight_c_comment() {
        let out = highlight_code_line("c", "// C comment");
        assert_eq!(out, format!("{DIM}// C comment{RESET}"));
    }

    #[test]
    fn test_highlight_c_string() {
        let out = highlight_code_line("c", "char *s = \"hello\";");
        assert!(out.contains(&format!("{GREEN}\"hello\"{RESET}")));
    }

    #[test]
    fn test_highlight_c_types() {
        let out = highlight_code_line("c", "size_t len = strlen(s);");
        assert!(out.contains(&format!("{MAGENTA}size_t{RESET}")));
    }

    #[test]
    fn test_highlight_hpp_recognized() {
        let out = highlight_code_line("hpp", "namespace foo {");
        assert!(out.contains(&format!("{BOLD_CYAN}namespace{RESET}")));
    }

    // --- Go highlighting: types ---

    #[test]
    fn test_highlight_go_types() {
        let out = highlight_code_line("go", "var x int = 42");
        assert!(out.contains(&format!("{BOLD_CYAN}var{RESET}")));
        assert!(out.contains(&format!("{MAGENTA}int{RESET}")));
        assert!(out.contains(&format!("{YELLOW}42{RESET}")));
    }

    #[test]
    fn test_highlight_go_string_type() {
        let out = highlight_code_line("go", "func greet(name string) error {");
        assert!(out.contains(&format!("{BOLD_CYAN}func{RESET}")));
        assert!(out.contains(&format!("{MAGENTA}string{RESET}")));
        assert!(out.contains(&format!("{MAGENTA}error{RESET}")));
    }

    // --- JSON highlighting ---

    #[test]
    fn test_highlight_json_key_value() {
        let out = highlight_code_line("json", r#"  "name": "yoyo","#);
        assert!(out.contains(&format!("{CYAN}\"name\"{RESET}")));
        assert!(out.contains(&format!("{GREEN}\"yoyo\"{RESET}")));
    }

    #[test]
    fn test_highlight_json_number() {
        let out = highlight_code_line("json", r#"  "count": 42,"#);
        assert!(out.contains(&format!("{CYAN}\"count\"{RESET}")));
        assert!(out.contains(&format!("{YELLOW}42{RESET}")));
    }

    #[test]
    fn test_highlight_json_boolean() {
        let out = highlight_code_line("json", r#"  "active": true,"#);
        assert!(out.contains(&format!("{BOLD_CYAN}true{RESET}")));
    }

    #[test]
    fn test_highlight_json_null() {
        let out = highlight_code_line("json", r#"  "value": null"#);
        assert!(out.contains(&format!("{BOLD_CYAN}null{RESET}")));
    }

    #[test]
    fn test_highlight_json_braces() {
        // Braces and brackets should pass through
        let out = highlight_code_line("json", "  {");
        assert!(out.contains('{'));
    }

    #[test]
    fn test_highlight_jsonc_recognized() {
        let out = highlight_code_line("jsonc", r#"  "key": "value""#);
        assert!(out.contains(&format!("{CYAN}\"key\"{RESET}")));
    }

    // --- YAML highlighting ---

    #[test]
    fn test_highlight_yaml_key_value() {
        let out = highlight_code_line("yaml", "name: yoyo");
        assert!(out.contains(&format!("{BOLD_YELLOW}name{RESET}")));
    }

    #[test]
    fn test_highlight_yaml_string_value() {
        let out = highlight_code_line("yaml", "name: \"yoyo\"");
        assert!(out.contains(&format!("{BOLD_YELLOW}name{RESET}")));
        assert!(out.contains(&format!("{GREEN}\"yoyo\"{RESET}")));
    }

    #[test]
    fn test_highlight_yaml_boolean() {
        let out = highlight_code_line("yaml", "enabled: true");
        assert!(out.contains(&format!("{BOLD_CYAN}true{RESET}")));
    }

    #[test]
    fn test_highlight_yaml_number() {
        let out = highlight_code_line("yaml", "port: 8080");
        assert!(out.contains(&format!("{YELLOW}8080{RESET}")));
    }

    #[test]
    fn test_highlight_yaml_comment() {
        let out = highlight_code_line("yml", "# a yaml comment");
        assert_eq!(out, format!("{DIM}# a yaml comment{RESET}"));
    }

    #[test]
    fn test_highlight_yaml_document_separator() {
        let out = highlight_code_line("yaml", "---");
        assert!(out.contains(&format!("{DIM}---{RESET}")));
    }

    #[test]
    fn test_highlight_yml_alias() {
        // "yml" should be recognized as yaml
        assert_eq!(normalize_lang("yml"), Some("yaml"));
    }

    // --- TOML highlighting ---

    #[test]
    fn test_highlight_toml_section() {
        let out = highlight_code_line("toml", "[package]");
        assert!(out.contains(&format!("{BOLD}{CYAN}[package]{RESET}")));
    }

    #[test]
    fn test_highlight_toml_key_string() {
        let out = highlight_code_line("toml", "name = \"yoyo\"");
        assert!(out.contains(&format!("{BOLD_YELLOW}name{RESET}")));
        assert!(out.contains(&format!("{GREEN}\"yoyo\"{RESET}")));
    }

    #[test]
    fn test_highlight_toml_key_number() {
        let out = highlight_code_line("toml", "version = 1");
        assert!(out.contains(&format!("{BOLD_YELLOW}version{RESET}")));
        assert!(out.contains(&format!("{YELLOW}1{RESET}")));
    }

    #[test]
    fn test_highlight_toml_boolean() {
        let out = highlight_code_line("toml", "enabled = true");
        assert!(out.contains(&format!("{BOLD_CYAN}true{RESET}")));
    }

    #[test]
    fn test_highlight_toml_comment() {
        let out = highlight_code_line("toml", "# a toml comment");
        assert_eq!(out, format!("{DIM}# a toml comment{RESET}"));
    }

    #[test]
    fn test_highlight_toml_array_section() {
        let out = highlight_code_line("toml", "[[bin]]");
        assert!(out.contains(&format!("{BOLD}{CYAN}[[bin]]{RESET}")));
    }

    // --- normalize_lang expanded aliases ---

    #[test]
    fn test_normalize_lang_c_family() {
        assert_eq!(normalize_lang("c"), Some("c"));
        assert_eq!(normalize_lang("cpp"), Some("c"));
        assert_eq!(normalize_lang("c++"), Some("c"));
        assert_eq!(normalize_lang("cc"), Some("c"));
        assert_eq!(normalize_lang("h"), Some("c"));
        assert_eq!(normalize_lang("hpp"), Some("c"));
    }

    #[test]
    fn test_normalize_lang_data_formats() {
        assert_eq!(normalize_lang("json"), Some("json"));
        assert_eq!(normalize_lang("jsonc"), Some("json"));
        assert_eq!(normalize_lang("yaml"), Some("yaml"));
        assert_eq!(normalize_lang("yml"), Some("yaml"));
        assert_eq!(normalize_lang("toml"), Some("toml"));
    }

    // --- End-to-end through MarkdownRenderer ---

    #[test]
    fn test_highlight_json_through_markdown() {
        let input = "```json\n{\"name\": \"yoyo\"}\n```\n";
        let out = render_full(input);
        assert!(out.contains(&format!("{CYAN}\"name\"{RESET}")));
        assert!(out.contains(&format!("{GREEN}\"yoyo\"{RESET}")));
    }

    #[test]
    fn test_highlight_yaml_through_markdown() {
        let input = "```yaml\nname: yoyo\n```\n";
        let out = render_full(input);
        assert!(out.contains(&format!("{BOLD_YELLOW}name{RESET}")));
    }

    #[test]
    fn test_highlight_toml_through_markdown() {
        let input = "```toml\n[package]\nname = \"yoyo\"\n```\n";
        let out = render_full(input);
        assert!(out.contains(&format!("{BOLD}{CYAN}[package]{RESET}")));
        assert!(out.contains(&format!("{GREEN}\"yoyo\"{RESET}")));
    }

    #[test]
    fn test_highlight_c_through_markdown() {
        let input = "```c\nint main() {\n    return 0;\n}\n```\n";
        let out = render_full(input);
        assert!(out.contains(&format!("{BOLD_CYAN}int{RESET}")));
        assert!(out.contains(&format!("{BOLD_CYAN}return{RESET}")));
        assert!(out.contains(&format!("{YELLOW}0{RESET}")));
    }

    // --- Spinner tests ---

    #[test]
    fn test_spinner_frames_not_empty() {
        assert!(!SPINNER_FRAMES.is_empty());
    }

    #[test]
    fn test_spinner_frames_are_braille() {
        // All braille characters are in the Unicode range U+2800..U+28FF
        for &frame in SPINNER_FRAMES {
            assert!(
                ('\u{2800}'..='\u{28FF}').contains(&frame),
                "Expected braille character, got {:?}",
                frame
            );
        }
    }

    #[test]
    fn test_spinner_frame_cycling() {
        // First 10 frames should match SPINNER_FRAMES exactly
        for (i, &expected) in SPINNER_FRAMES.iter().enumerate() {
            assert_eq!(spinner_frame(i), expected);
        }
    }

    #[test]
    fn test_spinner_frame_wraps_around() {
        let len = SPINNER_FRAMES.len();
        // After one full cycle, it should repeat
        assert_eq!(spinner_frame(0), spinner_frame(len));
        assert_eq!(spinner_frame(1), spinner_frame(len + 1));
        assert_eq!(spinner_frame(2), spinner_frame(len + 2));
    }

    #[test]
    fn test_spinner_frame_large_index() {
        // Should not panic even with very large indices
        let frame = spinner_frame(999_999);
        assert!(SPINNER_FRAMES.contains(&frame));
    }

    #[test]
    fn test_spinner_frames_all_unique() {
        // Each frame in the animation should be distinct
        let mut seen = std::collections::HashSet::new();
        for &frame in SPINNER_FRAMES {
            assert!(seen.insert(frame), "Duplicate spinner frame: {:?}", frame);
        }
    }

    // --- format_edit_diff tests ---

    #[test]
    fn test_format_edit_diff_single_line_change() {
        let diff = format_edit_diff("old line", "new line");
        assert!(diff.contains("- old line"));
        assert!(diff.contains("+ new line"));
        // Should have red for removed, green for added
        assert!(diff.contains(&format!("{RED}")));
        assert!(diff.contains(&format!("{GREEN}")));
    }

    #[test]
    fn test_format_edit_diff_multi_line_change() {
        let old = "line 1\nline 2\nline 3";
        let new = "line A\nline B";
        let diff = format_edit_diff(old, new);
        assert!(diff.contains("- line 1"));
        assert!(diff.contains("- line 2"));
        assert!(diff.contains("- line 3"));
        assert!(diff.contains("+ line A"));
        assert!(diff.contains("+ line B"));
    }

    #[test]
    fn test_format_edit_diff_addition_only() {
        let diff = format_edit_diff("", "new content\nmore content");
        // No removed lines
        assert!(!diff.contains("- "));
        // Added lines present
        assert!(diff.contains("+ new content"));
        assert!(diff.contains("+ more content"));
    }

    #[test]
    fn test_format_edit_diff_deletion_only() {
        let diff = format_edit_diff("old content\nmore old", "");
        // Removed lines present
        assert!(diff.contains("- old content"));
        assert!(diff.contains("- more old"));
        // No added lines
        assert!(!diff.contains("+ "));
    }

    #[test]
    fn test_format_edit_diff_long_diff_truncation() {
        // Generate a diff with more than MAX_DIFF_LINES lines
        let old_lines: Vec<&str> = (0..15).map(|_| "old").collect();
        let new_lines: Vec<&str> = (0..15).map(|_| "new").collect();
        let old = old_lines.join("\n");
        let new = new_lines.join("\n");
        let diff = format_edit_diff(&old, &new);
        // Should be truncated — total would be 30 lines, max is 20
        assert!(diff.contains("more lines)"));
    }

    #[test]
    fn test_format_edit_diff_empty_both() {
        let diff = format_edit_diff("", "");
        assert!(diff.is_empty());
    }

    #[test]
    fn test_format_edit_diff_empty_old_text_new_file_section() {
        // Simulates adding new content to a file (old_text is empty)
        let diff = format_edit_diff("", "fn new_function() {\n    println!(\"hello\");\n}");
        assert!(!diff.contains("- "));
        assert!(diff.contains("+ fn new_function()"));
        assert!(diff.contains("+ }"));
    }

    #[test]
    fn test_format_edit_diff_short_diff_not_truncated() {
        let diff = format_edit_diff("a", "b");
        assert!(!diff.contains("more lines"));
    }

    // --- format_tool_summary write_file with line count ---

    #[test]
    fn test_format_tool_summary_write_file_with_content() {
        let args = serde_json::json!({"path": "out.txt", "content": "line1\nline2\nline3"});
        let result = format_tool_summary("write_file", &args);
        assert_eq!(result, "write out.txt (3 lines)");
    }

    #[test]
    fn test_format_tool_summary_write_file_single_line() {
        let args = serde_json::json!({"path": "out.txt", "content": "hello"});
        let result = format_tool_summary("write_file", &args);
        assert_eq!(result, "write out.txt (1 lines)");
    }

    #[test]
    fn test_format_tool_summary_write_file_no_content() {
        let args = serde_json::json!({"path": "out.txt"});
        let result = format_tool_summary("write_file", &args);
        assert_eq!(result, "write out.txt");
    }
}
