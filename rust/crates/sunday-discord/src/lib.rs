use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready, id::UserId},
    prelude::*,
};
use std::sync::Arc;
use tracing::{info, error};
use sunday_tools::executor::ToolExecutor;
use sunday_engine::discover_engines;
use sunday_core::config::load_config;

pub struct Handler {
    pub model_name: String,
    pub bot_id: std::sync::Mutex<Option<UserId>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        // Get bot ID from cache or from the stored value from Ready event
        let bot_id = {
            let lock = self.bot_id.lock().unwrap();
            lock.unwrap_or_else(|| {
                // Fallback: try to get from HTTP if not cached yet
                // This is synchronous fallback - in practice Ready sets it
                UserId::new(0)
            })
        };

        // Skip if we don't have a valid bot ID yet
        if bot_id.get() == 0 {
            return;
        }

        let is_dm = msg.guild_id.is_none();
        let is_mentioned = msg.mentions.iter().any(|u| u.id == bot_id);

        if is_dm || is_mentioned {
            let mut content = msg.content.clone();
            if is_mentioned {
                content = content.replace(&format!("<@{}>", bot_id), "").trim().to_string();
                content = content.replace(&format!("<@!{}>", bot_id), "").trim().to_string();
            }

            if content.is_empty() {
                return;
            }

            info!("Discord Message from {}: {}", msg.author.name, content);

            // Emit Event to the Bus
            sunday_core::emit_event(
                sunday_core::EventType::BrainStatus,
                serde_json::json!({
                    "source": "discord",
                    "user": msg.author.name,
                    "action": "received_message",
                    "content_length": content.len()
                })
            );

            // Send typing indicator
            let _ = msg.channel_id.broadcast_typing(&ctx.http).await;
            let typing_msg = msg.channel_id.say(&ctx.http, "Thinking... 🧠").await;

            // Run Agent
            let reply = match self.run_agent(&content).await {
                Ok(res) => res,
                Err(e) => format!("Error: {}", e),
            };

            // Remove "Thinking..." message if possible, or just reply
            if let Ok(m) = typing_msg {
                let _ = m.delete(&ctx.http).await;
            }

            // Discord 2000 char limit
            let reply = if reply.len() > 1950 {
                format!("{}...", &reply[..1950])
            } else {
                reply
            };

            if let Err(why) = msg.channel_id.say(&ctx.http, reply).await {
                error!("Error sending message: {:?}", why);
            }
        }
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        // Store the bot's user ID for mention handling
        if let Ok(mut lock) = self.bot_id.lock() {
            *lock = Some(ready.user.id);
        }
    }
}

impl Handler {
    async fn run_agent(&self, input: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let config = load_config(None)?;
        let engines = discover_engines(&config);
        
        // Find the requested model or fallback to first available
        let _model = engines.iter()
            .find(|e| e.engine_id == self.model_name)
            .or_else(|| engines.first())
            .ok_or("No AI engines available")?;
            
        let _executor = Arc::new(ToolExecutor::new(None, None, None));
        // In a real implementation, we would populate the executor with tools
        
        // Note: OrchestratorAgent requires a CompletionModel, not EngineInfo.
        // For now, we return a placeholder since we'd need to construct the proper model.
        // let agent = OrchestratorAgent::new(
        //     model,
        //     "You are SUNDAY, a helpful AI assistant on Discord.",
        //     executor,
        //     10,
        // );
        // let result = agent.run(input, None).await?;
        // Ok(result.content)
        
        Ok(format!("SUNDAY received: {}", input))
    }
}

pub async fn start_bot(token: &str, model_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(token, intents)
        .event_handler(Handler { 
            model_name: model_name.to_string(),
            bot_id: std::sync::Mutex::new(None),
        })
        .await?;

    client.start().await?;
    Ok(())
}
