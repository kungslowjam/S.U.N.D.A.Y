//! App factory — creates the Axum router with all routes and middleware.

use crate::middleware::{auth_middleware, cors_layer, security_headers_middleware};
use crate::routes::{
    agents, channels, chat, connectors, digest, health, memory, models, sessions, speech,
    static_files, telemetry, tools, traces, upload, webhooks, ws,
};
use crate::state::AppState;
use axum::{
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;
use sunday_core::config::JarvisConfig;
use sunday_core::events::EventBus;
use sunday_engine::{discovery::get_engine_static, native::NativeLlamaEngine, InferenceEngine};
use tower_http::trace::TraceLayer;

use crate::memory_manager::MemoryManager;

/// Create the full Axum application router.
pub async fn create_app(config: JarvisConfig) -> Router {
    let bus = Arc::new(EventBus::new(true));

    let (engine, model): (Arc<dyn InferenceEngine>, String) =
        match config.engine.default.as_str() {
            "native" => {
                // Discovery: Look for GGUF models
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .unwrap_or_default();
                let global_model_dir =
                    std::path::PathBuf::from(home).join(".sunday").join("models");
                let local_model_dir = std::env::current_dir()
                    .unwrap_or_default()
                    .join("llama-cpp")
                    .join("models");

                let mut model_path = None;

                let mut gguf_paths = Vec::new();
                for dir in &[local_model_dir, global_model_dir] {
                    if let Ok(entries) = std::fs::read_dir(dir) {
                        for entry in entries.flatten() {
                            let p = entry.path();
                            if p.extension().and_then(|s| s.to_str()) == Some("gguf") {
                                gguf_paths.push(p);
                            }
                        }
                    }
                }

                // Prioritize Qwen models, then others
                gguf_paths.sort_by(|a, b| {
                    let an = a.file_name().unwrap().to_str().unwrap().to_lowercase();
                    let bn = b.file_name().unwrap().to_str().unwrap().to_lowercase();
                    let ap = if an.contains("qwen") { 0 } else { 1 };
                    let bp = if bn.contains("qwen") { 0 } else { 1 };
                    ap.cmp(&bp)
                });

                if !gguf_paths.is_empty() {
                    // Try to find the model specified in config first
                    let target_name = config.intelligence.default_model.to_lowercase();
                    if let Some(p) = gguf_paths.iter().find(|p| {
                        p.file_name().and_then(|s| s.to_str()).map(|s| s.to_lowercase() == target_name || s.to_lowercase().contains(&target_name)).unwrap_or(false)
                    }) {
                        model_path = Some(p.clone());
                    } else {
                        model_path = Some(gguf_paths[0].clone());
                    }
                }

                if let Some(path) = model_path {
                    tracing::info!("🎯 Using Native llama.cpp engine with model: {:?}", path);
                    let engine = NativeLlamaEngine::new(path.to_str().unwrap())
                        .expect("Failed to load native engine");
                    (
                        Arc::new(engine),
                        path.file_name().unwrap().to_str().unwrap().to_string(),
                    )
                } else {
                    tracing::warn!(
                        "🌐 No GGUF found for native engine, falling back to Ollama"
                    );
                    let engine = get_engine_static(&config, Some("ollama"))
                        .expect("Failed to create fallback Ollama engine");
                    let model = if config.intelligence.default_model.is_empty() {
                        "qwen3.5:latest".to_string()
                    } else {
                        config.intelligence.default_model.clone()
                    };
                    (Arc::new(engine), model)
                }
            }
            engine_key => {
                let engine = get_engine_static(&config, Some(engine_key)).unwrap_or_else(|e| {
                    tracing::warn!(
                        "Failed to create engine '{}': {}. Falling back to Ollama.",
                        engine_key,
                        e
                    );
                    get_engine_static(&config, Some("ollama"))
                        .expect("Failed to create fallback Ollama engine")
                });
                let model = if config.intelligence.default_model.is_empty() {
                    "qwen3.5:latest".to_string()
                } else {
                    config.intelligence.default_model.clone()
                };
                (Arc::new(engine), model)
            }
        };

    let mut tools = sunday_tools::executor::ToolExecutor::new(None, None, Some(bus.clone()));
    
    // Register all built-in tools
    #[allow(unused_imports)]
    use sunday_tools::builtin::*;
    tools.register(BuiltinTool::Calculator(CalculatorTool));
    tools.register(BuiltinTool::BrowserNavigate(BrowserNavigateTool));
    tools.register(BuiltinTool::BrowserScreenshot(BrowserScreenshotTool));
    tools.register(BuiltinTool::BrowserClick(BrowserClickTool));
    tools.register(BuiltinTool::BrowserType(BrowserTypeTool));
    tools.register(BuiltinTool::BrowserViewTree(BrowserViewTreeTool));
    tools.register(BuiltinTool::Think(ThinkTool));
    tools.register(BuiltinTool::FileRead(FileReadTool));
    tools.register(BuiltinTool::FileReadMultiple(FileReadMultipleTool));
    tools.register(BuiltinTool::FileWrite(FileWriteTool));
    tools.register(BuiltinTool::FileEdit(FileEditTool));
    tools.register(BuiltinTool::FileGrep(FileGrepTool));
    tools.register(BuiltinTool::ListDirectory(ListDirectoryTool));
    tools.register(BuiltinTool::ShellExec(ShellExecTool));
    tools.register(BuiltinTool::HttpRequest(HttpRequestTool));
    tools.register(BuiltinTool::GitStatus(GitStatusTool));
    tools.register(BuiltinTool::GitDiff(GitDiffTool));
    tools.register(BuiltinTool::GitLog(GitLogTool));
    tools.register(BuiltinTool::SemanticScholarSearch(SemanticScholarSearchTool));
    tools.register(BuiltinTool::ArxivSearch(ArxivSearchTool));
    tools.register(BuiltinTool::OpenAlexSearch(OpenAlexSearchTool));
    // tools.register(BuiltinTool::DelegateBrowser(DelegateBrowserTool));
    // tools.register(BuiltinTool::DelegateResearch(DelegateResearchTool));

    tools.register(BuiltinTool::WordWrite(WordWriteTool));
    tools.register(BuiltinTool::ExcelRead(ExcelReadTool));
    tools.register(BuiltinTool::ProjectCreate(ProjectWorkspaceTool));
    let memory_backend: Arc<dyn sunday_tools::storage::MemoryBackend> = {
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();
        let raw = &config.tools.storage.db_path;
        let db_path = if raw.starts_with("~/") {
            format!("{}/.{}", home, &raw[2..])
        } else {
            raw.clone()
        };
        let path = std::path::Path::new(&db_path);
        let backend = sunday_tools::storage::SQLiteMemory::new(path).expect("Failed to initialize SQLiteMemory backend");
        Arc::new(backend)
    };

    tools.register(BuiltinTool::MemorySearch(MemorySearchTool::new(memory_backend.clone())));
    tools.register(BuiltinTool::MemoryStore(MemoryStoreTool::new(memory_backend.clone())));
    tools.register(BuiltinTool::TaskPlanner(TaskPlannerTool));
    tools.register(BuiltinTool::RepoMap(RepoMapTool));
    tools.register(BuiltinTool::SystemHealth(SystemHealthTool));
    tools.register(BuiltinTool::ApplyPatch(ApplyPatchTool));
    tools.register(BuiltinTool::CodeAnalyzer(CodeAnalyzerTool));

    // Skill Discovery: Load skills from data directory
    let mut skill_list = Vec::new();
    let skill_dir = std::env::current_dir().unwrap_or_default().join("src").join("sunday").join("skills").join("data");
    if let Ok(entries) = std::fs::read_dir(skill_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(skill) = sunday_skills::load_skill(&content) {
                        skill_list.push(skill);
                    }
                }
            }
        }
    }
    tracing::info!("🎓 Loaded {} skills into the registry", skill_list.len());

    let skill_list = Arc::new(skill_list);
    let skills_for_api = skill_list.clone();

    let memory = {
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();
        let raw = &config.tools.storage.db_path;
        let db_path = if raw.starts_with("~/") {
            format!("{}/.{}", home, &raw[2..])
        } else {
            raw.clone()
        };
        if let Some(parent) = std::path::Path::new(&db_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        MemoryManager::new(&db_path).ok().map(std::sync::Arc::new)
    };

    let state = AppState::new(engine, config, bus, model, tools, memory);
 
    Router::new()
        // Health
        .route("/health", get(health::handler))
        // OpenAI-compatible chat
        .route("/v1/chat/completions", post(chat::completions_handler))
        // Models
        .route("/v1/models", get(models::list_handler))
        .route("/v1/models/pull", post(models::pull_handler))
        .route("/v1/models/switch", post(models::switch_handler))
        .route("/v1/models/{model_name}", delete(models::delete_handler))
        // Agents
        .route("/v1/agents", get(agents::list_handler).post(agents::create_handler))
        .route("/v1/agents/{id}", delete(agents::delete_handler))
        .route("/v1/agents/{id}/message", post(agents::message_handler))
        .route("/v1/managed-agents", get(agents::list_handler).post(agents::create_handler))
        .route("/v1/managed-agents/:id", get(agents::get_handler).delete(agents::delete_handler))
        .route("/v1/managed-agents/:id/state", get(agents::state_handler))
        .route("/v1/managed-agents/:id/messages", get(agents::get_messages_handler).post(agents::message_handler))
        .route("/v1/managed-agents/:id/run", post(agents::run_handler))
        // Memory
        .route("/v1/memory/store", post(memory::store_handler))
        .route("/v1/memory/search", post(memory::search_handler))
        .route("/v1/memory/related", post(memory::related_handler))
        .route("/v1/memory/facts/:entity", get(memory::get_facts_handler))
        .route("/v1/memory/stats", get(memory::stats_handler))
        // Telemetry
        .route("/v1/telemetry/stats", get(telemetry::stats_handler))
        .route("/v1/telemetry/energy", get(telemetry::energy_handler))
        .route("/v1/savings", get(telemetry::savings_handler))
        // Traces
        .route("/v1/traces", get(traces::list_handler))
        .route("/v1/traces/{trace_id}", get(traces::get_handler))
        // Tools
        .route("/v1/tools", get(tools::list_handler))
        // Skills
        .route("/v1/skills", get(move || {
            let s = skills_for_api.clone();
            async move {
                Json(json!({ "skills": *s, "count": s.len() }))
            }
        }))
        // Sessions
        .route("/v1/sessions", get(sessions::list_handler))
        .route("/v1/sessions/{session_id}", get(sessions::get_handler))
        // Channels
        .route("/v1/channels", get(channels::list_handler))
        .route("/v1/channels/{channel}/send", post(channels::send_handler))
        // Speech
        .route("/v1/speech/stt", post(speech::stt_handler))
        .route("/v1/speech/tts", post(speech::tts_handler))
        // Uploads
        .route("/v1/connectors/upload/ingest", post(upload::ingest_handler))
        .route("/v1/connectors/upload/ingest/files", post(upload::ingest_handler))
        // Info
        .route("/v1/info", get(|| async { Json(json!({ "model": "local", "agent": "native", "engine": "axum" })) }))
        .route("/v1/recommended-model", get(|| async { Json(json!({ "model": "local-model" })) }))
        .route("/v1/templates", get(|| async { Json(json!({ "templates": [] })) }))
        // Connectors
        .route("/v1/connectors", get(connectors::list_handler))
        .route("/v1/connectors/{connector}/sync", post(connectors::sync_handler))
        .route("/v1/connectors/{connector}/status", get(connectors::status_handler))
        // Digest
        .route("/v1/digest", post(digest::generate_handler))
        .route("/v1/digest/latest", get(digest::latest_handler))
        // Upload
        .route("/v1/upload/ingest", post(upload::ingest_handler))
        // Webhooks
        .route("/webhooks/twilio", post(webhooks::twilio_handler))
        .route("/webhooks/whatsapp", get(webhooks::whatsapp_verify_handler).post(webhooks::whatsapp_incoming_handler))
        .route("/webhooks/bluebubbles", post(webhooks::bluebubbles_handler))
        .route("/webhooks/line", post(webhooks::line_handler))
        // WebSocket
        .route("/v1/chat/stream", get(ws::chat_stream_handler))
        .route("/v1/agents/events", get(ws::agent_events_handler))
        // Static files + SPA catch-all
        .fallback(static_files::handler)
        // Middleware
        .layer(cors_layer())
        .layer(axum::middleware::from_fn(security_headers_middleware))
        .layer(axum::middleware::from_fn(auth_middleware))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
