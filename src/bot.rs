/// Funding rate arbitrage bot orchestration and state management
use crate::{
    OpportunityFinder, RestClient, PacificaTrading, PacificaCredentials,
    trading::{open_delta_neutral_position, close_delta_neutral_position, calculate_position_size, DeltaNeutralPosition},
    OpportunityConfig,
};
use crate::pacifica::types::PacificaPosition;
use crate::pacifica::PacificaWsTrading;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;
use futures_util::FutureExt;
use tracing::{info, warn, error};

const STATE_FILE: &str = "bot_state.json";
const MONITORING_INTERVAL_MINUTES: u64 = 15;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotState {
    pub current_position: Option<DeltaNeutralPosition>,
    pub last_rotation_time: Option<u64>,
    pub total_rotations: u64,
}

impl BotState {
    pub fn new() -> Self {
        Self {
            current_position: None,
            last_rotation_time: None,
            total_rotations: 0,
        }
    }

    /// Load state from JSON file
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        if Path::new(path).exists() {
            let content = fs::read_to_string(path)?;
            let state: BotState = serde_json::from_str(&content)?;
            info!("Loaded bot state from {}: {} rotations, position: {}",
                path,
                state.total_rotations,
                if state.current_position.is_some() { "active" } else { "none" }
            );
            Ok(state)
        } else {
            info!("No existing state file found, starting fresh");
            Ok(Self::new())
        }
    }

    /// Save state to JSON file
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        info!("Saved bot state to {}", path);
        Ok(())
    }

    /// Check if current position should be rotated based on configured hold time
    pub fn should_rotate(&self, hold_time_hours: u64) -> bool {
        if let Some(pos) = &self.current_position {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let elapsed_hours = (now - pos.opened_at) / 3600;
            elapsed_hours >= hold_time_hours
        } else {
            false
        }
    }

    /// Get time remaining until rotation (in hours)
    pub fn hours_until_rotation(&self, hold_time_hours: u64) -> Option<f64> {
        if let Some(pos) = &self.current_position {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let elapsed_hours = (now - pos.opened_at) as f64 / 3600.0;
            let remaining = hold_time_hours as f64 - elapsed_hours;
            Some(remaining.max(0.0))
        } else {
            None
        }
    }
}

pub struct FundingBot {
    extended_client: RestClient,
    pacifica_client: PacificaTrading,
    pacifica_creds: PacificaCredentials,
    opportunity_finder: OpportunityFinder,
    config: OpportunityConfig,
    state: BotState,
    stark_private_key: String,
    stark_public_key: String,
    vault_id: String,
}

impl FundingBot {
    pub fn new(
        extended_api_key: Option<String>,
        pacifica_creds: PacificaCredentials,
        config: OpportunityConfig,
        stark_private_key: String,
        stark_public_key: String,
        vault_id: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let extended_client = RestClient::new_mainnet(extended_api_key.clone())?;
        let pacifica_client = PacificaTrading::new(pacifica_creds.clone());
        let opportunity_finder = OpportunityFinder::new(
            extended_api_key.clone(),
            pacifica_creds.clone(),
            config.clone(),
        )?;

        let state = BotState::load_from_file(STATE_FILE)?;

        Ok(Self {
            extended_client,
            pacifica_client,
            pacifica_creds,
            opportunity_finder,
            config,
            state,
            stark_private_key,
            stark_public_key,
            vault_id,
        })
    }

