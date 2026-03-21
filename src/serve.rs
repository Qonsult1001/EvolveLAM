//! IDE server mode for yoyo — axum-based API gateway.
//!
//! Exposes a local OpenAI-compatible HTTP endpoint that routes requests through
//! yoyo's configured agent. This lets IDEs (Cursor, VS Code + Continue, etc.)
//! connect to yoyo as a local LLM backend without calling external APIs directly.
//!
//! Also exposes brain API endpoints for querying the SCA knowledge graph.
//!
//! Supports both streaming (SSE) and non-streaming responses.
//!
//! Usage:
//!   cargo run -- --serve                    # Listen on 127.0.0.1:8787
//!   cargo run -- --serve --port 9000        # Custom port

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::{Html, IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use tokio_stream::wrappers::ReceiverStream;
use tower_http::cors::CorsLayer;
use yoagent::*;

use crate::context_lens::ContextLens;
use crate::format::*;
use crate::memory::ConnectionGraph;
use crate::prompt::run_prompt;
use crate::AgentConfig;

/// Default port for the IDE server.
pub const DEFAULT_PORT: u16 = 8787;

/// Shared server state.
struct AppState {
    config: AgentConfig,
    lens: ContextLens,
}

/// Start the IDE server on the given port.
pub async fn start_server(agent_config: AgentConfig, port: u16) {
    let addr = format!("127.0.0.1:{port}");

    // Create the SCA Context Lens for brain queries
    let graph = ConnectionGraph::load();
    let skills_path = std::path::Path::new("skills");
    let skills_dir = if skills_path.is_dir() {
        Some(skills_path.to_path_buf())
    } else {
        None
    };
    let lens = ContextLens::new(graph, skills_dir);

    let state = Arc::new(AppState {
        config: agent_config,
        lens,
    });

    let app = Router::new()
        // Existing endpoints
        .route("/", get(root_status))
        .route("/health", get(health))
        .route("/v1/health", get(health))
        .route("/v1/models", get(models))
        .route("/models", get(models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/chat/completions", post(chat_completions))
        .route("/v1/completions", post(completions))
        .route("/completions", post(completions))
        // Brain endpoints (new — SCA layer)
        .route("/v1/brain/query", post(brain_query))
        .route("/v1/brain/concepts", get(brain_concepts))
        .route("/v1/brain/connections", get(brain_connections))
        .route("/v1/brain/learn", post(brain_learn))
        .layer(CorsLayer::permissive())
        .with_state(state);

    println!("{BOLD}yoyo IDE server{RESET}");
    println!("{DIM}  listening: http://{addr}{RESET}");

    // Print config info (we access it via a fresh ref before moving into state)
    println!("{DIM}  endpoints: GET  /health, /v1/health (health check){RESET}");
    println!("{DIM}             GET  / (status page){RESET}");
    println!("{DIM}             POST /v1/chat/completions (streaming + non-streaming){RESET}");
    println!("{DIM}             POST /v1/completions (inline completions){RESET}");
    println!("{DIM}             POST /v1/brain/query (brain context query){RESET}");
    println!("{DIM}             GET  /v1/brain/concepts (search concepts){RESET}");
    println!("{DIM}             GET  /v1/brain/connections (get connections){RESET}");
    println!("{DIM}             POST /v1/brain/learn (ingest connection){RESET}");
    println!("{DIM}             GET  /v1/models{RESET}");
    println!("{DIM}  stop:      Ctrl+C{RESET}\n");
    println!("{DIM}  Configure your IDE to use:{RESET}");
    println!("{BOLD}    Base URL: http://{addr}/v1{RESET}");
    println!("{BOLD}    API Key:  any-string-works{RESET}\n");

    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{RED}  Failed to bind to {addr}: {e}{RESET}");
            return;
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("{RED}  Server error: {e}{RESET}");
    }
}

// ── Existing Endpoints ───────────────────────────────────────────────

async fn health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "model": &state.config.model,
        "provider": &state.config.provider
    }))
}

