use extended_connector::{init_logging, PacificaCredentials, PacificaWsTrading, RestClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║            ACCOUNT BALANCE & CAPITAL CHECKER                     ║");
    println!("║         Extended DEX & Pacifica - Available Trading Capital      ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Load API keys from environment
    dotenv::dotenv().ok();
    let extended_api_key = std::env::var("EXTENDED_API_KEY")
        .or_else(|_| std::env::var("API_KEY"))
        .ok();

    if extended_api_key.is_none() {
        println!("⚠️  No API key found for Extended DEX.");
        println!("   Set EXTENDED_API_KEY or API_KEY in .env file.\n");
    }

    // ═══════════════════════════════════════════════════
    // EXTENDED DEX
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                       EXTENDED DEX                               ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let mut extended_available = 0.0;

    if let Some(ref api_key) = extended_api_key {
        let client = RestClient::new_mainnet(Some(api_key.clone()))?;
        println!("📊 Fetching Extended DEX account balance...\n");

        match client.get_balance().await {
        Ok(balance) => {
            println!("✅ Extended DEX Balance Retrieved:");
            println!("────────────────────────────────────────────────────────────────");
            println!("  Collateral:              {}", balance.collateral_name);
            println!("  Total Balance:           ${}", balance.balance);
            println!("  Equity:                  ${}", balance.equity);
            println!("  💰 Available for Trade:  ${} ← AVAILABLE CAPITAL", balance.available_for_trade);
            println!("  Available for Withdraw:  ${}", balance.available_for_withdrawal);
            println!("  Unrealized PnL:          ${}", balance.unrealised_pnl);
            println!("  Initial Margin:          ${}", balance.initial_margin);
            println!("  Margin Ratio:            {}%", balance.margin_ratio);
            println!("────────────────────────────────────────────────────────────────\n");

            extended_available = balance.available_for_trade_f64();
        }
        Err(e) => {
            println!("❌ Failed to fetch Extended balance: {}", e);
            println!("\nPossible reasons:");
            println!("  - Invalid or expired API key");
            println!("  - Network connectivity issues");
            println!("  - API rate limiting\n");
        }
    }
    } else {
        println!("⏭️  Skipping Extended DEX (no API key)\n");
    }

    // ═══════════════════════════════════════════════════
    // PACIFICA
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                         PACIFICA                                 ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let mut pacifica_available = 0.0;

    match PacificaCredentials::from_env() {
        Ok(credentials) => {
            println!("📊 Fetching Pacifica account balance via WebSocket...\n");

            let ws_client = PacificaWsTrading::new(credentials, false);

            match ws_client.get_account_info().await {
                Ok(account_info) => {
                    println!("✅ Pacifica Balance Retrieved:");
                    println!("────────────────────────────────────────────────────────────────");
                    println!("  Account Equity:          ${}", account_info.account_equity);
                    println!("  💰 Available to Spend:   ${} ← AVAILABLE CAPITAL", account_info.available_to_spend);
                    println!("  Available to Withdraw:   ${}", account_info.available_to_withdraw);
                    println!("  Balance:                 ${}", account_info.balance);
                    println!("  Margin Used:             ${}", account_info.margin_used);
                    println!("  Maintenance Margin:      ${}", account_info.maintenance_margin);
                    println!("  Fee Tier:                {}", account_info.fee_tier);
                    println!("  Open Orders:             {}", account_info.orders_count);
                    println!("  Open Positions:          {}", account_info.positions_count);
                    println!("────────────────────────────────────────────────────────────────\n");

                    pacifica_available = account_info.available_to_spend_f64();
                }
                Err(e) => {
                    println!("❌ Failed to fetch Pacifica balance: {}", e);
                    println!("\nPossible reasons:");
                    println!("  - Invalid credentials");
                    println!("  - WebSocket connection issues");
                    println!("  - Network connectivity issues\n");
                }
            }
        }
        Err(e) => {
            println!("⚠️  Pacifica credentials not configured: {}", e);
            println!("   Set SOL_WALLET, API_PUBLIC, API_PRIVATE in .env file.\n");
        }
    }

    // ═══════════════════════════════════════════════════
    // COMPARISON & LIMITING FACTOR
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              AVAILABLE CAPITAL COMPARISON                        ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("📊 Available Trading Capital:");
    println!("────────────────────────────────────────────────────────────────");
    println!("  Extended DEX:  ${:.2}", extended_available);
    println!("  Pacifica:      ${:.2}", pacifica_available);
    println!("────────────────────────────────────────────────────────────────\n");

    if extended_available > 0.0 && pacifica_available > 0.0 {
        let limiting_capital = extended_available.min(pacifica_available);
        let limiting_exchange = if extended_available < pacifica_available {
            "Extended DEX"
        } else {
            "Pacifica"
        };

        println!("💡 LIMITING FACTOR:");
        println!("   For multi-exchange trading strategies, your maximum position");
        println!("   size is limited by the exchange with the LOWEST available capital.\n");

        println!("🔍 Current Status:");
        println!("   - Extended DEX: ${:.2} available {}",
                 extended_available,
                 if extended_available == limiting_capital { "⚠️ " } else { "✅" });
        println!("   - Pacifica: ${:.2} available {}",
                 pacifica_available,
                 if pacifica_available == limiting_capital { "⚠️ " } else { "✅" });
        println!();
        println!("   🎯 Limiting Exchange: {} (${:.2})", limiting_exchange, limiting_capital);
        println!("   📏 Max Position Size: ${:.2} on each exchange\n", limiting_capital);

        // Calculate percentage difference
        let difference = (extended_available - pacifica_available).abs();
        let percent_diff = (difference / extended_available.max(pacifica_available)) * 100.0;

        if percent_diff > 10.0 {
            println!("⚠️  WARNING: Significant capital imbalance detected!");
            println!("   Difference: ${:.2} ({:.1}%)", difference, percent_diff);
            println!("   Consider rebalancing before large trades.\n");
        } else {
            println!("✅ Capital is relatively balanced between exchanges.\n");
        }
    } else if extended_available > 0.0 || pacifica_available > 0.0 {
        println!("⚠️  Only one exchange has available capital.");
        println!("   Cannot perform hedged/neutral strategies without both.\n");
    } else {
        println!("❌ No available capital detected on either exchange.\n");
    }

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                        USAGE NOTES                               ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("📝 Available Capital Explained:");
    println!("   • Available for Trade: Capital that can be used for new positions");
    println!("   • This accounts for existing positions and margin requirements");
    println!("   • Always ensure sufficient capital before placing orders\n");

    println!("⚠️  Important:");
    println!("   • Extended DEX: Uses REST API (/user/balance)");
    println!("   • Pacifica: Uses WebSocket (account_info channel)");
    println!("   • For production trading, monitor both in real-time\n");

    println!("🔧 Next Steps:");
    println!("   1. Check positions: cargo run --example check_positions");
    println!("   2. Check Pacifica positions: cargo run --example pacifica_check_positions\n");

    Ok(())
}
