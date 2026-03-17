//! IDE Bridge — routes LLM requests through the host coding agent's subscription.
//!
//! When `--provider ide` is used, yoyo detects the host coding agent and routes
//! LLM requests through its existing subscription. No separate API key needed.
//!
//! Detection priority:
//!   1. Claude Code session token (CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR or
//!      session ingress token file) — direct API call using host's auth
//!   2. `claude -p "prompt"` CLI — subprocess call to Claude Code
//!   3. Custom command via --ide-cmd
//!
//! When a session token is found, yoyo skips the bridge entirely and configures
//! yoagent to call the Anthropic API directly using the host's credentials.

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

/// The result of IDE detection — either direct API credentials or a CLI backend.
#[derive(Debug, Clone)]
pub enum IdeDetection {
    /// Found session credentials — proxy to Anthropic API with Bearer auth.
    DirectApi {
        api_key: String,
        proxy: Option<String>,
    },
    /// Use a CLI subprocess backend.
    CliBackend(IdeBackend),
}

/// Credentials for direct API calls through the host's session.
#[derive(Debug, Clone)]
pub struct SessionCreds {
    pub bearer_token: String,
    pub proxy_url: Option<String>,
}

/// Which IDE CLI backend to use for subprocess calls.
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

/// Detect the host IDE environment.
/// Returns the best available method for routing LLM requests.
pub fn detect_ide() -> Option<IdeDetection> {
    // Priority 1: Check for Claude Code session ingress token file
    if let Ok(token_path) = std::env::var("CLAUDE_SESSION_INGRESS_TOKEN_FILE") {
        if let Ok(token) = std::fs::read_to_string(&token_path) {
            let token = token.trim().to_string();
            if !token.is_empty() {
                let proxy = std::env::var("GLOBAL_AGENT_HTTP_PROXY").ok();
                return Some(IdeDetection::DirectApi {
                    api_key: token,
                    proxy,
                });
            }
        }
    }

    // Priority 2: Check for Claude Code CLI
    if which("claude") {
        // Test if it's actually logged in
        let test = std::process::Command::new("claude")
            .args(["-p", "test"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output();

        match test {
            Ok(output) if output.status.success() => {
                return Some(IdeDetection::CliBackend(IdeBackend::ClaudeCode));
            }
            _ => {
                // claude exists but isn't logged in — check for session token as fallback
                // (already checked above, so this is a dead end)
            }
        }

        // Even if not logged in, the CLI existing means we're in a Claude Code env.
        // Return it anyway — the error message will be clearer.
        return Some(IdeDetection::CliBackend(IdeBackend::ClaudeCode));
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

/// Run a prompt through the IDE CLI backend and return the response text.
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

/// Call the Anthropic Messages API directly using the host's session credentials.
/// Uses curl to handle the proxy and Bearer auth properly.
fn call_anthropic_api(creds: &SessionCreds, prompt: &str, model: &str) -> Result<String, String> {
    let body = serde_json::json!({
        "model": model,
        "max_tokens": 8192,
        "messages": [{"role": "user", "content": prompt}]
    });

    let mut cmd = std::process::Command::new("curl");
    cmd.args([
        "-s",
        "https://api.anthropic.com/v1/messages",
        "-H",
        "content-type: application/json",
        "-H",
        &format!("Authorization: Bearer {}", creds.bearer_token),
        "-H",
        "anthropic-version: 2023-06-01",
        "-d",
        &serde_json::to_string(&body).unwrap_or_default(),
    ]);

    if let Some(ref proxy) = creds.proxy_url {
        cmd.args(["-x", proxy]);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to call Anthropic API: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("curl failed: {stderr}"));
    }

    let response_str = String::from_utf8_lossy(&output.stdout);
    let response: serde_json::Value =
        serde_json::from_str(&response_str).map_err(|e| format!("JSON parse error: {e}"))?;

    // Check for API error
    if let Some(error) = response.get("error") {
        let msg = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        return Err(format!("API error: {msg}"));
    }

    // Extract text from Anthropic response format
    let text = response
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("");

    Ok(text.to_string())
}

/// Start the IDE bridge server using session credentials (Bearer auth → Anthropic API).
/// Returns the port it's listening on.
pub async fn start_api_bridge(creds: SessionCreds) -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Failed to bind bridge: {e}"))?;

    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed to get bridge address: {e}"))?
        .port();

    let creds = Arc::new(creds);

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => continue,
            };
            let creds = Arc::clone(&creds);
            tokio::spawn(async move {
                let _ = handle_api_bridge_request(stream, &creds).await;
            });
        }
    });

    Ok(port)
}