async fn root_status(State(state): State<Arc<AppState>>) -> Html<String> {
    Html(format!(
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
        <li><code>POST /v1/brain/query</code> — brain context query</li>\
        <li><code>GET /v1/brain/concepts</code> — search concepts</li>\
        <li><code>GET /v1/brain/connections</code> — get connections</li>\
        <li><code>POST /v1/brain/learn</code> — ingest connection</li>\
        </ul>\
        <h2>IDE Setup</h2>\
        <p>Base URL: <code>{{this URL}}/v1</code></p>\
        <p>API Key: <code>any-string-works</code></p>\
        </body></html>",
        state.config.model, state.config.provider
    ))
}

async fn models(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "object": "list",
        "data": [{
            "id": &state.config.model,
            "object": "model",
            "owned_by": "yoyo"
        }]
    }))
}

async fn chat_completions(
    State(state): State<Arc<AppState>>,
    axum::Json(request): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    let prompt = extract_prompt(&request);
    if prompt.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "No user message found in request")
            .into_response();
    }

    let is_streaming = request
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    println!(
        "{DIM}  prompt: {} (stream={}){RESET}",
        truncate_for_log(&prompt, 80),
        is_streaming
    );

    let mut agent = state.config.build_agent();

    if is_streaming {
        return handle_streaming_chat(agent, state, &prompt).into_response();
    }

    // Non-streaming: run full prompt and return
    let mut usage = yoagent::Usage::default();
    let response_text =
        run_prompt(&mut agent, &prompt, &mut usage, &state.config.model, None).await;

    println!(
        "{DIM}  response: {} chars, {} in + {} out tokens{RESET}",
        response_text.len(),
        usage.input,
        usage.output
    );

    let completion = format_completion(&state.config.model, &response_text, &usage);
    Json(completion).into_response()
}

