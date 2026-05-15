use crate::traits::{InferenceEngine, TokenStream};
use sunday_core::error::{EngineError, SUNDAYError};
use sunday_core::{GenerateResult, Message, Usage};
use sunday_core::events::{emit_event, EventType};
use sunday_core::shared_mem::SharedMemorySegment;
use serde_json::Value;
use std::sync::Arc;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::params::LlamaModelParams;

pub struct NativeLlamaEngine {
    model: LlamaModel,
    backend: Arc<LlamaBackend>,
    model_path: String,
}

impl NativeLlamaEngine {
    pub fn new(model_path: &str) -> Result<Self, SUNDAYError> {
        let backend = LlamaBackend::init()
            .map_err(|e| SUNDAYError::Engine(EngineError::Initialization(format!("Failed to init llama backend: {}", e))))?;
        let backend = Arc::new(backend);

        let model_params = LlamaModelParams::default();
        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
            .map_err(|e| SUNDAYError::Engine(EngineError::Initialization(format!("Failed to load model: {}", e))))?;
        
        Ok(Self {
            model,
            backend,
            model_path: model_path.to_string(),
        })
    }

    fn messages_to_prompt(messages: &[Message]) -> String {
        let mut prompt = String::new();
        for msg in messages {
            let role = msg.role.to_string();
            let mut content = msg.content.clone();
            
            // Resolve SHM tags: @shm:name
            if content.contains("@shm:") {
                // We'll use a simple search and replace or regex if needed.
                // For simplicity in this env, let's use a manual check or regex if available.
                let re = regex::Regex::new(r"@shm:([a-zA-Z0-9_-]+)").unwrap();
                content = re.replace_all(&content, |caps: &regex::Captures| {
                    let name = &caps[1];
                    let shm = SharedMemorySegment::new(name);
                    match shm.read() {
                        Ok(data) => {
                            if let Ok(json) = serde_json::from_slice::<Value>(&data) {
                                serde_json::to_string_pretty(&json).unwrap_or_else(|_| String::from_utf8_lossy(&data).to_string())
                            } else {
                                String::from_utf8_lossy(&data).to_string()
                            }
                        },
                        Err(_) => format!("[Error: SHM segment {} not found]", name)
                    }
                }).to_string();
            }

            prompt.push_str(&format!("<|{}|>\n{}\n", role, content));
        }
        prompt.push_str("<|assistant|>\n");
        prompt
    }
}

#[async_trait::async_trait]
impl InferenceEngine for NativeLlamaEngine {
    fn engine_id(&self) -> &str {
        "native"
    }

    fn generate(
        &self,
        messages: &[Message],
        _model: &str,
        _temperature: f64,
        max_tokens: i64,
        _extra: Option<&Value>,
    ) -> Result<GenerateResult, SUNDAYError> {
        use llama_cpp_2::llama_batch::LlamaBatch;
        use llama_cpp_2::token::data_array::LlamaTokenDataArray;
        use llama_cpp_2::model::AddBos;

        emit_event(EventType::InferenceStart, serde_json::json!({
            "engine": "native",
            "model_path": self.model_path,
        }));

        let ctx_params = LlamaContextParams::default();
        let mut ctx = self.model.new_context(&self.backend, ctx_params)
            .map_err(|e| SUNDAYError::Engine(EngineError::Initialization(format!("Failed to create context: {}", e))))?;

        let prompt = Self::messages_to_prompt(messages);
        let tokens = self.model.str_to_token(&prompt, AddBos::Always)
            .map_err(|e| SUNDAYError::Engine(EngineError::Generation(format!("Tokenization failed: {}", e))))?;

        let n_ctx = ctx.n_ctx() as usize;
        let n_tokens = tokens.len();

        if n_tokens > n_ctx {
            return Err(SUNDAYError::Engine(EngineError::Generation("Prompt too long".into())));
        }

        // Evaluate prompt
        let mut batch = LlamaBatch::new(512, 1);
        for (i, &token) in tokens.iter().enumerate() {
            let is_last = i == n_tokens - 1;
            let _ = batch.add(token, i as i32, &[0], is_last);
        }

        ctx.decode(&mut batch)
            .map_err(|e| SUNDAYError::Engine(EngineError::Generation(format!("Decode failed: {}", e))))?;

        let mut output = String::new();
        let mut n_predict = 0;
        let mut decoder = encoding_rs::UTF_8.new_decoder();

        while n_predict < max_tokens {
            let candidates = ctx.candidates_ith(batch.n_tokens() as i32 - 1);
            let mut candidates_p = LlamaTokenDataArray::from_iter(candidates, false);
            let token = candidates_p.sample_token_greedy();

            if self.model.is_eog_token(token) {
                break;
            }

            let token_str = self.model.token_to_piece(token, &mut decoder, false, None)
                .map_err(|e| SUNDAYError::Engine(EngineError::Generation(format!("Token conversion failed: {}", e))))?;
            output.push_str(&token_str);

            n_predict += 1;

            batch.clear();
            let _ = batch.add(token, (n_tokens + n_predict as usize - 1) as i32, &[0], true);
            ctx.decode(&mut batch)
                .map_err(|e| SUNDAYError::Engine(EngineError::Generation(format!("Decode failed: {}", e))))?;
        }

        let result = GenerateResult {
            content: output,
            usage: Usage {
                prompt_tokens: n_tokens as i64,
                completion_tokens: n_predict as i64,
                total_tokens: (n_tokens + n_predict as usize) as i64,
            },
            model: self.model_path.clone(),
            finish_reason: "stop".to_string(),
            tool_calls: None,
            ttft: 0.0,
            cost_usd: 0.0,
            metadata: std::collections::HashMap::new(),
        };

        emit_event(EventType::InferenceEnd, serde_json::json!({
            "engine": "native",
            "usage": result.usage,
        }));

        Ok(result)
    }

    async fn stream(
        &self,
        _messages: &[Message],
        _model: &str,
        _temperature: f64,
        _max_tokens: i64,
        _extra: Option<&Value>,
    ) -> Result<TokenStream, SUNDAYError> {
        Err(SUNDAYError::Engine(EngineError::NotImplemented("Native streaming not yet implemented".into())))
    }

    fn list_models(&self) -> Result<Vec<String>, SUNDAYError> {
        Ok(vec![self.model_path.clone()])
    }

    fn health(&self) -> bool {
        true
    }
}
