//! Chat completions endpoint with SSE streaming and non-streaming support.

use axum::{
    extract::State,
    response::{sse::Event, Sse, IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use sunday_core::{GenerateResult, Message, Role};
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::StreamExt;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<serde_json::Value>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i64,
    pub tools: Option<serde_json::Value>,
    pub extra: Option<serde_json::Value>,
}

fn default_temperature() -> f64 {
    0.7
}
fn default_max_tokens() -> i64 {
    2048
}

// ---------------------------------------------------------------------------
// Non-streaming response shape (OpenAI-compatible)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ChatCompletionResponse {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<Choice>,
    usage: sunday_core::Usage,
}

#[derive(Serialize)]
struct Choice {
    index: u32,
    message: ResponseMessage,
    finish_reason: String,
}

#[derive(Serialize)]
struct ResponseMessage {
    role: String,
    content: String,
}

impl From<GenerateResult> for ChatCompletionResponse {
    fn from(result: GenerateResult) -> Self {
        Self {
            id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
            object: "chat.completion".into(),
            created: chrono::Utc::now().timestamp(),
            model: result.model.clone(),
            choices: vec![Choice {
                index: 0,
                message: ResponseMessage {
                    role: "assistant".into(),
                    content: result.content,
                },
                finish_reason: result.finish_reason,
            }],
            usage: result.usage,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_messages(req_messages: &[serde_json::Value]) -> Vec<Message> {
    req_messages
        .iter()
        .filter_map(|m| {
            let role = m.get("role")?.as_str()?;
            let content = m.get("content")?.as_str()?;
            let role = match role {
                "system" => Role::System,
                "user" => Role::User,
                "assistant" => Role::Assistant,
                _ => Role::User,
            };
            Some(Message::new(role, content))
        })
        .collect()
}

fn build_persist_queue(req_messages: &[serde_json::Value]) -> Vec<(String, String)> {
    req_messages
        .iter()
        .filter_map(|m| {
            let role = m.get("role")?.as_str()?.to_string();
            let content = m.get("content")?.as_str()?.to_string();
            Some((role, content))
        })
        .collect()
}

fn persist_conversation(
    memory: Option<Arc<crate::memory_manager::MemoryManager>>,
    session_tag: String,
    messages: Vec<(String, String)>,
) {
    if let Some(mem) = memory {
        tokio::task::spawn_blocking(move || {
            for (role, content) in messages {
                if let Err(e) = mem.add_message(&session_tag, &role, &content) {
                    tracing::warn!("Failed to persist message to memory: {}", e);
                }
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

use sunday_agents::loop_guard::LoopGuard;

/// OpenAI-compatible chat completions endpoint with Agentic Loop support.
pub async fn completions_handler(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Response {
    let mut messages = parse_messages(&req.messages);

    let model = if req.model.is_empty() {
        state.model.read().await.clone()
    } else {
        req.model
    };

    let mut extra = req.extra.unwrap_or_else(|| serde_json::json!({}));
    if let Some(ref tools) = req.tools {
        extra["tools"] = tools.clone();
    }

    // Session tag for episodic memory
    let session_tag = format!(
        "chat-{}-{}",
        chrono::Utc::now().timestamp_millis(),
        uuid::Uuid::new_v4()
    );
    let mut persist_queue = build_persist_queue(&req.messages);
    let req_messages_for_stream = req.messages.clone();

    let mut use_agent_loop = false;
    if let Some(ref tools) = req.tools {
        if tools.as_array().map(|a| a.iter().any(|t| t["function"]["name"] == "sunday_agent_tools")).unwrap_or(false) {
            use_agent_loop = true;
            
            // Filter out the marker tool from being sent to the LLM engine
            if let Some(tools_arr) = extra["tools"].as_array_mut() {
                tools_arr.retain(|t| t["function"]["name"] != "sunday_agent_tools");
            }
            
            let tool_specs = state.tools.tool_specs();
            let mut tool_desc = String::new();
            for spec in tool_specs {
                if let Some(name) = spec["function"]["name"].as_str() {
                    let desc = spec["function"]["description"].as_str().unwrap_or("");
                    tool_desc.push_str(&format!("- {}: {}\n", name, desc));
                }
            }

            let agent_prompt = format!(
                "You are SUNDAY, a high-performance autonomous agent.\n\
                 Your goal is to fulfill user requests by using the tools below. You MUST use a tool if the request requires external information (like prices, news, file access, etc.).\n\n\
                 Available tools:\n{}\n\n\
                 HOW TO USE TOOLS:\n\
                 1. If you need a tool, output ONLY: TOOL: tool_name({{\"arg\": \"val\"}})\n\
                 2. DO NOT repeat the tool call in your final answer.\n\
                 3. Provide your final response directly and cleanly after getting the tool results.\n\n\
                 CRITICAL: Keep responses professional and direct!",
                tool_desc
            );
            
            // Insert or update system prompt
            if let Some(sys_msg) = messages.iter_mut().find(|m| m.role == Role::System) {
                sys_msg.content = format!("{}\n\n{}", agent_prompt, sys_msg.content);
            } else {
                messages.insert(0, Message::new(Role::System, &agent_prompt));
            }
        }
    }

    if req.stream {
        let engine_lock = state.engine.clone();
        let tools_executor = state.tools.clone();
        let memory = state.memory.clone();
        let session_tag_clone = session_tag.clone();

        let stream = async_stream::stream! {
            let engine = engine_lock.read().await.clone();
            let mut current_messages = messages.clone();
            let mut turns = 0;
            const MAX_TURNS: usize = 10;
            let mut loop_guard = LoopGuard::default();
            let mut final_content = String::new();

            yield Ok::<_, std::convert::Infallible>(Event::default().event("agent_turn_start").data("{}"));

            while turns < MAX_TURNS {
                turns += 1;
                let mut accumulated_content = String::new();
                let mut formal_tool_calls: Vec<serde_json::Value> = Vec::new();
                let mut hide_subsequent = false;
                let mut stream_buffer = String::new();

                match engine.stream(&current_messages, &model, req.temperature, req.max_tokens, Some(&extra)).await {
                    Ok(mut token_stream) => {
                        while let Some(chunk) = token_stream.next().await {
                            match chunk {
                                Ok(val) => {
                                    // 1. Accumulate text content
                                    let mut content_to_accumulate = String::new();
                                    if let Some(content) = val["choices"][0]["delta"]["content"].as_str() {
                                        accumulated_content.push_str(content);
                                        content_to_accumulate.push_str(content);
                                    }
                                    
                                    // 2. Accumulate formal tool calls (OpenAI style)
                                    if let Some(tcs) = val["choices"][0]["delta"]["tool_calls"].as_array() {
                                        for tc in tcs {
                                            let idx = tc["index"].as_u64().unwrap_or(0) as usize;
                                            while formal_tool_calls.len() <= idx {
                                                formal_tool_calls.push(serde_json::json!({"id": "", "function": {"name": "", "arguments": ""}}));
                                            }
                                            let current_tc = &mut formal_tool_calls[idx];
                                            
                                            if let Some(id) = tc["id"].as_str() { current_tc["id"] = id.into(); }
                                            if let Some(name) = tc["function"]["name"].as_str() { current_tc["function"]["name"] = name.into(); }
                                            if let Some(args) = tc["function"]["arguments"].as_str() {
                                                let prev_args = current_tc["function"]["arguments"].as_str().unwrap_or("");
                                                current_tc["function"]["arguments"] = format!("{}{}", prev_args, args).into();
                                            }
                                        }
                                    }

                                    // 3. Process stream filter
                                    if use_agent_loop {
                                        if hide_subsequent {
                                            // Do not stream any text to the user
                                        } else {
                                            stream_buffer.push_str(&content_to_accumulate);
                                            
                                            let (chars_to_stream, remaining, found_tool) = {
                                                if stream_buffer.contains("TOOL:") {
                                                    let idx = stream_buffer.find("TOOL:").unwrap();
                                                    (stream_buffer[..idx].to_string(), String::new(), true)
                                                } else {
                                                    let prefixes = ["TOOL", "TOO", "TO", "T"];
                                                    let mut matched = false;
                                                    let mut res = (stream_buffer.clone(), String::new(), false);
                                                    for pfx in &prefixes {
                                                        if stream_buffer.ends_with(pfx) {
                                                            let idx = stream_buffer.len() - pfx.len();
                                                            res = (stream_buffer[..idx].to_string(), stream_buffer[idx..].to_string(), false);
                                                            matched = true;
                                                            break;
                                                        }
                                                    }
                                                    res
                                                }
                                            };
                                            
                                            stream_buffer = remaining;
                                            if found_tool {
                                                hide_subsequent = true;
                                            }
                                            
                                            if !chars_to_stream.is_empty() {
                                                let mut modified_val = val.clone();
                                                modified_val["choices"][0]["delta"]["content"] = serde_json::json!(chars_to_stream);
                                                let event = Event::default().data(serde_json::to_string(&modified_val).unwrap_or_default());
                                                yield Ok::<_, std::convert::Infallible>(event);
                                            } else if !formal_tool_calls.is_empty() {
                                                // If there's formal tool calls, we still yield the chunk so the frontend gets them
                                                let event = Event::default().data(serde_json::to_string(&val).unwrap_or_default());
                                                yield Ok::<_, std::convert::Infallible>(event);
                                            }
                                        }
                                    } else {
                                        // Standard pass through
                                        let event = Event::default().data(serde_json::to_string(&val).unwrap_or_default());
                                        yield Ok::<_, std::convert::Infallible>(event);
                                    }
                                }
                                Err(e) => {
                                    let error_msg = format!("\n\n[⚠️ SUNDAY Error]: {}\n", e);
                                    let error_json = serde_json::json!({
                                        "choices": [{
                                            "index": 0,
                                            "delta": { "content": error_msg },
                                            "finish_reason": "error"
                                        }]
                                    });
                                    yield Ok(Event::default().data(serde_json::to_string(&error_json).unwrap()));
                                    yield Ok(Event::default().event("error").data(format!("{{\"error\":\"{}\"}}", e)));
                                    return;
                                }
                            }
                        }

                        // Flush remaining buffer if we didn't find any tool
                        if use_agent_loop && !hide_subsequent && !stream_buffer.is_empty() {
                            let final_chunk = serde_json::json!({
                                "choices": [{
                                    "index": 0,
                                    "delta": { "content": stream_buffer },
                                    "finish_reason": null
                                }]
                            });
                            let event = Event::default().data(serde_json::to_string(&final_chunk).unwrap_or_default());
                            yield Ok::<_, std::convert::Infallible>(event);
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("\n\n[⚠️ SUNDAY Error]: {}\n", e);
                        let error_json = serde_json::json!({
                            "choices": [{
                                "index": 0,
                                "delta": { "content": error_msg },
                                "finish_reason": "error"
                            }]
                        });
                        yield Ok(Event::default().data(serde_json::to_string(&error_json).unwrap()));
                        yield Ok(Event::default().event("error").data(format!("{{\"error\":\"{}\"}}", e)));
                        return;
                    }
                }

                final_content = accumulated_content.clone();
                let mut tool_to_execute = None;

                // Priority 1: Formal tool calls
                if !formal_tool_calls.is_empty() {
                    let tc = &formal_tool_calls[0]; // Take the first one for simplicity
                    let name = tc["function"]["name"].as_str().unwrap_or("");
                    let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
                    let args: serde_json::Value = serde_json::from_str(args_str).unwrap_or(serde_json::json!({}));
                    tool_to_execute = Some((name.to_string(), args));
                } 
                // Priority 2: Text-based TOOL: format
                else if use_agent_loop && accumulated_content.contains("TOOL:") {
                    if let Some(start_idx) = accumulated_content.find("TOOL:") {
                        let rest = &accumulated_content[start_idx + 5..];
                        if let Some(name_end) = rest.find('(') {
                            let tool_name = rest[..name_end].trim();
                            let rest = &rest[name_end + 1..];
                            if let Some(args_end) = rest.rfind(')') {
                                let args_str = &rest[..args_end];
                                let args: serde_json::Value = serde_json::from_str(args_str).unwrap_or(serde_json::json!({}));
                                tool_to_execute = Some((tool_name.to_string(), args));
                            }
                        }
                    }
                }

                if let Some((name, args)) = tool_to_execute {
                    let args_str = args.to_string();
                    if let Some(loop_msg) = loop_guard.check(&name, &args_str) {
                        let error_json = serde_json::json!({
                            "choices": [{
                                "index": 0,
                                "delta": { "content": format!("\n\n[⚠️ SUNDAY Loop Guard]: {}\n", loop_msg) },
                                "finish_reason": "stop"
                            }]
                        });
                        yield Ok(Event::default().data(serde_json::to_string(&error_json).unwrap()));
                        break;
                    }

                    // Emit tool_call_start event for the frontend React components to render.
                    yield Ok(Event::default().event("tool_call_start").data(serde_json::to_string(&serde_json::json!({
                        "tool": name,
                        "arguments": args
                    })).unwrap()));

                    // Execute tool in a dedicated blocking thread to prevent blocking the async executor.
                    // This allows yielded events (like tool_call_start) to be instantly flushed to the user!
                    let start_time = std::time::Instant::now();
                    let name_for_exec = name.clone();
                    let args_for_exec = args.clone();
                    let executor_for_exec = tools_executor.clone();

                    let join_res = tokio::task::spawn_blocking(move || {
                        executor_for_exec.execute(&name_for_exec, &args_for_exec, Some("orchestrator"), None)
                    }).await;

                    match join_res {
                        Ok(Ok(res)) => {
                            let latency = start_time.elapsed().as_millis() as u64;
                            yield Ok(Event::default().event("tool_call_end").data(serde_json::to_string(&serde_json::json!({
                                "tool": name,
                                "success": res.success,
                                "result": res.content,
                                "latency": latency
                            })).unwrap()));

                            current_messages.push(Message::new(Role::Assistant, &accumulated_content));
                            current_messages.push(Message::new(Role::User, &format!("Observation: {}", res.content)));
                        }
                        Ok(Err(e)) => {
                            yield Ok(Event::default().event("tool_call_end").data(serde_json::to_string(&serde_json::json!({
                                "tool": name,
                                "success": false,
                                "result": e.to_string()
                            })).unwrap()));

                            current_messages.push(Message::new(Role::Assistant, &accumulated_content));
                            current_messages.push(Message::new(Role::User, &format!("Observation: Error - {}", e)));
                            // Continue the loop so the agent can retry or explain
                        }
                        Err(join_err) => {
                            let err_msg = format!("Task join error: {}", join_err);
                            yield Ok(Event::default().event("tool_call_end").data(serde_json::to_string(&serde_json::json!({
                                "tool": name,
                                "success": false,
                                "result": err_msg.clone()
                            })).unwrap()));

                            current_messages.push(Message::new(Role::Assistant, &accumulated_content));
                            current_messages.push(Message::new(Role::User, &format!("Observation: Error - {}", err_msg)));
                        }
                    }
                } else {
                    // No tool call found, this is a final turn
                    break;
                }
            }

            // Persist conversation to memory (fire-and-forget)
            let mut final_queue = build_persist_queue(&req_messages_for_stream);
            final_queue.push(("assistant".into(), final_content));
            persist_conversation(memory, session_tag_clone, final_queue);

            yield Ok(Event::default().data("[DONE]"));
        };

        Sse::new(stream)
            .keep_alive(
                axum::response::sse::KeepAlive::new()
                    .interval(Duration::from_secs(15))
                    .text(""),
            )
            .into_response()
    } else {
        let engine = state.engine.read().await.clone();
        let result = engine.generate(&messages, &model, req.temperature, req.max_tokens, Some(&extra));
        match result {
            Ok(result) => {
                let mut resp: ChatCompletionResponse = result.into();
                if resp.model.is_empty() {
                    resp.model = model;
                }
                // Persist conversation
                persist_queue.push(("assistant".into(), resp.choices[0].message.content.clone()));
                persist_conversation(state.memory.clone(), session_tag, persist_queue);
                Json(resp).into_response()
            }
            Err(e) => {
                let body = serde_json::json!({"error": e.to_string()});
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
            }
        }
    }
}
