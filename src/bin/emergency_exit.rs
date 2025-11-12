//! Emergency Exit Script
//!
//! Standalone binary that immediately closes ALL positions on both Extended and Pacifica.
//! Uses the bot's proven close logic with working signatures.
//!
//! Usage: cargo run --bin emergency_exit
//!
//! This script:
//! - Loads credentials from .env
//! - Initializes the bot infrastructure (which has working order placement)
//! - Closes all positions using the bot's proven close logic
//! - Reports success/failure
//!
//! WARNING: This will close ALL positions without confirmation!

use extended_connector::bot::FundingBot;
use extended_connector::opportunity::Config as OpportunityConfig;
use extended_connector::pacifica::PacificaCredentials;
use std::error::Error;
use tracing::{error, info, Level};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .compact()
        .init();

    info!("╔═══════════════════════════════════════════════════════════════╗");
    info!("║                    EMERGENCY EXIT INITIATED                   ║");
    info!("╠═══════════════════════════════════════════════════════════════╣");
    info!("║  This will close ALL positions on BOTH exchanges!            ║");
    info!("║  Starting in 3 seconds... Press Ctrl+C to abort              ║");
    info!("╚═══════════════════════════════════════════════════════════════╝");

    // Give user a chance to abort
    sleep(Duration::from_secs(3)).await;

    info!("\n🔄 Loading configuration...");

    // Load environment
    dotenv::dotenv().ok();

    // Load Extended credentials
    let api_key = std::env::var("API_KEY").ok();
    let stark_public = std::env::var("STARK_PUBLIC")?;
    let stark_private = std::env::var("STARK_PRIVATE")?;
    let vault_number = std::env::var("VAULT_NUMBER")?;

    // Load Pacifica credentials
    let pacifica_creds = PacificaCredentials::from_env()?;

    // Load config
    let config = OpportunityConfig::load("config.json")?;

    info!("✅ Configuration loaded");
    info!("\n🔄 Initializing bot with working close logic...");

    // Initialize bot with all required parameters
    // This uses the bot's infrastructure which has proven order placement
    let mut bot = FundingBot::new(
        api_key,
        pacifica_creds,
        config,
        stark_private,
        stark_public,
        vault_number,
    )?;

    info!("✅ Bot initialized\n");

    // Display current status
    info!("📊 Current positions:");
    let _ = bot.display_status().await;

    info!("\n🔥 Closing all positions using bot's proven close logic...");

    // Use the bot's proven close logic which has working signatures
    match bot.close_current_position().await {
        Ok(()) => {
            info!("\n╔═══════════════════════════════════════════════════════════════╗");
            info!("║                  EMERGENCY EXIT SUCCESSFUL                    ║");
            info!("╚═══════════════════════════════════════════════════════════════╝");
            info!("\n✅ All positions closed successfully!");
        }
        Err(e) => {
            error!("\n╔═══════════════════════════════════════════════════════════════╗");
            error!("║                     EMERGENCY EXIT FAILED                     ║");
            error!("╚═══════════════════════════════════════════════════════════════╝");
            error!("\n❌ Failed to close positions: {}", e);
            error!("\n⚠️  IMMEDIATE ACTIONS REQUIRED:");
            error!("   1. Close manually via exchange web interfaces:");
            error!("      - Extended: https://app.extended.exchange");
            error!("      - Pacifica: https://app.pacifica.fi");
            error!("   2. Try force_rotation: cargo run --bin force_rotation");
            error!("      (Sets rotation time to force close on next bot cycle)");
            error!("   3. Check error details above for specific issues");
            error!("\n⚠️  Some positions may still be open! Verify manually!");

            return Err(e);
        }
    }

    Ok(())
}
