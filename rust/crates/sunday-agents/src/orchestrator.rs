//! OrchestratorAgent — high-performance multi-turn tool loop.
//!
//! Handles complex response parsing, loop detection, and hybrid routing
//! between local and cloud models.

use crate::traits::OjAgent;
use crate::utils::strip_think_tags;
use sunday_core::{AgentContext, AgentResult, SUNDAYError, Role, Message as SundayMessage};
use sunday_tools::executor::ToolExecutor;
use rig::completion::request::CompletionModel;
use rig::completion::message::Message as RigMessage;
use std::collections::HashMap;
use std::sync::Arc;
use regex::Regex;
use once_cell::sync::Lazy;
use sha2::{Sha256, Digest};

// Compile regexes once for performance
static THOUGHT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)(?:THOUGHT|Thinking Process|Reasoning|<thought>):\s*(.+?)(?=\nTOOL:|\nFINAL[_ ]?ANSWER:|</thought>|\z)").unwrap());
static FINAL_ANSWER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)FINAL[_ ]?ANSWER:\s*(.+)").unwrap());
static XML_TOOL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?s)<tool_call>\s*(\{.*?\})\s*</tool_call>").unwrap());
static INLINE_TOOL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)TOOL:\s*([\w_]+)\s*\((.+?)\)(?=\n||\z)").unwrap());
static TOOL_NAME_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)TOOL:\s*([\w_]+)").unwrap());
static INPUT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)INPUT:\s*(.+?)(?=\n(?:THOUGHT|Thinking Process|Reasoning):|\nTOOL:|\nFINAL|\z)").unwrap());

#[derive(Default)]
struct StateTracker {
    states: Vec<String>,
}

impl StateTracker {
    fn record(&mut self, tool_name: &str, result: &str) {
        if !tool_name.starts_with("browser_") {
            return;
        }
        
        let mut hasher = Sha256::new();
        // Use first 2000 chars for hash to be efficient
        let limit = result.len().min(2000);
        hasher.update(&result.as_bytes()[..limit]);
        let hash = format!("{:x}", hasher.finalize());
        
        // Extract URL if present
        let mut url = "";
        for line in result.lines().take(5) {
            if line.contains("http") {
                url = line.trim();
                if url.len() > 120 { url = &url[..120]; }
                break;
            }
        }
        
        self.states.push(format!("{}|{}", url, &hash[..8]));
    }

    fn is_stuck(&self, window: usize) -> bool {
        if self.states.len() < window {
            return false;
        }
        let recent = &self.states[self.states.len() - window..];
        recent.iter().all(|s| s == &recent[0])
    }
}

struct ParsedResponse {
    #[allow(dead_code)]
    thought: String,
    tool: Option<String>,
    input: Option<String>,
    final_answer: Option<String>,
}

/// Multi-turn agent with function calling and loop detection.
pub struct OrchestratorAgent<M: CompletionModel> {
    model: M,
    executor: Arc<ToolExecutor>,
    max_turns: usize,
    system_prompt: String,
}

impl<M: CompletionModel> OrchestratorAgent<M> {
    pub fn new(
        model: M,
        system_prompt: &str,
        executor: Arc<ToolExecutor>,
        max_turns: usize,
    ) -> Self {
        Self {
            model,
            executor,
            max_turns,
            system_prompt: system_prompt.to_string(),
        }
    }

    fn classify_task(&self, messages: &[SundayMessage]) -> &'static str {
        let full_text = messages.iter()
            .map(|m| m.content.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
            
        let tool_keywords = ["โรงแรม", "hotel", "booking", "จอง", "ค้นหา", "ราคา", "price", "research", "paper", "amazon", "browse", "browser", "web_search", "search"];
        
        if tool_keywords.iter().any(|&k| full_text.contains(k)) {
            "cloud"
        } else {
            "local"
        }
    }

    fn parse_response(&self, text: &str) -> ParsedResponse {
        let mut thought = String::new();
        if let Some(caps) = THOUGHT_RE.captures(text) {
            thought = caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
        }

        // Priority 1: FINAL_ANSWER
        if let Some(caps) = FINAL_ANSWER_RE.captures(text) {
            return ParsedResponse {
                thought,
                tool: None,
                input: None,
                final_answer: Some(caps.get(1).map_or("", |m| m.as_str()).trim().to_string()),
            };
        }

        // Priority 2: XML Tool Call
        if let Some(caps) = XML_TOOL_RE.captures(text) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(caps.get(1).map_or("", |m| m.as_str())) {
                return ParsedResponse {
                    thought,
                    tool: val.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    input: val.get("arguments").map(|v| v.to_string()),
                    final_answer: None,
                };
            }
        }

