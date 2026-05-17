//! ScanChunksTool — semantic grep via LM-powered chunk scanning.

use crate::traits::BaseTool;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec, Role, Message as SundayMessage};
use once_cell::sync::Lazy;
use serde_json::Value;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;

static SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "scan_chunks".into(),
    description: "Semantic search — feeds chunks from the knowledge store to an LM that reads the actual text looking for relevant information.".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "question": {
                "type": "string",
                "description": "What to look for in the chunks."
            },
            "source": {
                "type": "string",
                "description": "Filter by source (e.g. 'granola', 'gmail')."
            },
            "doc_type": {
                "type": "string",
                "description": "Filter by doc type (e.g. 'document', 'email')."
            },
            "since": {
                "type": "string",
                "description": "Only chunks after this ISO timestamp."
            },
            "until": {
                "type": "string",
                "description": "Only chunks before this ISO timestamp."
            },
            "max_chunks": {
                "type": "integer",
                "description": "Max chunks to scan (default 200)."
            }
        },
        "required": ["question"]
    }),
    category: "knowledge".into(),
    cost_estimate: 0.1,
    latency_estimate: 5.0,
    requires_confirmation: false,
    timeout_seconds: 120.0,
    required_capabilities: vec![],
    metadata: HashMap::new(),
});

pub struct ScanChunksTool;

impl BaseTool for ScanChunksTool {
    fn tool_id(&self) -> &str {
        "scan_chunks"
    }

    fn spec(&self) -> &ToolSpec {
        &SPEC
    }

    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let question = params.get("question").and_then(|v| v.as_str()).unwrap_or("");
        if question.is_empty() {
            return Ok(ToolResult::failure(self.tool_id(), "No question provided."));
        }

        let source = params.get("source").and_then(|v| v.as_str()).unwrap_or("");
        let doc_type = params.get("doc_type").and_then(|v| v.as_str()).unwrap_or("");
        let since = params.get("since").and_then(|v| v.as_str()).unwrap_or("");
        let until = params.get("until").and_then(|v| v.as_str()).unwrap_or("");
        let max_chunks = params.get("max_chunks").and_then(|v| v.as_i64()).unwrap_or(200) as usize;
        let batch_size = 20;

        let home = std::env::var("HOME")
            .map_err(|_| SUNDAYError::Io(std::io::Error::other("HOME environment variable not set")))?;
        let db_path = PathBuf::from(home).join(".sunday").join("knowledge.db");

        if !db_path.exists() {
            return Ok(ToolResult::failure(self.tool_id(), "Knowledge database not found."));
        }

        let pool = crate::storage::pool::get_sqlite_pool(&db_path)?;
        let conn = pool.get()
            .map_err(|e| SUNDAYError::Io(std::io::Error::other(e.to_string())))?;

        let mut where_clauses = Vec::new();
        let mut sql_params: Vec<String> = Vec::new();

        if !source.is_empty() {
            where_clauses.push("source = ?");
            sql_params.push(source.to_string());
        }
        if !doc_type.is_empty() {
            where_clauses.push("doc_type = ?");
            sql_params.push(doc_type.to_string());
        }
        if !since.is_empty() {
            where_clauses.push("timestamp >= ?");
            sql_params.push(since.to_string());
        }
        if !until.is_empty() {
            where_clauses.push("timestamp <= ?");
            sql_params.push(until.to_string());
        }

        let where_clause = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        let sql = format!(
            "SELECT content, source, title, author FROM knowledge_chunks {} LIMIT {}",
            where_clause, max_chunks
        );

        let mut stmt = conn.prepare(&sql)
            .map_err(|e| SUNDAYError::Io(std::io::Error::other(e.to_string())))?;

        // Convert string params to rusqlite ToSql trait objects
        let params_iter: Vec<&dyn rusqlite::ToSql> = sql_params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

        let rows = stmt.query_map(&params_iter[..], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| SUNDAYError::Io(std::io::Error::other(e.to_string())))?;

        let rows: Vec<_> = rows.filter_map(Result::ok).collect();

        if rows.is_empty() {
            let mut res = ToolResult::success(self.tool_id(), "No chunks found matching filters.");
            res.metadata.insert("chunks_scanned".to_string(), Value::Number(serde_json::Number::from(0)));
            return Ok(res);
        }

        let config = sunday_core::JarvisConfig::default();
        let engine_arc = std::sync::Arc::new(sunday_engine::get_engine_static(&config, Some("ollama"))
            .map_err(|e| SUNDAYError::Agent(sunday_core::error::AgentError::Execution(e.to_string())))?);
        let adapter = sunday_engine::rig_adapter::RigModelAdapter::new(engine_arc.clone(), "qwen3:8b".to_string());
        
        let rt = tokio::runtime::Handle::current();

        let mut findings = Vec::new();

        for batch in rows.chunks(batch_size) {
            let mut batch_text = String::new();
            for (idx, (content, source, title, author)) in batch.iter().enumerate() {
                if idx > 0 {
                    batch_text.push_str("\n\n---\n\n");
                }
                batch_text.push_str(&format!("[{}] {} by {}:\n{}", source, title, author, content));
            }

            let prompt_text = format!(
                "/no_think\nExtract any information relevant to this question: {}\n\nIf nothing is relevant, reply with exactly: NOTHING_RELEVANT\n\nChunks:\n{}",
                question, batch_text
            );

            let msg = rig::completion::message::Message::user(&prompt_text);
            
            let result = rt.block_on(async {
                use rig::completion::request::CompletionModel;
                let request = adapter.completion_request(msg.clone());
                request.send().await
            });

            if let Ok(completion) = result {
                let content = match completion.choice.first_ref() {
                    rig::completion::AssistantContent::Text(t) => t.text().to_string(),
                    _ => String::new(),
                };
                let content = content.trim();
                if !content.is_empty() && !content.contains("NOTHING_RELEVANT") {
                    findings.push(content.to_string());
                }
            }
        }

        let mut res = if findings.is_empty() {
            ToolResult::success(
                self.tool_id(),
                &format!("Scanned {} chunks — no relevant information found.", rows.len()),
            )
        } else {
            ToolResult::success(self.tool_id(), &findings.join("\n\n"))
        };

        res.metadata.insert("chunks_scanned".to_string(), Value::Number(serde_json::Number::from(rows.len())));
        res.metadata.insert("batches_with_findings".to_string(), Value::Number(serde_json::Number::from(findings.len())));

        Ok(res)
    }
}
