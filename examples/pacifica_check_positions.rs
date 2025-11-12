use extended_connector::{init_logging, PacificaCredentials, PacificaTrading};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              PACIFICA POSITIONS CHECKER                          ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Load credentials
    let credentials = PacificaCredentials::from_env()?;
    let client = PacificaTrading::new(credentials);

    println!("📊 Fetching current positions...\n");

    // Get all positions
    let positions = client.get_positions().await?;

    if positions.is_empty() {
        println!("✅ No open positions found.");
        println!("   Account is flat (all positions closed).\n");
    } else {
        println!("⚠️  Found {} open position(s):\n", positions.len());

        for (idx, pos) in positions.iter().enumerate() {
            println!("Position #{}:", idx + 1);
            println!("  Symbol: {}", pos.symbol);
            println!("  Side: {} {}",
                    if pos.is_long() { "LONG 📈" } else { "SHORT 📉" },
                    if pos.is_long() { "(bid)" } else { "(ask)" });
            println!("  Amount: {}", pos.amount);
            println!("  Entry Price: ${}", pos.entry_price);
            println!("  Funding Paid: ${}", pos.funding);
            println!("  Margin Mode: {}",
                    if pos.isolated { "Isolated" } else { "Cross" });
            println!("  Position Size (USD): ${:.2}",
                    pos.size() * pos.entry());
            println!();
        }

        // Summary
        println!("────────────────────────────────────────────────────────────────");
        println!("Summary:");
        let total_value: f64 = positions.iter()
            .map(|p| p.size() * p.entry())
            .sum();
        println!("  Total Position Value: ${:.2}", total_value);

        let long_count = positions.iter().filter(|p| p.is_long()).count();
        let short_count = positions.len() - long_count;
        println!("  Long Positions: {}", long_count);
        println!("  Short Positions: {}", short_count);
        println!();
    }

    Ok(())
}
