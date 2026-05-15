use std::env;
use sunday_discord::start_bot;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_BOT_TOKEN")
        .expect("Expected DISCORD_BOT_TOKEN in environment");
    
    let model = env::var("SUNDAY_MODEL")
        .unwrap_or_else(|_| "qwen3.5:9b".to_string());

    println!("Starting SUNDAY Discord Daemon (Rust)...");
    start_bot(&token, &model).await?;

    Ok(())
}
