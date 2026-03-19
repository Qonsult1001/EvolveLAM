//! IDE server mode for yoyo.
//!
//! Exposes a local OpenAI-compatible HTTP endpoint that routes requests through
//! yoyo's configured agent. This lets IDEs (Cursor, VS Code + Continue, etc.)
//! connect to yoyo as a local LLM backend without calling external APIs directly.
//!
//! Supports both streaming (SSE) and non-streaming responses.
//!
//! Usage:
//!   cargo run -- --serve                    # Listen on 127.0.0.1:8787
//!   cargo run -- --serve --port 9000        # Custom port
//!   cargo run -- --serve --provider openai  # Use OpenAI backend

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use yoagent::*;

use crate::format::*;
use crate::prompt::run_prompt;
use crate::AgentConfig;

/// Default port for the IDE server.
pub const DEFAULT_PORT: u16 = 8787;

/// Start the IDE server on the given port.
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
    println!("{DIM}  endpoints: GET  /health, /v1/health (health check){RESET}");
    println!("{DIM}             GET  / (status page){RESET}");
    println!("{DIM}             POST /v1/chat/completions (streaming + non-streaming){RESET}");
    println!("{DIM}             GET  /v1/models{RESET}");
    println!("{DIM}  stop:      Ctrl+C{RESET}\n");
    println!("{DIM}  Configure your IDE to use:{RESET}");
    println!("{BOLD}    Base URL: http://{addr}/v1{RESET}");
    println!("{BOLD}    API Key:  any-string-works{RESET}\n");

    let config = Arc::new(agent_config);

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{RED}  accept error: {e}{RESET}");
                continue;
            }
        };

        let config = Arc::clone(&config);

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, config, peer).await {
                eprintln!("{RED}  connection error: {e}{RESET}");
            }
        });
    }
}

