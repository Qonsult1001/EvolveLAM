//! IDE server mode for yoyo.
//!
//! Exposes a local OpenAI-compatible HTTP endpoint that routes requests through
//! yoyo's configured agent. This lets IDEs (Cursor, VS Code + Continue, etc.)
//! connect to yoyo as a local LLM backend without calling external APIs directly.
//!
//! The agent uses whatever provider is already configured (Anthropic, OpenAI, etc.),
//! so users with existing subscriptions don't need to pay for a separate API key.
//!
//! Usage:
//!   cargo run -- --serve                    # Listen on 127.0.0.1:8787
//!   cargo run -- --serve --port 9000        # Custom port
//!   cargo run -- --serve --provider openai  # Use OpenAI backend

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use yoagent::agent::Agent;

use crate::format::*;
use crate::prompt::run_prompt;
use crate::AgentConfig;

/// Default port for the IDE server.
pub const DEFAULT_PORT: u16 = 8787;

/// Start the IDE server on the given port.
/// Listens for OpenAI-compatible chat completion requests and routes them
/// through yoyo's configured agent.
pub async fn start_server(agent_config: AgentConfig, port: u16) {
    let addr = format!("127.0.0.1:{port}");
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{RED}  Failed to bind to {addr}: {e}{RESET}");
            return;
        }
    };

    println!("{BOLD}yoyo IDE server{RESET}");
    println!("{DIM}  listening: http://{addr}{RESET}");
    println!("{DIM}  provider:  {}{RESET}", agent_config.provider);
    println!("{DIM}  model:     {}{RESET}", agent_config.model);
    println!("{DIM}  endpoint:  POST /v1/chat/completions{RESET}");
    println!("{DIM}  stop:      Ctrl+C{RESET}\n");
    println!("{DIM}  Configure your IDE to use:{RESET}");
    println!("{BOLD}    Base URL: http://{addr}/v1{RESET}");
    println!("{BOLD}    API Key:  any-string-works{RESET}\n");

    let agent = agent_config.build_agent();
    let agent = Arc::new(Mutex::new(agent));
    let config = Arc::new(agent_config);

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{RED}  accept error: {e}{RESET}");
                continue;
            }
        };

        let agent = Arc::clone(&agent);
        let config = Arc::clone(&config);

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, agent, config, peer).await {
                eprintln!("{RED}  connection error: {e}{RESET}");
            }
        });
    }
}

/// Handle a single HTTP connection.
async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    agent: Arc<Mutex<Agent>>,
    config: Arc<AgentConfig>,
    peer: std::net::SocketAddr,
) -> Result<(), String> {
    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);

    // Read request line
    let mut request_line = String::new();
    buf_reader
        .read_line(&mut request_line)
        .await
        .map_err(|e| format!("read error: {e}"))?;

    // Read headers
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
        if let Some(val) = trimmed.strip_prefix("Content-Length:") {
            content_length = val.trim().parse().unwrap_or(0);
        }
        if let Some(val) = trimmed.strip_prefix("content-length:") {
            content_length = val.trim().parse().unwrap_or(0);
        }
    }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    let method = parts.first().unwrap_or(&"");
    let path = parts.get(1).unwrap_or(&"");

    println!("{DIM}  [{peer}] {method} {path}{RESET}");

    // Handle CORS preflight
    if *method == "OPTIONS" {
        let response = "HTTP/1.1 204 No Content\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Access-Control-Allow-Methods: POST, GET, OPTIONS\r\n\
            Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
            Content-Length: 0\r\n\r\n";
        writer
            .write_all(response.as_bytes())
            .await
            .map_err(|e| format!("write error: {e}"))?;
        return Ok(());
    }

    // Health/models endpoint
    if *path == "/v1/models" || *path == "/models" {
        let models_json = format!(
            r#"{{"object":"list","data":[{{"id":"{}","object":"model","owned_by":"yoyo"}}]}}"#,
            config.model
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Type: application/json\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Content-Length: {}\r\n\r\n{}",
            models_json.len(),
            models_json
        );
        writer
            .write_all(response.as_bytes())
            .await
            .map_err(|e| format!("write error: {e}"))?;
        return Ok(());
    }

    // Chat completions endpoint
    if (*path == "/v1/chat/completions" || *path == "/chat/completions") && *method == "POST" {
        // Read body
        let mut body = vec![0u8; content_length];
        buf_reader
            .read_exact(&mut body)
            .await
            .map_err(|e| format!("body read error: {e}"))?;

        let body_str = String::from_utf8_lossy(&body);
        let request: serde_json::Value =
            serde_json::from_str(&body_str).map_err(|e| format!("JSON parse error: {e}"))?;

        // Extract the last user message as the prompt
        let prompt = extract_prompt(&request);
        if prompt.is_empty() {
            return send_error(&mut writer, 400, "No user message found in request").await;
        }

        println!(
            "{DIM}  [{peer}] prompt: {}{RESET}",
            truncate_for_log(&prompt, 80)
        );

        // Run through yoyo's agent
        let mut agent = agent.lock().await;
        let mut usage = yoagent::Usage::default();
        let response_text = run_prompt(&mut agent, &prompt, &mut usage, &config.model).await;

        println!(
            "{DIM}  [{peer}] response: {} chars, {} input + {} output tokens{RESET}",
            response_text.len(),
            usage.input,
            usage.output
        );

        // Format as OpenAI-compatible response
        let completion = format_completion(&config.model, &response_text);
        let json = serde_json::to_string(&completion).unwrap_or_default();

        let response = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Type: application/json\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Content-Length: {}\r\n\r\n{}",
            json.len(),
            json
        );
        writer
            .write_all(response.as_bytes())
            .await
            .map_err(|e| format!("write error: {e}"))?;
        return Ok(());
    }

    // Unknown endpoint
    send_error(&mut writer, 404, "Not found. Use POST /v1/chat/completions").await
}