    /// Reconcile saved state with live exchange positions.
    /// If saved state indicates an active position but neither exchange has it,
    /// clear the state to avoid erroneous closes/rotations. If only one leg exists,
    /// keep that leg in state so a subsequent close will only act on the live leg.
    pub async fn reconcile_state(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(saved_pos) = self.state.current_position.clone() else {
            // Nothing to reconcile
            return Ok(());
        };

        let symbol = saved_pos.symbol.clone();
        let extended_market = format!("{}-USD", symbol);

        // Query live positions on Extended (ignore errors to avoid false clearing)
        let live_ext: Option<crate::types::Position> = match self
            .extended_client
            .get_positions(Some(&extended_market))
            .await
        {
            Ok(list) => list.into_iter().find(|p| p.market == extended_market),
            Err(e) => {
                warn!("Skipping Extended reconciliation (could not fetch positions): {}", e);
                None
            }
        };

        // Query live position on Pacifica
        let live_pac: Option<PacificaPosition> = match self.pacifica_client.get_position(&symbol).await {
            Ok(pos_opt) => pos_opt,
            Err(e) => {
                warn!("Skipping Pacifica reconciliation (could not fetch positions): {}", e);
                None
            }
        };

        // If both legs are missing, clear state
        if live_ext.is_none() && live_pac.is_none() {
            warn!(
                "State shows active {}, but no live positions found on either exchange. Clearing stale state.",
                symbol
            );
            self.state.current_position = None;
            self.state.save_to_file(STATE_FILE)?;
            return Ok(());
        }

        // Otherwise, update state to reflect only the legs that actually exist
        let mut updated = saved_pos.clone();
        updated.extended_position = live_ext;
        updated.pacifica_position = live_pac;
        self.state.current_position = Some(updated);
        self.state.save_to_file(STATE_FILE)?;
        Ok(())
    }