async fn completions(
    State(state): State<Arc<AppState>>,
    axum::Json(request): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    let prompt = request
        .get("prompt")
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_string();
    if prompt.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "No prompt found in request")
            .into_response();
    }

    let suffix = request.get("suffix").and_then(|s| s.as_str()).unwrap_or("");
    let full_prompt = if suffix.is_empty() {
        format!(
            "Complete the following code. Only output the completion, no explanation:\n\n{prompt}"
        )
    } else {
        format!(
            "Fill in the code between the prefix and suffix. Only output the inserted code, no explanation:\n\nPrefix:\n{prompt}\n\nSuffix:\n{suffix}"
        )
    };

    let is_streaming = request
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    println!(
        "{DIM}  completion: {}{RESET}",
        truncate_for_log(&prompt, 80)
    );

    let mut agent = state.config.build_agent();
    let mut usage = yoagent::Usage::default();
    let response_text = run_prompt(
        &mut agent,
        &full_prompt,
        &mut usage,
        &state.config.model,
        None,
    )
    .await;

    let completion_id = gen_id("cmpl");
    let created = unix_secs();

    if is_streaming {
        let chunk = serde_json::json!({
            "id": &completion_id,
            "object": "text_completion",
            "created": created,
            "model": &state.config.model,
            "choices": [{
                "text": response_text,
                "index": 0,
                "finish_reason": "stop"
            }]
        });
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(2);
        let json_str = serde_json::to_string(&chunk).unwrap_or_default();
        let _ = tx.send(Ok(Event::default().data(json_str))).await;
        let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
        drop(tx);
        return Sse::new(ReceiverStream::new(rx)).into_response();
    }

    let completion = serde_json::json!({
        "id": &completion_id,
        "object": "text_completion",
        "created": created,
        "model": &state.config.model,
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
    Json(completion).into_response()
}

// ── Brain Endpoints (SCA Layer) ──────────────────────────────────────

/// POST /v1/brain/query — query brain context for an input
async fn brain_query(
    State(state): State<Arc<AppState>>,
    axum::Json(request): axum::Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let input = request.get("input").and_then(|v| v.as_str()).unwrap_or("");

    let result = state.lens.query(input);

    Json(serde_json::json!({
        "input": input,
        "relevance_score": result.relevance_score,
        "context": result.text,
        "has_context": !result.text.is_empty()
    }))
}

/// GET /v1/brain/concepts?q=rust — search concepts in the graph
async fn brain_concepts(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let query = params.get("q").map(|s| s.as_str()).unwrap_or("");

    let all_concepts: Vec<&String> = state.lens.graph().edges.keys().collect();

    let matched: Vec<&str> = if query.is_empty() {
        all_concepts.iter().map(|s| s.as_str()).collect()
    } else {
        let q_lower = query.to_lowercase();
        all_concepts
            .iter()
            .filter(|c| c.to_lowercase().contains(&q_lower))
            .map(|s| s.as_str())
            .collect()
    };

    Json(serde_json::json!({
        "query": query,
        "count": matched.len(),
        "concepts": matched
    }))
}

/// GET /v1/brain/connections?from=error_handling — get connections for a concept
async fn brain_connections(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let from = params.get("from").map(|s| s.as_str()).unwrap_or("");

    if from.is_empty() {
        // Return summary of all connections
        let total_edges: usize = state.lens.graph().edges.values().map(|v| v.len()).sum();
        let total_nodes = state.lens.graph().edges.len();
        return Json(serde_json::json!({
            "total_nodes": total_nodes,
            "total_edges": total_edges,
            "hint": "Pass ?from=concept_name to see connections for a specific concept"
        }));
    }

    let connections: Vec<serde_json::Value> = state
        .lens
        .graph()
        .edges
        .get(from)
        .map(|conns| {
            conns
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "to": c.to,
                        "weight": c.weight,
                        "activations": c.activations,
                        "kind": c.kind.to_string(),
                        "last_activated": c.last_activated
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Json(serde_json::json!({
        "from": from,
        "count": connections.len(),
        "connections": connections
    }))
}

/// POST /v1/brain/learn — ingest a new connection (enforces hostile set rules)
async fn brain_learn(axum::Json(request): axum::Json<serde_json::Value>) -> impl IntoResponse {
    let from = request.get("from").and_then(|v| v.as_str()).unwrap_or("");
    let to = request.get("to").and_then(|v| v.as_str()).unwrap_or("");
    let kind = request
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("semantic");

    if from.is_empty() || to.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Both 'from' and 'to' fields are required"
            })),
        );
    }

    // Validate kind
    let valid_kinds = [
        "semantic",
        "causal",
        "temporal",
        "mathematical",
        "scientific",
    ];
    if !valid_kinds.contains(&kind) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Invalid kind '{}'. Must be one of: {:?}", kind, valid_kinds)
            })),
        );
    }

    // Hostile set rule: causal connections must not create cycles
    // (We can only check if from == to here; full cycle detection would need the graph)
    if kind == "causal" && from == to {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Causal connections cannot be self-referential (would violate DAG constraint)"
            })),
        );
    }

    // Append to connections.jsonl using the same format as the rest of the codebase
    let now = chrono_now_iso();
    let weight: f64 = request
        .get("weight")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.1);

    let record = serde_json::json!({
        "from": from,
        "to": to,
        "weight": weight,
        "activations": 1,
        "last_activated": now,
        "kind": kind
    });

    let path = std::path::Path::new("memory/connections.jsonl");
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        Ok(mut file) => {
            use std::io::Write;
            let line = serde_json::to_string(&record).unwrap_or_default();
            if writeln!(file, "{line}").is_err() {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Failed to write connection"})),
                );
            }
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to open connections file: {e}")})),
            );
        }
    }

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "status": "created",
            "connection": record
        })),
    )
}

// ── Streaming ────────────────────────────────────────────────────────

