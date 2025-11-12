use extended_connector::{init_logging, RestClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              EXTENDED DEX POSITIONS CHECKER                      ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Load API key from environment
    dotenv::dotenv().ok();
    let api_key = std::env::var("EXTENDED_API_KEY")
        .or_else(|_| std::env::var("API_KEY"))
        .ok();

    if api_key.is_none() {
        println!("⚠️  No API key found. Set EXTENDED_API_KEY in .env file.");
        println!("   Some endpoints may not work without authentication.\n");
    }

    let client = RestClient::new_mainnet(api_key)?;

    println!("📊 Fetching current positions...\n");

    // Get all positions
    let positions = client.get_positions(None).await?;

    if positions.is_empty() {
        println!("✅ No open positions found.");
        println!("   Account is flat (all positions closed).\n");
    } else {
        println!("⚠️  Found {} open position(s):\n", positions.len());

        for (idx, pos) in positions.iter().enumerate() {
            println!("Position #{}:", idx + 1);
            println!("  Market: {}", pos.market);
            println!("  Side: {} {}",
                    if pos.is_long() { "LONG 📈" } else { "SHORT 📉" },
                    if pos.is_long() { "(buy)" } else { "(sell)" });
            println!("  Size: {}", pos.size);
            println!("  Value: ${}", pos.value);

            if let Some(entry) = &pos.entry_price {
                println!("  Entry Price: ${}", entry);
            }

            if let Some(pnl) = &pos.unrealized_pnl {
                let pnl_f64 = pos.pnl_f64();
                let color = if pnl_f64 >= 0.0 { "✅" } else { "❌" };
                println!("  Unrealized PnL: ${} {}", pnl, color);
            }
            println!();
        }

        // Summary
        println!("────────────────────────────────────────────────────────────────");
        println!("Summary:");

        let total_value: f64 = positions.iter()
            .map(|p| p.value_f64())
            .sum();
        println!("  Total Position Value: ${:.2}", total_value);

        let total_pnl: f64 = positions.iter()
            .map(|p| p.pnl_f64())
            .sum();
        let pnl_indicator = if total_pnl >= 0.0 { "✅" } else { "❌" };
        println!("  Total Unrealized PnL: ${:.2} {}", total_pnl, pnl_indicator);

        let long_count = positions.iter().filter(|p| p.is_long()).count();
        let short_count = positions.len() - long_count;
        println!("  Long Positions: {}", long_count);
        println!("  Short Positions: {}", short_count);
        println!();
    }

    Ok(())
}