    /// Display current status summary
    pub async fn display_status(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("╔═══════════════════════════════════════════════════════════════╗");
        info!("║                  {}                      ║",
            "FUNDING RATE BOT STATUS");
        info!("╠═══════════════════════════════════════════════════════════════╣");

        if let Some(pos) = &self.state.current_position {
            let hold_time_hours = self.config.trading.hold_time_hours;
            let hours_remaining = self.state.hours_until_rotation(hold_time_hours).unwrap_or(0.0);

            // Convert opened_at timestamp to datetime
            let opened_datetime = chrono::DateTime::from_timestamp(pos.opened_at as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            // Calculate rotation time
            let rotation_timestamp = pos.opened_at + (hold_time_hours * 3600);
            let rotation_datetime = chrono::DateTime::from_timestamp(rotation_timestamp as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let hours_formatted = format!("{:>37.1} hours", hours_remaining);

            info!("║ Symbol:              {:<43} ║", pos.symbol);
            info!("║ Notional:            {:<43} ║", format!("${:.2}", pos.target_notional_usd));
            info!("║ Opened:              {:<43} ║", opened_datetime);
            info!("║ Rotation:            {:<43} ║", rotation_datetime);
            info!("║ Time Remaining:      {:>43} ║", hours_formatted);
            info!("║ Extended Position:   {:<42} ║",
                if pos.extended_position.is_some() {
                    "ACTIVE"
                } else {
                    "NONE"
                }
            );
            info!("║ Pacifica Position:   {:<42} ║",
                if pos.pacifica_position.is_some() {
                    "ACTIVE"
                } else {
                    "NONE"
                }
            );

            // Fetch current positions for PnL display
            if let Ok(extended_positions) = self.extended_client.get_positions(None).await {
                if let Some(ext_pos) = extended_positions.iter().find(|p| p.market.starts_with(&pos.symbol)) {
                    let pnl = ext_pos.unrealized_pnl.as_ref()
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);

                    let pnl_formatted = format!("${:.2}", pnl);

                    info!("║ Extended PnL:        {:>43} ║", pnl_formatted);
                }
            }

            if let Ok(pacifica_positions) = self.pacifica_client.get_positions().await {
                if let Some(pac_pos) = pacifica_positions.iter().find(|p| p.symbol == pos.symbol) {
                    let entry = pac_pos.entry();
                    let size = pac_pos.size();
                    info!("║ Pacifica Entry:      {:<43} ║", format!("${:.2}", entry));
                    info!("║ Pacifica Size:       {:>43} ║", format!("{:.6}", size));
                }
            }
        } else {
            info!("║ Status: {}                                    ║",
                "NO ACTIVE POSITION");
        }

        info!("║ Total Rotations:     {:>43} ║", self.state.total_rotations.to_string());
        info!("╚═══════════════════════════════════════════════════════════════╝");

        Ok(())
    }

    /// Find and open the best opportunity
    pub async fn open_best_opportunity(
        &mut self,
        extended_api_key: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("{}", "🔍 Scanning for best opportunity...");

        let scan_result = self.opportunity_finder.scan(extended_api_key.clone()).await?;

        // Display comprehensive scan summary
        scan_result.display_summary(&self.config.filters);

        if scan_result.opportunities.is_empty() {
            warn!("{}", "No opportunities found matching criteria");
            return Ok(());
        }

        let best = &scan_result.opportunities[0];
        info!("{} {} {}",
            "✅ Selected best opportunity:",
            best.symbol,
            format!("(Net APR: {:.2}%)", best.best_net_apr));
        info!("   {}: {}", "Strategy", best.best_direction);
        info!("   {}: ${:.0}", "Volume", best.total_volume_24h);
        info!("   {}: Ext {:.3}%, Pac {:.3}%, Cross {:.3}%",
            "Spreads",
            best.extended_spread_pct, best.pacifica_spread_pct, best.cross_spread_pct);

        // Determine position direction
        let long_on_extended = best.best_direction.contains("Long Extended");

        // Get market symbols
        let extended_market = format!("{}-USD", best.symbol);
        let pacifica_market = best.symbol.clone();

        // Fetch current prices and account info
        let extended_balance = self.extended_client.get_balance().await?;
        let extended_free = extended_balance.available_for_trade.parse::<f64>()?;

        // Fetch Pacifica account balance via WebSocket
        let pacifica_ws = PacificaWsTrading::new(self.pacifica_creds.clone(), false); // false = mainnet
        let pacifica_account_info = pacifica_ws.get_account_info().await?;
        let pacifica_free = pacifica_account_info.available_to_spend_f64();

        info!("{} {}", "💰 Extended free collateral:", format!("${:.2}", extended_free));
        info!("{} {}", "💰 Pacifica free collateral:", format!("${:.2}", pacifica_free));

        // Get lot sizes
        let extended_market_config = self.extended_client.get_market_config(&extended_market).await?;
        let extended_lot_size = extended_market_config.trading_config.min_order_size_change.parse::<f64>()?;

        let pacifica_markets = self.pacifica_client.get_market_info().await?;
        let pacifica_market_info = pacifica_markets.get(&pacifica_market)
            .ok_or_else(|| format!("Pacifica market {} not found", pacifica_market))?;
        let pacifica_lot_size = pacifica_market_info.lot_size.parse::<f64>()?;

        // Get current price
        let orderbook = self.extended_client.get_orderbook(&extended_market).await?;
        let current_price = if let (Some(bid), Some(ask)) = (orderbook.bid.first(), orderbook.ask.first()) {
            let bid_price = bid.price.parse::<f64>()?;
            let ask_price = ask.price.parse::<f64>()?;
            (bid_price + ask_price) / 2.0
        } else {
            return Err("No orderbook data available".into());
        };

        // Calculate position size
        let position_size = calculate_position_size(
            extended_free,
            pacifica_free,
            extended_lot_size,
            pacifica_lot_size,
            current_price,
            self.config.trading.max_position_size_usd,
        );

        if position_size <= 0.0 {
            return Err("Insufficient capital to open position".into());
        }

        info!("{} {:.6} {} ({})",
            "📊 Calculated position size:",
            position_size,
            best.symbol,
            format!("${:.2}", position_size * current_price));

        // Open delta neutral position
        let position = open_delta_neutral_position(
            &best.symbol,
            long_on_extended,
            position_size,
            current_price,
            &self.extended_client,
            &mut self.pacifica_client,
            &extended_market,
            &pacifica_market,
            &self.stark_private_key,
            &self.stark_public_key,
            &self.vault_id,
        ).await.map_err(|e| format!("Failed to open position: {}", e))?;

        // Update state
        self.state.current_position = Some(position);
        self.state.last_rotation_time = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs()
        );
        self.state.total_rotations += 1;
        self.state.save_to_file(STATE_FILE)?;

        info!("{}", "✅ Position opened successfully!");

        Ok(())
    }

