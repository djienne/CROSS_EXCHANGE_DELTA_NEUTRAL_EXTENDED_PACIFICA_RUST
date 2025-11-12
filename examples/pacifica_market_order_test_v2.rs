use extended_connector::{init_logging, PacificaCredentials, PacificaTrading};
use extended_connector::pacifica::OrderSide;
use std::time::Duration;
use tokio::time::sleep;

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
    println!("║   PACIFICA MARKET ORDER TEST - Buy/Wait/Sell with Position Check║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Load credentials
    let credentials = PacificaCredentials::from_env()?;
    let mut client = PacificaTrading::new(credentials);

    // Define test coins - amounts designed to exceed $20 minimum
    let test_coins = vec![
        TestCoin {
            symbol: "ETH",
            size: 0.01,      // 0.01 ETH (~$30 at $3000/ETH)
            slippage: 1.0,   // 1% slippage tolerance
            wait_seconds: 5, // Wait 5 seconds between buy and sell
        },
        TestCoin {
            symbol: "SOL",
            size: 0.15,      // 0.15 SOL (~$30 at $200/SOL)
            slippage: 1.0,
            wait_seconds: 5,
        },
        TestCoin {
            symbol: "PUMP",
            size: 300.0,     // 300 PUMP (should be >$20)
            slippage: 1.0,
            wait_seconds: 5,
        },
        TestCoin {
            symbol: "XPL",
            size: 100.0,     // 100 XPL (should be >$20)
            slippage: 1.0,
            wait_seconds: 5,
        },
    ];

    println!("⚠️  WARNING: This will execute REAL market orders on Pacifica!");
    println!("⚠️  Make sure you have sufficient balance in your account.");
    println!("\nTesting {} coins with buy/wait/sell cycles...", test_coins.len());

    // Show what will be tested
    println!("\nCoins to test:");
    for coin in &test_coins {
        println!("  - {}: {} units ({}% slippage)", coin.symbol, coin.size, coin.slippage);
    }

    println!("\n⚠️  Type 'yes' to continue or anything else to cancel: ");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() != "yes" {
        println!("\n❌ Test cancelled by user.");
        return Ok(());
    }

    println!("\n✅ Starting tests...\n");

    // ═══════════════════════════════════════════════════
    // STEP 1: Check initial positions
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              STEP 1: CHECK INITIAL POSITIONS                     ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let initial_positions = client.get_positions().await?;

    if initial_positions.is_empty() {
        println!("✅ No existing positions. Account is flat.\n");
    } else {
        println!("⚠️  WARNING: Found {} existing position(s):", initial_positions.len());
        for pos in &initial_positions {
            println!("   - {}", pos);
        }
        println!("\n⚠️  These existing positions will not be affected by the test.");
        println!("⚠️  Only new positions from this test will be verified as flat.\n");
    }

    // Track results
    let mut results = Vec::new();

    // ═══════════════════════════════════════════════════
    // STEP 2: Run test cycles
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              STEP 2: RUN TEST CYCLES                             ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    for (idx, coin) in test_coins.iter().enumerate() {
        println!("────────────────────────────────────────────────────────────────");
        println!("Test {}/{}: {} (size: {}, slippage: {}%)",
                 idx + 1, test_coins.len(), coin.symbol, coin.size, coin.slippage);
        println!("────────────────────────────────────────────────────────────────\n");

        match run_test_cycle(&mut client, coin).await {
            Ok(result) => {
                println!("✅ Test completed successfully for {}", coin.symbol);
                println!("   Buy Order ID: {}", result.buy_order_id);
                println!("   Sell Order ID: {}\n", result.sell_order_id);
                results.push(result);
            }
            Err(e) => {
                println!("❌ Test failed for {}: {}\n", coin.symbol, e);
            }
        }

        // Wait a bit between different coins
        if idx < test_coins.len() - 1 {
            println!("Waiting 3 seconds before next test...\n");
            sleep(Duration::from_secs(3)).await;
        }
    }

    // ═══════════════════════════════════════════════════
    // STEP 3: Verify positions are flat
    // ═══════════════════════════════════════════════════
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║              STEP 3: VERIFY POSITIONS ARE FLAT                   ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("⏳ Waiting 5 seconds for orders to settle...");
    sleep(Duration::from_secs(5)).await;

    let final_positions = client.get_positions().await?;

    // Check if we have any NEW positions from our test
    let test_symbols: Vec<&str> = test_coins.iter().map(|c| c.symbol).collect();
    let new_positions: Vec<_> = final_positions.iter()
        .filter(|p| test_symbols.contains(&p.symbol.as_str()))
        .collect();

    if new_positions.is_empty() {
        println!("✅ SUCCESS: All test positions are FLAT!");
        println!("   No open positions for tested symbols: {:?}\n", test_symbols);
    } else {
        println!("⚠️  WARNING: Found {} position(s) from test:", new_positions.len());
        for pos in &new_positions {
            println!("   - {}", pos);
        }
        println!("\n❌ POSITIONS ARE NOT FLAT!");
        println!("   This may indicate:");
        println!("   • Orders did not fill completely");
        println!("   • Size mismatch between buy and sell");
        println!("   • Market conditions caused partial fills\n");
    }

    // Print summary
    print_summary(&results, &initial_positions, &final_positions);

    Ok(())
}