        // Priority 3: Inline Tool Call
        if let Some(caps) = INLINE_TOOL_RE.captures(text) {
            return ParsedResponse {
                thought,
                tool: Some(caps.get(1).map_or("", |m| m.as_str()).to_string()),
                input: Some(caps.get(2).map_or("", |m| m.as_str()).to_string()),
                final_answer: None,
            };
        }

        // Priority 4: Loose TOOL/INPUT
        let tool = TOOL_NAME_RE.captures(text).map(|c| c.get(1).map_or("", |m| m.as_str()).to_string());
        let input = INPUT_RE.captures(text).map(|c| c.get(1).map_or("", |m| m.as_str()).trim().to_string());

        ParsedResponse {
            thought,
            tool,
            input,
            final_answer: None,
        }
    }
}

#[async_trait::async_trait]
impl<M: CompletionModel + 'static> OjAgent for OrchestratorAgent<M> {
    fn agent_id(&self) -> &str {
        "orchestrator"
    }

    fn accepts_tools(&self) -> bool {
        true
    }

    async fn run(
        &self,
        input: &str,
        context: Option<&AgentContext>,
    ) -> Result<AgentResult, SUNDAYError> {
        let mut messages = vec![SundayMessage::new(Role::System, &self.system_prompt)];
        
        if let Some(ctx) = context {
            for m in &ctx.conversation.messages {
                messages.push(m.clone());
            }
        }
        messages.push(SundayMessage::new(Role::User, input));

        let mut all_tool_results = Vec::new();
        let mut state_tracker = StateTracker::default();
        let mut turns = 0;

        let task_tier = self.classify_task(&messages);
        tracing::info!("[🧠 {}] Orchestration starting...", task_tier);

        while turns < self.max_turns {
            turns += 1;
            
            // Generate response
            let mut prompt_msgs = Vec::new();
            let mut preamble = String::new();

            for m in &messages {
                match m.role {
                    Role::System => preamble = m.content.clone(),
                    Role::User => prompt_msgs.push(RigMessage::user(&m.content)),
                    Role::Assistant => prompt_msgs.push(RigMessage::assistant(&m.content)),
                    _ => {}
                }
            }

            // Rig-core: completion_request takes the current prompt, and .messages() takes the history.
            // We'll use the last message as the current prompt.
            let last_msg = prompt_msgs.pop().ok_or_else(|| {
                SUNDAYError::Agent(sunday_core::error::AgentError::Execution("Empty message history".to_string()))
            })?;

            let mut request = self.model.completion_request(last_msg)
                .messages(prompt_msgs);
            
            if !preamble.is_empty() {
                request = request.preamble(preamble);
            }

            let completion = request.send()
                .await
                .map_err(|e| SUNDAYError::Agent(sunday_core::error::AgentError::Execution(e.to_string())))?;

            let content = match completion.choice.first_ref() {
                rig::completion::AssistantContent::Text(t) => t.text().to_string(),
                _ => String::new(),
            };
            
            let parsed = self.parse_response(&content);

            // Handle Final Answer
            if let Some(answer) = parsed.final_answer {
                return Ok(AgentResult {
                    content: strip_think_tags(&answer),
                    tool_results: all_tool_results,
                    turns,
                    metadata: HashMap::new(),
                });
            }

            // Handle Tool Call
            if let Some(tool_name) = parsed.tool {
                messages.push(SundayMessage::new(Role::Assistant, &content));
                
                let args_json = parsed.input.unwrap_or_else(|| "{}".to_string());
                let args: serde_json::Value = serde_json::from_str(&args_json).unwrap_or(serde_json::json!({}));

                tracing::info!("[🛠️  EXECUTING] {} with {:?}", tool_name, args);
                
                let tool_result = self.executor.execute(
                    &tool_name,
                    &args,
                    Some(self.agent_id()),
                    None,
                )
                .map_err(|e| SUNDAYError::Agent(sunday_core::error::AgentError::Execution(e.to_string())))?;
                
                state_tracker.record(&tool_name, &tool_result.content);
                
                if state_tracker.is_stuck(3) {
                    tracing::warn!("[🔴 STUCK] Browser loop detected!");
                    messages.push(SundayMessage::new(Role::User, 
                        "[⚠️ STUCK DETECTED] The page has not changed. Try a completely different approach (search or navigate)."));
                    state_tracker.states.clear();
                } else {
                    messages.push(SundayMessage::new(Role::User, &format!("Observation: {}", tool_result.content)));
                }

                all_tool_results.push(tool_result);
            } else {
                // No tool found but not a final answer -> treat as final answer but warn
                return Ok(AgentResult {
                    content: strip_think_tags(&content),
                    tool_results: all_tool_results,
                    turns,
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(AgentResult {
            content: "Maximum turns reached without a final answer.".to_string(),
            tool_results: all_tool_results,
            turns,
            metadata: HashMap::new(),
        })
    }
}
