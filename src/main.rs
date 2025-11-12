/// Funding Rate Arbitrage Bot - Main Entry Point
///
/// This bot automatically finds and executes delta neutral funding rate arbitrage
/// opportunities between Extended DEX and Pacifica.
///
/// Setup:
/// 1. Create .env file with credentials:
///    - API_KEY (Extended API key)
///    - SOL_WALLET, API_PUBLIC, API_PRIVATE (for Pacifica)
///    - STARK_PRIVATE, STARK_PUBLIC (for Extended trading)
///    - VAULT_NUMBER (Extended vault/position ID)
///
/// 2. Adjust config.json for desired filtering parameters
///
/// 3. Run: cargo run
///
use extended_connector::{
    FundingBot, OpportunityConfig, PacificaCredentials,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_line_number(true)
        .init();

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║         FUNDING RATE ARBITRAGE BOT (Extended/Pacifica)       ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    // Load environment variables
    dotenv::dotenv().ok();

    // Load credentials
    let extended_api_key = std::env::var("EXTENDED_API_KEY")
        .or_else(|_| std::env::var("API_KEY"))
        .ok();

    if extended_api_key.is_none() {
        eprintln!("⚠️  Warning: No Extended API key found. Some features may be limited.");
    }

    let pacifica_creds = PacificaCredentials::from_env()?;
    println!("✅ Loaded Pacifica credentials");

    // Load Starknet credentials for Extended trading
    let stark_private_key = std::env::var("STARK_PRIVATE")
        .expect("STARK_PRIVATE must be set in .env");
    let stark_public_key = std::env::var("STARK_PUBLIC")
        .expect("STARK_PUBLIC must be set in .env");
    let vault_id = std::env::var("VAULT_NUMBER")
        .expect("VAULT_NUMBER must be set in .env");
    println!("✅ Loaded Starknet credentials");

    // Load configuration
    let config = OpportunityConfig::load("config.json")?;
    println!("✅ Loaded config from config.json");
    println!("   • Min Volume: ${:.0}M", config.filters.min_combined_volume_usd / 1_000_000.0);
    println!("   • Max Intra Spread: {:.2}%", config.filters.max_intra_exchange_spread_pct);
    println!("   • Max Cross Spread: {:.2}%", config.filters.max_cross_exchange_spread_pct);
    println!("   • Min Net APR: {:.1}%", config.filters.min_net_apr_pct);
    println!("   • Max Position Size: ${:.0}", config.trading.max_position_size_usd);
    println!();

    // Create and run bot
    println!("🤖 Initializing bot...");
    let mut bot = FundingBot::new(
        extended_api_key.clone(),
        pacifica_creds,
        config,
        stark_private_key,
        stark_public_key,
        vault_id,
    )?;

    println!("✅ Bot initialized successfully");
    println!("⚡ Starting main bot loop...");
    println!();

    // Run bot (this will loop forever)
    bot.run(extended_api_key).await?;

    Ok(())
}