    /// Close the current position
    pub async fn close_current_position(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.state.current_position.is_some() {
            // Ensure state matches live positions before attempting close
            self.reconcile_state().await.ok();

            // If reconciliation cleared the state, nothing to do
            if self.state.current_position.is_none() {
                warn!("No live positions to close after reconciliation");
                return Ok(());
            }

            let pos = self.state.current_position.as_ref().unwrap();
            info!("{} {}", "🔄 Closing current position:", pos.symbol);

            close_delta_neutral_position(
                pos,
                &self.extended_client,
                &mut self.pacifica_client,
                &self.stark_private_key,
                &self.stark_public_key,
                &self.vault_id,
            ).await.map_err(|e| format!("Failed to close position: {}", e))?;

            // Clear position from state
            self.state.current_position = None;
            self.state.save_to_file(STATE_FILE)?;

            info!("{}", "✅ Position closed successfully!");
        } else {
            warn!("{}", "No active position to close");
        }

        Ok(())
    }

    /// Main bot loop
    pub async fn run(&mut self, extended_api_key: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        info!("{}", "🚀 Starting Funding Rate Arbitrage Bot");
        info!("{} {} {}",
            "📊 Monitoring interval:",
            MONITORING_INTERVAL_MINUTES,
            "minutes");
        info!("{} {} {}",
            "⏱️  Position hold time:",
            self.config.trading.hold_time_hours,
            "hours");
        info!("{}", "🛑 Press Ctrl+C to stop gracefully");

        loop {
            // Non-blocking check for Ctrl+C (gracefully exit; keep positions open)
            if tokio::signal::ctrl_c().now_or_never().is_some() {
                info!("{}", "");
                info!("{}", "🛑 Shutdown signal received. Stopping bot gracefully...");
                info!("{}", "ℹ️  Open positions (if any) will remain open.");
                info!("{}", "   Manage them from the exchange dashboards or restart the bot.");
                info!("{}", "👋 Bot stopped. Goodbye!");
                return Ok(());
            }

            // Reconcile any stale state before acting
            self.reconcile_state().await.ok();

            // Display status
            self.display_status().await?;

            // Always scan and display opportunities at start of each cycle
            info!("");
            info!("{}", "🔍 Scanning current market opportunities...");
            if let Ok(scan_result) = self.opportunity_finder.scan(extended_api_key.clone()).await {
                scan_result.display_summary(&self.config.filters);
            } else {
                warn!("{}", "Failed to scan opportunities");
            }
            info!("");

            // Check if we need to rotate
            if self.state.should_rotate(self.config.trading.hold_time_hours) {
                info!("{} {} {}",
                    "⏰ Position has been open for",
                    self.config.trading.hold_time_hours,
                    "hours, rotating...");

                // Close current position
                if let Err(e) = self.close_current_position().await {
                    error!("{} {}", "Failed to close position:", e);
                    info!("{}", "Will retry next cycle.");
                    sleep(Duration::from_secs(MONITORING_INTERVAL_MINUTES * 60)).await;
                    continue;
                }

                // Wait a bit before opening new position
                sleep(Duration::from_secs(5)).await;

                // Open new position
                if let Err(e) = self.open_best_opportunity(extended_api_key.clone()).await {
                    error!("{} {}", "Failed to open new position:", e);
                    info!("{}", "Will retry next cycle.");
                }
            } else if self.state.current_position.is_none() {
                // No position, try to open one
                info!("{}", "📭 No active position, looking for opportunity...");

                if let Err(e) = self.open_best_opportunity(extended_api_key.clone()).await {
                    error!("{} {}", "Failed to open position:", e);
                    info!("{}", "Will retry next cycle.");
                }
            } else {
                // Position active, just monitoring
                if let Some(hours) = self.state.hours_until_rotation(self.config.trading.hold_time_hours) {
                    info!("{} {} {}",
                        "⏳ Position active,",
                        format!("{:.1}", hours),
                        "hours until rotation");
                }
            }

            // Wait for next monitoring cycle (interruptible by Ctrl+C)
            info!("{} {} {}",
                "😴 Sleeping for",
                MONITORING_INTERVAL_MINUTES,
                "minutes...");
            let wait = sleep(Duration::from_secs(MONITORING_INTERVAL_MINUTES * 60));
            tokio::pin!(wait);
            tokio::select! {
                _ = &mut wait => {},
                _ = tokio::signal::ctrl_c() => {
                    info!("{}", "");
                    info!("{}", "🛑 Shutdown signal received during sleep. Stopping gracefully.");
                    info!("{}", "ℹ️  Open positions (if any) will remain open.");
                    return Ok(());
                }
            }
        }
    }
}