fn handle_streaming_chat(
    mut agent: Agent,
    state: Arc<AppState>,
    prompt: &str,
) -> Sse<ReceiverStream<Result<Event, std::convert::Infallible>>> {
    let completion_id = gen_id("chatcmpl");
    let created = unix_secs();
    let model = state.config.model.clone();
    let prompt = prompt.to_string();

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(32);

    tokio::spawn(async move {
        // Send initial role chunk
        let role_chunk = serde_json::json!({
            "id": &completion_id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": &model,
            "choices": [{
                "index": 0,
                "delta": { "role": "assistant", "content": "" },
                "finish_reason": null
            }]
        });
        let _ = tx
            .send(Ok(
                Event::default().data(serde_json::to_string(&role_chunk).unwrap_or_default())
            ))
            .await;

        // Run through yoyo's agent with streaming
        let mut event_rx = agent.prompt(&prompt).await;
        let mut total_chars = 0u64;

        loop {
            match event_rx.recv().await {
                Some(AgentEvent::MessageUpdate {
                    delta: StreamDelta::Text { delta },
                    ..
                }) => {
                    total_chars += delta.len() as u64;

                    let chunk = serde_json::json!({
                        "id": &completion_id,
                        "object": "chat.completion.chunk",
                        "created": created,
                        "model": &model,
                        "choices": [{
                            "index": 0,
                            "delta": { "content": delta },
                            "finish_reason": null
                        }]
                    });

                    if tx
                        .send(Ok(Event::default()
                            .data(serde_json::to_string(&chunk).unwrap_or_default())))
                        .await
                        .is_err()
                    {
                        println!("{DIM}  client disconnected during stream{RESET}");
                        return;
                    }
                }
                Some(AgentEvent::AgentEnd { .. }) => break,
                Some(AgentEvent::ToolExecutionStart { tool_name, .. }) => {
                    let tool_chunk = serde_json::json!({
                        "id": &completion_id,
                        "object": "chat.completion.chunk",
                        "created": created,
                        "model": &model,
                        "choices": [{
                            "index": 0,
                            "delta": { "content": format!("\n> Running: {tool_name}\n") },
                            "finish_reason": null
                        }]
                    });
                    let _ = tx
                        .send(Ok(Event::default()
                            .data(serde_json::to_string(&tool_chunk).unwrap_or_default())))
                        .await;
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
                            "model": &model,
                            "choices": [{
                                "index": 0,
                                "delta": { "content": format!("\n> Error in {tool_name}\n") },
                                "finish_reason": null
                            }]
                        });
                        let _ = tx
                            .send(Ok(Event::default()
                                .data(serde_json::to_string(&err_chunk).unwrap_or_default())))
                            .await;
                    }
                }
                None => break,
                _ => {}
            }
        }

        // Send finish chunk
        let finish_chunk = serde_json::json!({
            "id": &completion_id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": &model,
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }]
        });
        let _ = tx
            .send(Ok(Event::default().data(
                serde_json::to_string(&finish_chunk).unwrap_or_default(),
            )))
            .await;

        // Send [DONE]
        let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;

        println!("{DIM}  stream complete: {total_chars} chars{RESET}");
    });

    Sse::new(ReceiverStream::new(rx))
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Extract the last user message from an OpenAI-format chat completion request.
fn extract_prompt(request: &serde_json::Value) -> String {
    let messages = match request.get("messages").and_then(|m| m.as_array()) {
        Some(m) => m,
        None => return String::new(),
    };

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
    serde_json::json!({
        "id": gen_id("chatcmpl"),
        "object": "chat.completion",
        "created": unix_secs(),
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

fn error_response(status: StatusCode, message: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({
            "error": {
                "message": message,
                "type": "invalid_request_error"
            }
        })),
    )
}

fn gen_id(prefix: &str) -> String {
    format!(
        "{prefix}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    )
}

fn unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn chrono_now_iso() -> String {
    let now = unix_secs();
    let days_since_epoch = now / 86400;
    let years = 1970 + days_since_epoch / 365;
    let remaining_days = days_since_epoch % 365;
    let month = remaining_days / 30 + 1;
    let day = remaining_days % 30 + 1;
    format!("{years:04}-{month:02}-{day:02}T00:00:00Z")
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

    #[test]
    fn test_gen_id_has_prefix() {
        let id = gen_id("test");
        assert!(id.starts_with("test-"));
    }

    #[test]
    fn test_chrono_now_iso_format() {
        let ts = chrono_now_iso();
        assert!(ts.contains('T'));
        assert!(ts.ends_with('Z'));
    }
}