/// Extract the last user message from an OpenAI-format chat completion request.
fn extract_prompt(request: &serde_json::Value) -> String {
    let messages = match request.get("messages").and_then(|m| m.as_array()) {
        Some(m) => m,
        None => return String::new(),
    };

    // Collect all user messages into a single prompt, focusing on the last one
    // but including system messages as context
    let mut parts = Vec::new();

    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
        let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");

        match role {
            "system" => {
                if !content.is_empty() {
                    parts.push(format!("[System context: {}]", content));
                }
            }
            "user" => {
                parts.push(content.to_string());
            }
            _ => {}
        }
    }

    parts.join("\n\n")
}

/// Format a response as an OpenAI-compatible chat completion.
fn format_completion(model: &str, text: &str) -> serde_json::Value {
    let id = format!(
        "yoyo-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    serde_json::json!({
        "id": id,
        "object": "chat.completion",
        "created": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        "model": model,
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
    })
}

/// Send an error response.
async fn send_error(
    writer: &mut (impl AsyncWriteExt + Unpin),
    status: u16,
    message: &str,
) -> Result<(), String> {
    let body = serde_json::json!({
        "error": {
            "message": message,
            "type": "invalid_request_error"
        }
    });
    let json = serde_json::to_string(&body).unwrap_or_default();
    let status_text = match status {
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\n\
        Content-Type: application/json\r\n\
        Access-Control-Allow-Origin: *\r\n\
        Content-Length: {}\r\n\r\n{}",
        json.len(),
        json
    );
    writer
        .write_all(response.as_bytes())
        .await
        .map_err(|e| format!("write error: {e}"))?;
    Ok(())
}

/// Truncate a string for logging display.
fn truncate_for_log(s: &str, max: usize) -> String {
    let first_line = s.lines().next().unwrap_or(s);
    if first_line.len() > max {
        format!("{}...", &first_line[..max.saturating_sub(3)])
    } else {
        first_line.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_prompt_basic() {
        let req = serde_json::json!({
            "messages": [
                {"role": "user", "content": "hello world"}
            ]
        });
        assert_eq!(extract_prompt(&req), "hello world");
    }

    #[test]
    fn test_extract_prompt_with_system() {
        let req = serde_json::json!({
            "messages": [
                {"role": "system", "content": "You are helpful"},
                {"role": "user", "content": "write code"}
            ]
        });
        let prompt = extract_prompt(&req);
        assert!(prompt.contains("System context"));
        assert!(prompt.contains("write code"));
    }

    #[test]
    fn test_extract_prompt_empty_messages() {
        let req = serde_json::json!({"messages": []});
        assert_eq!(extract_prompt(&req), "");
    }

    #[test]
    fn test_extract_prompt_no_messages() {
        let req = serde_json::json!({"prompt": "test"});
        assert_eq!(extract_prompt(&req), "");
    }

    #[test]
    fn test_format_completion() {
        let result = format_completion("test-model", "Hello!");
        assert_eq!(
            result["choices"][0]["message"]["content"].as_str().unwrap(),
            "Hello!"
        );
        assert_eq!(result["model"].as_str().unwrap(), "test-model");
        assert_eq!(result["object"].as_str().unwrap(), "chat.completion");
    }

    #[test]
    fn test_truncate_for_log() {
        assert_eq!(truncate_for_log("short", 20), "short");
        assert_eq!(
            truncate_for_log("a".repeat(100).as_str(), 20),
            format!("{}...", "a".repeat(17))
        );
        assert_eq!(truncate_for_log("line1\nline2", 20), "line1");
    }
}