/// Handle a single HTTP connection.
///
/// Each chat/completion request creates a fresh agent for conversation isolation.
async fn handle_connection(
    stream: tokio::net::TcpStream,
    config: Arc<AgentConfig>,
    peer: std::net::SocketAddr,
) -> Result<(), String> {
    let (reader, mut writer) = stream.into_split();
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

    // Health check endpoint
    if *path == "/health" || *path == "/v1/health" {
        let health_json = serde_json::json!({
            "status": "ok",
            "model": &*config.model,
            "provider": &*config.provider
        });
        let json = serde_json::to_string(&health_json).unwrap_or_default();
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

    // Root status page
    if *path == "/" && *method == "GET" {
        let html = format!(
            "<html><head><title>yoyo IDE server</title></head>\
            <body style=\"font-family:monospace;max-width:600px;margin:40px auto;\">\
            <h1>yoyo IDE server</h1>\
            <p><strong>Status:</strong> running</p>\
            <p><strong>Model:</strong> {}</p>\
            <p><strong>Provider:</strong> {}</p>\
            <h2>Endpoints</h2>\
            <ul>\
            <li><code>GET /health</code> — health check</li>\
            <li><code>GET /v1/models</code> — list models</li>\
            <li><code>POST /v1/chat/completions</code> — chat (streaming + non-streaming)</li>\
            <li><code>POST /v1/completions</code> — inline completions</li>\
            </ul>\
            <h2>IDE Setup</h2>\
            <p>Base URL: <code>{{this URL}}/v1</code></p>\
            <p>API Key: <code>any-string-works</code></p>\
            </body></html>",
            config.model, config.provider
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Type: text/html\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Content-Length: {}\r\n\r\n{}",
            html.len(),
            html
        );
        writer
            .write_all(response.as_bytes())
            .await
            .map_err(|e| format!("write error: {e}"))?;
        return Ok(());
    }

    // Models endpoint
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

        let prompt = extract_prompt(&request);
        if prompt.is_empty() {
            return send_error(&mut writer, 400, "No user message found in request").await;
        }

        let is_streaming = request
            .get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        println!(
            "{DIM}  [{peer}] prompt: {} (stream={}){RESET}",
            truncate_for_log(&prompt, 80),
            is_streaming
        );

        // Create a fresh agent for this request (conversation isolation)
        let mut agent = config.build_agent();

        if is_streaming {
            return handle_streaming_chat(&mut writer, &mut agent, config, &prompt, peer).await;
        }

        // Non-streaming: run full prompt and return
        let mut usage = yoagent::Usage::default();
        let response_text = run_prompt(&mut agent, &prompt, &mut usage, &config.model).await;

        println!(
            "{DIM}  [{peer}] response: {} chars, {} in + {} out tokens{RESET}",
            response_text.len(),
            usage.input,
            usage.output
        );

        let completion = format_completion(&config.model, &response_text, &usage);
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

    // Completions endpoint (for inline/tab completions)
    if (*path == "/v1/completions" || *path == "/completions") && *method == "POST" {
        let mut body = vec![0u8; content_length];
        buf_reader
            .read_exact(&mut body)
            .await
            .map_err(|e| format!("body read error: {e}"))?;

        let body_str = String::from_utf8_lossy(&body);
        let request: serde_json::Value =
            serde_json::from_str(&body_str).map_err(|e| format!("JSON parse error: {e}"))?;

        let prompt = request
            .get("prompt")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string();
        if prompt.is_empty() {
            return send_error(&mut writer, 400, "No prompt found in request").await;
        }

        let suffix = request.get("suffix").and_then(|s| s.as_str()).unwrap_or("");
        let full_prompt = if suffix.is_empty() {
            format!("Complete the following code. Only output the completion, no explanation:\n\n{prompt}")
        } else {
            format!(
                "Fill in the code between the prefix and suffix. Only output the inserted code, no explanation:\n\nPrefix:\n{prompt}\n\nSuffix:\n{suffix}"
            )
        };

        println!(
            "{DIM}  [{peer}] completion: {}{RESET}",
            truncate_for_log(&prompt, 80)
        );

        let is_streaming = request
            .get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Create a fresh agent for this request (conversation isolation)
        let mut agent = config.build_agent();
        let mut usage = yoagent::Usage::default();
        let response_text = run_prompt(&mut agent, &full_prompt, &mut usage, &config.model).await;

        let completion_id = format!(
            "cmpl-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if is_streaming {
            // Stream the full response as a single SSE chunk (fill-in-middle is usually short)
            let headers = "HTTP/1.1 200 OK\r\n\
                Content-Type: text/event-stream\r\n\
                Cache-Control: no-cache\r\n\
                Connection: keep-alive\r\n\
                Access-Control-Allow-Origin: *\r\n\r\n";
            writer
                .write_all(headers.as_bytes())
                .await
                .map_err(|e| format!("write error: {e}"))?;

            let chunk = serde_json::json!({
                "id": &completion_id,
                "object": "text_completion",
                "created": created,
                "model": &config.model,
                "choices": [{
                    "text": response_text,
                    "index": 0,
                    "finish_reason": "stop"
                }]
            });
            send_sse_data(&mut writer, &chunk).await?;
            writer
                .write_all(b"data: [DONE]\n\n")
                .await
                .map_err(|e| format!("write error: {e}"))?;
        } else {
            let completion = serde_json::json!({
                "id": &completion_id,
                "object": "text_completion",
                "created": created,
                "model": &config.model,
                "choices": [{
                    "text": response_text,
                    "index": 0,
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": usage.input,
                    "completion_tokens": usage.output,
                    "total_tokens": usage.input + usage.output
                }
            });
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
        }
        return Ok(());
    }

    // Unknown endpoint
    send_error(
        &mut writer,
        404,
        "Not found. Available: GET /health, GET /, GET /v1/models, POST /v1/chat/completions, POST /v1/completions",
    )
    .await
}

/// Handle a streaming chat completion request via SSE.
async fn handle_streaming_chat(
    writer: &mut (impl AsyncWriteExt + Unpin),
    agent: &mut Agent,
    config: Arc<AgentConfig>,
    prompt: &str,
    peer: std::net::SocketAddr,
) -> Result<(), String> {
    // Send SSE headers
    let headers = "HTTP/1.1 200 OK\r\n\
        Content-Type: text/event-stream\r\n\
        Cache-Control: no-cache\r\n\
        Connection: keep-alive\r\n\
        Access-Control-Allow-Origin: *\r\n\r\n";
    writer
        .write_all(headers.as_bytes())
        .await
        .map_err(|e| format!("write error: {e}"))?;

    let completion_id = format!(
        "chatcmpl-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Send initial role chunk
    let role_chunk = serde_json::json!({
        "id": &completion_id,
        "object": "chat.completion.chunk",
        "created": created,
        "model": &config.model,
        "choices": [{
            "index": 0,
            "delta": { "role": "assistant", "content": "" },
            "finish_reason": null
        }]
    });
    send_sse_data(writer, &role_chunk).await?;

    // Run through yoyo's agent with streaming
    let mut rx = agent.prompt(prompt).await;
    let mut total_chars = 0u64;

    loop {
        match rx.recv().await {
            Some(AgentEvent::MessageUpdate {
                delta: StreamDelta::Text { delta },
                ..
            }) => {
                total_chars += delta.len() as u64;

                let chunk = serde_json::json!({
                    "id": &completion_id,
                    "object": "chat.completion.chunk",
                    "created": created,
                    "model": &config.model,
                    "choices": [{
                        "index": 0,
                        "delta": { "content": delta },
                        "finish_reason": null
                    }]
                });

                if let Err(e) = send_sse_data(writer, &chunk).await {
                    // Client disconnected — that's normal
                    println!("{DIM}  [{peer}] client disconnected during stream{RESET}");
                    return Err(e);
                }
            }
            Some(AgentEvent::AgentEnd { .. }) => {
                break;
            }
            Some(AgentEvent::ToolExecutionStart { tool_name, .. }) => {
                // Send tool usage as a comment in the stream so IDEs can show status
                let tool_chunk = serde_json::json!({
                    "id": &completion_id,
                    "object": "chat.completion.chunk",
                    "created": created,
                    "model": &config.model,
                    "choices": [{
                        "index": 0,
                        "delta": { "content": format!("\n> Running: {tool_name}\n") },
                        "finish_reason": null
                    }]
                });
                let _ = send_sse_data(writer, &tool_chunk).await;
            }
            Some(AgentEvent::ToolExecutionEnd {
                tool_name,
                is_error,
                ..
            }) => {
                if is_error {
                    let err_chunk = serde_json::json!({
                        "id": &completion_id,
                        "object": "chat.completion.chunk",
                        "created": created,
                        "model": &config.model,
                        "choices": [{
                            "index": 0,
                            "delta": { "content": format!("\n> Error in {tool_name}\n") },
                            "finish_reason": null
                        }]
                    });
                    let _ = send_sse_data(writer, &err_chunk).await;
                }
            }
            None => {
                // Channel closed
                break;
            }
            _ => {
                // Other events (thinking, progress, etc.) — skip
            }
        }
    }

    // Send finish chunk
    let finish_chunk = serde_json::json!({
        "id": &completion_id,
        "object": "chat.completion.chunk",
        "created": created,
        "model": &config.model,
        "choices": [{
            "index": 0,
            "delta": {},
            "finish_reason": "stop"
        }]
    });
    send_sse_data(writer, &finish_chunk).await?;

    // Send [DONE] marker
    writer
        .write_all(b"data: [DONE]\n\n")
        .await
        .map_err(|e| format!("write error: {e}"))?;

    println!("{DIM}  [{peer}] stream complete: {total_chars} chars{RESET}");

    Ok(())
}

/// Send a single SSE data event.
async fn send_sse_data(
    writer: &mut (impl AsyncWriteExt + Unpin),
    data: &serde_json::Value,
) -> Result<(), String> {
    let json = serde_json::to_string(data).unwrap_or_default();
    let sse_line = format!("data: {json}\n\n");
    writer
        .write_all(sse_line.as_bytes())
        .await
        .map_err(|e| format!("SSE write error: {e}"))?;
    writer
        .flush()
        .await
        .map_err(|e| format!("SSE flush error: {e}"))?;
    Ok(())
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
fn format_completion(model: &str, text: &str, usage: &yoagent::Usage) -> serde_json::Value {
    let id = format!(
        "chatcmpl-{}",
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
            "prompt_tokens": usage.input,
            "completion_tokens": usage.output,
            "total_tokens": usage.input + usage.output
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
        let usage = yoagent::Usage {
            input: 100,
            output: 50,
            ..Default::default()
        };
        let result = format_completion("test-model", "Hello!", &usage);
        assert_eq!(
            result["choices"][0]["message"]["content"].as_str().unwrap(),
            "Hello!"
        );
        assert_eq!(result["model"].as_str().unwrap(), "test-model");
        assert_eq!(result["object"].as_str().unwrap(), "chat.completion");
        assert_eq!(result["usage"]["prompt_tokens"].as_u64().unwrap(), 100);
        assert_eq!(result["usage"]["completion_tokens"].as_u64().unwrap(), 50);
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

    #[test]
    fn test_extract_prompt_multi_turn() {
        let req = serde_json::json!({
            "messages": [
                {"role": "system", "content": "You are a coding assistant"},
                {"role": "user", "content": "What is Rust?"},
                {"role": "assistant", "content": "Rust is a systems programming language."},
                {"role": "user", "content": "Show me an example"}
            ]
        });
        let prompt = extract_prompt(&req);
        assert!(prompt.contains("coding assistant"));
        assert!(prompt.contains("What is Rust?"));
        assert!(prompt.contains("Show me an example"));
    }
}
