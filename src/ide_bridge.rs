//! IDE Bridge — routes LLM requests through the host coding agent's CLI.
//!
//! When `--provider ide` is used, yoyo starts a tiny local HTTP server that
//! translates OpenAI-compatible API requests into subprocess calls to the host
//! IDE's CLI (e.g., `claude -p "prompt"`). yoagent's `OpenAiCompatProvider`
//! connects to this bridge, so all LLM calls go through the IDE's subscription
//! — no separate API key or billing needed.
//!
//! Supported host IDEs:
//!   - Claude Code:  `claude -p "prompt"`
//!   - Aider:        `aider --message "prompt" --no-auto-commits`
//!   - GitHub Copilot: `gh copilot suggest "prompt"`
//!
//! Flow:
//!   yoyo agent → OpenAiCompatProvider → http://127.0.0.1:PORT/v1/chat/completions
//!   → ide_bridge extracts prompt → subprocess `claude -p "..."` → response → JSON

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

/// Which IDE CLI backend to use.
#[derive(Debug, Clone)]
pub enum IdeBackend {
    /// `claude -p "prompt"` — Claude Code CLI
    ClaudeCode,
    /// Custom command template. `{}` is replaced with the prompt.
    #[allow(dead_code)]
    Custom(String),
}

impl std::fmt::Display for IdeBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdeBackend::ClaudeCode => write!(f, "claude -p"),
            IdeBackend::Custom(cmd) => write!(f, "{cmd}"),
        }
    }
}

/// Detect which IDE CLI is available on this system.
/// Returns the first detected backend, or None.
pub fn detect_ide() -> Option<IdeBackend> {
    // Check for Claude Code CLI
    if which("claude") {
        return Some(IdeBackend::ClaudeCode);
    }
    None
}

/// Check if a command exists on PATH.
fn which(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run a prompt through the IDE backend and return the response text.
pub fn run_ide_prompt(backend: &IdeBackend, prompt: &str) -> Result<String, String> {
    match backend {
        IdeBackend::ClaudeCode => {
            let output = std::process::Command::new("claude")
                .args(["-p", prompt])
                .output()
                .map_err(|e| format!("Failed to run claude: {e}"))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("claude -p failed: {stderr}"));
            }

            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        IdeBackend::Custom(cmd_template) => {
            let cmd = cmd_template.replace("{}", prompt);
            let output = std::process::Command::new("sh")
                .args(["-c", &cmd])
                .output()
                .map_err(|e| format!("Failed to run custom command: {e}"))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Custom IDE command failed: {stderr}"));
            }

            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
    }
}

/// Start the IDE bridge server. Returns the port it's listening on.
/// The server runs in the background as a tokio task.
pub async fn start_bridge(backend: IdeBackend) -> Result<u16, String> {
    // Bind to a random available port
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Failed to bind bridge: {e}"))?;

    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed to get bridge address: {e}"))?
        .port();

    let backend = Arc::new(backend);

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => continue,
            };
            let backend = Arc::clone(&backend);
            tokio::spawn(async move {
                let _ = handle_bridge_request(stream, &backend).await;
            });
        }
    });

    Ok(port)
}

/// Handle a single HTTP request on the bridge.
async fn handle_bridge_request(
    mut stream: tokio::net::TcpStream,
    backend: &IdeBackend,
) -> Result<(), String> {
    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);

    // Read request line
    let mut request_line = String::new();
    buf_reader
        .read_line(&mut request_line)
        .await
        .map_err(|e| format!("read error: {e}"))?;

    // Read headers to get content-length
    let mut content_length: usize = 0;
    loop {
        let mut header_line = String::new();
        buf_reader
            .read_line(&mut header_line)
            .await
            .map_err(|e| format!("header read error: {e}"))?;
        let trimmed = header_line.trim();
        if trimmed.is_empty() {
            break;
        }
        let lower = trimmed.to_lowercase();
        if let Some(val) = lower.strip_prefix("content-length:") {
            content_length = val.trim().parse().unwrap_or(0);
        }
    }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    let method = parts.first().unwrap_or(&"");
    let path = parts.get(1).unwrap_or(&"");

    // Models endpoint — return a fake model list
    if *path == "/v1/models" || *path == "/models" {
        let body =
            r#"{"object":"list","data":[{"id":"ide-bridge","object":"model","owned_by":"ide"}]}"#;
        return send_response(&mut writer, 200, body).await;
    }

    // Chat completions — the main endpoint
    if (*path == "/v1/chat/completions" || *path == "/chat/completions") && *method == "POST" {
        let mut body = vec![0u8; content_length];
        buf_reader
            .read_exact(&mut body)
            .await
            .map_err(|e| format!("body read error: {e}"))?;

        let body_str = String::from_utf8_lossy(&body);
        let request: serde_json::Value =
            serde_json::from_str(&body_str).map_err(|e| format!("JSON parse error: {e}"))?;

        // Extract prompt from messages
        let prompt = extract_prompt_from_messages(&request);
        if prompt.is_empty() {
            return send_response(
                &mut writer,
                400,
                r#"{"error":{"message":"No user message found"}}"#,
            )
            .await;
        }

        // Run through the IDE CLI (blocking — spawn_blocking to avoid holding async)
        let backend_clone = backend.clone();
        let prompt_clone = prompt.clone();
        let result =
            tokio::task::spawn_blocking(move || run_ide_prompt(&backend_clone, &prompt_clone))
                .await
                .map_err(|e| format!("spawn error: {e}"))?;

        match result {
            Ok(response_text) => {
                let completion = format_completion_json(&response_text);
                return send_response(&mut writer, 200, &completion).await;
            }
            Err(e) => {
                let error_body = serde_json::json!({
                    "error": {"message": e, "type": "ide_bridge_error"}
                });
                return send_response(
                    &mut writer,
                    500,
                    &serde_json::to_string(&error_body).unwrap_or_default(),
                )
                .await;
            }
        }
    }

    // Unknown endpoint
    send_response(&mut writer, 404, r#"{"error":{"message":"Not found"}}"#).await
}