#[derive(Debug)]
struct TestResult {
    symbol: String,
    buy_order_id: u64,
    sell_order_id: u64,
    size: f64,
}

async fn run_test_cycle(
    client: &mut PacificaTrading,
    coin: &TestCoin,
) -> Result<TestResult, Box<dyn std::error::Error>> {
    // Step 1: Place BUY market order
    println!("📈 Step 1: Placing BUY market order for {} {}...", coin.size, coin.symbol);
    let buy_order = client
        .place_market_order(coin.symbol, OrderSide::Buy, coin.size, coin.slippage)
        .await?;

    let buy_order_id = buy_order.order_id.or(buy_order.i).unwrap_or(0);
    println!("   ✓ Buy order placed successfully");
    println!("   Order ID: {}", buy_order_id);
    println!("   Client Order ID: {}", buy_order.client_order_id.as_deref().unwrap_or("N/A"));

    // Step 2: Wait
    println!("\n⏳ Step 2: Waiting {} seconds...", coin.wait_seconds);
    sleep(Duration::from_secs(coin.wait_seconds)).await;

    // Step 3: Place SELL market order (exact same size to close position)
    println!("\n📉 Step 3: Placing SELL market order for {} {}...", coin.size, coin.symbol);
    let sell_order = client
        .place_market_order(coin.symbol, OrderSide::Sell, coin.size, coin.slippage)
        .await?;

    let sell_order_id = sell_order.order_id.or(sell_order.i).unwrap_or(0);
    println!("   ✓ Sell order placed successfully");
    println!("   Order ID: {}", sell_order_id);
    println!("   Client Order ID: {}\n", sell_order.client_order_id.as_deref().unwrap_or("N/A"));

    Ok(TestResult {
        symbol: coin.symbol.to_string(),
        buy_order_id,
        sell_order_id,
        size: coin.size,
    })
}

fn print_summary(
    results: &[TestResult],
    initial_positions: &[extended_connector::PacificaPosition],
    final_positions: &[extended_connector::PacificaPosition],
) {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║                        TEST SUMMARY                              ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    if results.is_empty() {
        println!("❌ No successful tests completed.\n");
        return;
    }

    println!("✅ Successfully completed {} test(s):\n", results.len());

    for (idx, result) in results.iter().enumerate() {
        println!("  {}. {} (size: {})", idx + 1, result.symbol, result.size);
        println!("     Buy Order ID:  {}", result.buy_order_id);
        println!("     Sell Order ID: {}", result.sell_order_id);
        println!();
    }

    println!("────────────────────────────────────────────────────────────────\n");

    println!("Position Status:");
    println!("  Initial positions: {}", initial_positions.len());
    println!("  Final positions:   {}", final_positions.len());
    println!();

    println!("📊 Notes:");
    println!("   • All orders were executed as market orders");
    println!("   • Buy and sell sizes matched exactly to close positions");
    println!("   • Check Pacifica UI for actual fill prices and P&L");
    println!("   • Small losses from spread and fees are expected\n");

    println!("💡 To check detailed positions:");
    println!("   cargo run --example pacifica_check_positions\n");

    println!("💡 To view trade history:");
    println!("   Visit: https://app.pacifica.fi/\n");
}
