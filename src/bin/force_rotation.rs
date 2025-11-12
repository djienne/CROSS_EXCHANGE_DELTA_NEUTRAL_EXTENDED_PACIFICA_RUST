//! Force Rotation Script
//!
//! This script modifies bot_state.json to force immediate rotation.
//! The bot's own working close logic will then close positions on next check.
//!
//! Usage: cargo run --bin force_rotation
//!
//! This is safer than direct API calls since it uses the bot's proven close logic.

use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use tracing::{info, Level};

const STATE_FILE: &str = "bot_state.json";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .compact()
        .init();

    info!("╔═══════════════════════════════════════════════════════════════╗");
    info!("║                    FORCE ROTATION SCRIPT                      ║");
    info!("╠═══════════════════════════════════════════════════════════════╣");
    info!("║  This will set rotation time to 48+ hours ago                ║");
    info!("║  The bot will close positions on next monitoring cycle       ║");
    info!("╚═══════════════════════════════════════════════════════════════╝\n");

    // Read current state
    let state_content = fs::read_to_string(STATE_FILE)
        .context(format!("Failed to read {}", STATE_FILE))?;

    let mut state: Value = serde_json::from_str(&state_content)
        .context("Failed to parse bot_state.json")?;

    // Check if there's a current position
    if state.get("current_position").is_none() || state["current_position"].is_null() {
        info!("✅ No active position found. Nothing to force.");
        return Ok(());
    }

    // Get current time
    let now = chrono::Utc::now().timestamp();

    // Set last_rotation_time to 49 hours ago (forces immediate rotation)
    let forced_time = now - (49 * 3600);

    info!("Current timestamp: {}", now);
    info!("Setting last_rotation_time to: {} (49 hours ago)", forced_time);

    state["last_rotation_time"] = serde_json::json!(forced_time);

    // Write back to file
    let state_json = serde_json::to_string_pretty(&state)?;
    fs::write(STATE_FILE, state_json)
        .context(format!("Failed to write {}", STATE_FILE))?;

    info!("\n✅ Successfully forced rotation time!");
    info!("\n📌 Next steps:");
    info!("   1. The bot is probably already running");
    info!("   2. It will detect the expired position on next check (within 15 min)");
    info!("   3. It will close positions using its working close logic");
    info!("\n   OR run: cargo run  (to start bot manually if not running)");

    Ok(())
}
