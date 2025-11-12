use extended_connector::{init_logging, PacificaCredentials, PacificaTrading};

/// Test configuration for each coin
struct TestCoin {
    symbol: &'static str,
    size: f64,           // Order size
    slippage: f64,       // Slippage tolerance (%)
    wait_seconds: u64,   // Time to wait between buy and sell
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║     PACIFICA MARKET ORDER DRY RUN - No Real Orders Executed     ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Load credentials (for fetching market info only)
    let credentials = PacificaCredentials::from_env()?;
    let mut client = PacificaTrading::new(credentials);

    // Define test coins - amounts designed to exceed $20 minimum
    let test_coins = vec![
        TestCoin {
            symbol: "SOL",
            size: 0.15,      // ~$30 at $200/SOL
            slippage: 1.0,
            wait_seconds: 5,
        },
        TestCoin {
            symbol: "ETH",
            size: 0.01,      // ~$30 at $3000/ETH
            slippage: 1.0,
            wait_seconds: 5,
        },
        TestCoin {
            symbol: "ASTER",
            size: 100.0,     // Should be >$20
            slippage: 1.0,
            wait_seconds: 5,
        },
    ];

    println!("📊 Fetching market information...\n");
    let market_info = client.get_market_info().await?;

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                     DRY RUN PLAN                                 ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    for (idx, coin) in test_coins.iter().enumerate() {
        println!("Test {}/{}: {}", idx + 1, test_coins.len(), coin.symbol);
        println!("────────────────────────────────────────────────────────────────");

        if let Some(info) = market_info.get(coin.symbol) {
            let symbol = info.symbol.clone();
            let tick_size = info.tick_size.clone();
            let lot_size = info.lot_size.clone();

            println!("  Symbol: {}", symbol);
            println!("  Tick Size: {}", tick_size);
            println!("  Lot Size: {}", lot_size);

            // Calculate rounded size
            let lot: f64 = lot_size.parse().unwrap_or(1.0);
            let rounded = (coin.size / lot).round() * lot;

            println!("\n  Planned Actions:");
            println!("  1. BUY {} {} (slippage: {}%)", rounded, symbol, coin.slippage);
            println!("  2. WAIT {} seconds", coin.wait_seconds);
            println!("  3. SELL {} {} (slippage: {}%)", rounded, symbol, coin.slippage);

            println!("\n  ✅ Market info validated");
        } else {
            println!("  ❌ Market not found!");
        }

        println!();
    }

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                        SUMMARY                                   ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("✅ Dry run completed successfully!");
    println!("   {} coins would be tested", test_coins.len());
    println!("   Total test cycles: {}", test_coins.len() * 2); // buy + sell per coin
    println!("\n💡 To execute real orders, run:");
    println!("   cargo run --example pacifica_market_order_test\n");

    println!("⚠️  Important Notes:");
    println!("   • Make sure you have sufficient balance");
    println!("   • Market orders execute immediately at current price");
    println!("   • Slippage protection may cause orders to fail if market moves");
    println!("   • Small test sizes minimize risk");
    println!("   • Monitor your positions after execution\n");

    Ok(())
}
