//! Crypto price tool using public APIs.

use crate::traits::BaseTool;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;

static SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "get_crypto_price".into(),
    description: "Get the current price of a cryptocurrency (e.g., bitcoin, ethereum)".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "symbol": { "type": "string", "description": "Cryptocurrency symbol or name (e.g., btc, bitcoin)" },
            "currency": { "type": "string", "description": "Target currency (default: usd)" }
        },
        "required": ["symbol"]
    }),
    category: "finance".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.5,
    requires_confirmation: false,
    timeout_seconds: 10.0,
    required_capabilities: vec!["network:fetch".into()],
    metadata: HashMap::new(),
});

pub struct CryptoPriceTool;

impl BaseTool for CryptoPriceTool {
    fn tool_id(&self) -> &str {
        "get_crypto_price"
    }
    fn spec(&self) -> &ToolSpec {
        &SPEC
    }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let symbol = params["symbol"].as_str().unwrap_or("bitcoin").to_lowercase();
        let currency = params["currency"].as_str().unwrap_or("usd").to_lowercase();

        // Map common symbols to Coingecko IDs
        let id = match symbol.as_str() {
            "btc" => "bitcoin",
            "eth" => "ethereum",
            "sol" => "solana",
            "bnb" => "binancecoin",
            _ => &symbol,
        };

        let url = format!("https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies={}", id, currency);
        
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| SUNDAYError::Io(std::io::Error::other(e.to_string())))?;

        match client.get(url).send() {
            Ok(resp) => {
                let data: Value = resp.json().unwrap_or(serde_json::json!({}));
                if let Some(price) = data[id][&currency].as_f64() {
                    Ok(ToolResult::success("get_crypto_price", format!("Current {} price: {} {}", symbol.to_uppercase(), price, currency.to_uppercase())))
                } else {
                    Ok(ToolResult::failure("get_crypto_price", "Could not find price for that symbol."))
                }
            }
            Err(e) => Ok(ToolResult::failure("get_crypto_price", format!("API request failed: {}", e))),
        }
    }
}