/// Handle a single request on the API bridge (session credentials path).
async fn handle_api_bridge_request(
    mut stream: tokio::net::TcpStream,
    creds: &SessionCreds,
) -> Result<(), String> {
    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);

    let mut request_line = String::new();
    buf_reader
        .read_line(&mut request_line)
        .await
        .map_err(|e| format!("read error: {e}"))?;

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

    if *path == "/v1/models" || *path == "/models" {
        let body =
            r#"{"object":"list","data":[{"id":"ide-bridge","object":"model","owned_by":"ide"}]}"#;
        return send_response(&mut writer, 200, body).await;
    }

    if (*path == "/v1/chat/completions" || *path == "/chat/completions") && *method == "POST" {
        let mut body = vec![0u8; content_length];
        buf_reader
            .read_exact(&mut body)
            .await
            .map_err(|e| format!("body read error: {e}"))?;

        let body_str = String::from_utf8_lossy(&body);
        let request: serde_json::Value =
            serde_json::from_str(&body_str).map_err(|e| format!("JSON parse error: {e}"))?;

        let prompt = extract_prompt_from_messages(&request);
        if prompt.is_empty() {
            return send_response(
                &mut writer,
                400,
                r#"{"error":{"message":"No user message found"}}"#,
            )
            .await;
        }

        // Extract model from request, default to sonnet
        let model = request
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("claude-sonnet-4-20250514")
            .to_string();

        let creds_clone = creds.clone();
        let prompt_clone = prompt.clone();
        let result = tokio::task::spawn_blocking(move || {
            call_anthropic_api(&creds_clone, &prompt_clone, &model)
        })
        .await
        .map_err(|e| format!("spawn error: {e}"))?;

        match result {
            Ok(response_text) => {
                return send_sse_response(&mut writer, &response_text).await;
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

    send_response(&mut writer, 404, r#"{"error":{"message":"Not found"}}"#).await
}

/// Start the IDE bridge server for CLI backends. Returns the port it's listening on.
/// The server runs in the background as a tokio task.
pub async fn start_bridge(backend: IdeBackend) -> Result<u16, String> {
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

    let mut request_line = String::new();
    buf_reader
        .read_line(&mut request_line)
        .await
        .map_err(|e| format!("read error: {e}"))?;

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

    if *path == "/v1/models" || *path == "/models" {
        let body =
            r#"{"object":"list","data":[{"id":"ide-bridge","object":"model","owned_by":"ide"}]}"#;
        return send_response(&mut writer, 200, body).await;
    }

    if (*path == "/v1/chat/completions" || *path == "/chat/completions") && *method == "POST" {
        let mut body = vec![0u8; content_length];
        buf_reader
            .read_exact(&mut body)
            .await
            .map_err(|e| format!("body read error: {e}"))?;

        let body_str = String::from_utf8_lossy(&body);
        let request: serde_json::Value =
            serde_json::from_str(&body_str).map_err(|e| format!("JSON parse error: {e}"))?;

        let prompt = extract_prompt_from_messages(&request);
        if prompt.is_empty() {
            return send_response(
                &mut writer,
                400,
                r#"{"error":{"message":"No user message found"}}"#,
            )
            .await;
        }

        let backend_clone = backend.clone();
        let prompt_clone = prompt.clone();
        let result =
            tokio::task::spawn_blocking(move || run_ide_prompt(&backend_clone, &prompt_clone))
                .await
                .map_err(|e| format!("spawn error: {e}"))?;

        match result {
            Ok(response_text) => {
                return send_sse_response(&mut writer, &response_text).await;
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

/// Send an SSE (Server-Sent Events) streaming response compatible with OpenAI's format.
/// This is required because yoagent's OpenAiCompatProvider uses EventSource (SSE).
async fn send_sse_response(
    writer: &mut (impl AsyncWriteExt + Unpin),
    text: &str,
) -> Result<(), String> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let id = format!("ide-{}", ts.as_millis());

    // SSE headers
    let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\nAccess-Control-Allow-Origin: *\r\n\r\n";
    writer
        .write_all(header.as_bytes())
        .await
        .map_err(|e| format!("write error: {e}"))?;

    // Send one chunk with the full content
    let chunk = serde_json::json!({
        "id": id,
        "object": "chat.completion.chunk",
        "created": ts.as_secs(),
        "model": "ide-bridge",
        "choices": [{
            "index": 0,
            "delta": {"role": "assistant", "content": text},
            "finish_reason": null
        }]
    });
    let chunk_str = serde_json::to_string(&chunk).unwrap_or_default();
    writer
        .write_all(format!("data: {chunk_str}\n\n").as_bytes())
        .await
        .map_err(|e| format!("write error: {e}"))?;

    // Send finish chunk with usage
    let finish = serde_json::json!({
        "id": id,
        "object": "chat.completion.chunk",
        "created": ts.as_secs(),
        "model": "ide-bridge",
        "choices": [{
            "index": 0,
            "delta": {},
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0
        }
    });
    let finish_str = serde_json::to_string(&finish).unwrap_or_default();
    writer
        .write_all(format!("data: {finish_str}\n\n").as_bytes())
        .await
        .map_err(|e| format!("write error: {e}"))?;

    // Send [DONE] marker
    writer
        .write_all(b"data: [DONE]\n\n")
        .await
        .map_err(|e| format!("write error: {e}"))?;

    Ok(())
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
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
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
    fn test_extract_prompt_no_messages_key() {
        let req = serde_json::json!({"prompt": "test"});
        assert_eq!(extract_prompt_from_messages(&req), "");
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
        assert!(which("sh"));
    }
}