/// Extract a prompt string from OpenAI-format chat messages.
fn extract_prompt_from_messages(request: &serde_json::Value) -> String {
    let messages = match request.get("messages").and_then(|m| m.as_array()) {
        Some(m) => m,
        None => return String::new(),
    };

    let mut parts = Vec::new();
    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
        let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
        match role {
            "system" if !content.is_empty() => {
                parts.push(format!("[System: {}]", content));
            }
            "user" if !content.is_empty() => {
                parts.push(content.to_string());
            }
            "assistant" if !content.is_empty() => {
                parts.push(format!("[Previous response: {}]", content));
            }
            _ => {}
        }
    }
    parts.join("\n\n")
}

/// Format a response as OpenAI-compatible chat completion JSON.
fn format_completion_json(text: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();

    let completion = serde_json::json!({
        "id": format!("ide-{}", ts.as_millis()),
        "object": "chat.completion",
        "created": ts.as_secs(),
        "model": "ide-bridge",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": text
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0
        }
    });

    serde_json::to_string(&completion).unwrap_or_default()
}

/// Send an HTTP response.
async fn send_response(
    writer: &mut (impl AsyncWriteExt + Unpin),
    status: u16,
    body: &str,
) -> Result<(), String> {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\n\
        Content-Type: application/json\r\n\
        Content-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    writer
        .write_all(response.as_bytes())
        .await
        .map_err(|e| format!("write error: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_ide_does_not_panic() {
        // Just verify it runs without panicking
        let _ = detect_ide();
    }

    #[test]
    fn test_which_nonexistent() {
        assert!(!which("this_command_does_not_exist_xyz123"));
    }

    #[test]
    fn test_extract_prompt_basic() {
        let req = serde_json::json!({
            "messages": [
                {"role": "user", "content": "hello world"}
            ]
        });
        assert_eq!(extract_prompt_from_messages(&req), "hello world");
    }

    #[test]
    fn test_extract_prompt_with_system_and_history() {
        let req = serde_json::json!({
            "messages": [
                {"role": "system", "content": "You are helpful"},
                {"role": "user", "content": "question 1"},
                {"role": "assistant", "content": "answer 1"},
                {"role": "user", "content": "question 2"}
            ]
        });
        let prompt = extract_prompt_from_messages(&req);
        assert!(prompt.contains("System: You are helpful"));
        assert!(prompt.contains("question 1"));
        assert!(prompt.contains("Previous response: answer 1"));
        assert!(prompt.contains("question 2"));
    }

    #[test]
    fn test_extract_prompt_empty() {
        let req = serde_json::json!({"messages": []});
        assert_eq!(extract_prompt_from_messages(&req), "");
    }

    #[test]
    fn test_format_completion_json() {
        let json_str = format_completion_json("Hello!");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            parsed["choices"][0]["message"]["content"].as_str().unwrap(),
            "Hello!"
        );
        assert_eq!(parsed["model"].as_str().unwrap(), "ide-bridge");
    }

    #[test]
    fn test_ide_backend_display() {
        assert_eq!(format!("{}", IdeBackend::ClaudeCode), "claude -p");
        assert_eq!(
            format!("{}", IdeBackend::Custom("my-cmd {}".into())),
            "my-cmd {}"
        );
    }

    #[test]
    fn test_which_finds_sh() {
        // sh should exist on any unix system
        assert!(which("sh"));
    }
}
