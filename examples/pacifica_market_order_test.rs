use extended_connector::{init_logging, PacificaCredentials, PacificaTrading, OrderSide};
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
    println!("║        PACIFICA MARKET ORDER TEST - Buy/Wait/Sell Cycle         ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Load credentials
    let credentials = PacificaCredentials::from_env()?;
    let mut client = PacificaTrading::new(credentials);

    // Define test coins - amounts designed to exceed $20 minimum
    let test_coins = vec![
        TestCoin {
            symbol: "SOL",
            size: 0.15,      // 0.15 SOL (~$30 at $200/SOL)
            slippage: 1.0,   // 1% slippage tolerance
            wait_seconds: 5, // Wait 5 seconds between buy and sell
        },
        TestCoin {
            symbol: "ETH",
            size: 0.01,      // 0.01 ETH (~$30 at $3000/ETH)
            slippage: 1.0,
            wait_seconds: 5,
        },
        TestCoin {
            symbol: "ASTER",
            size: 100.0,     // 100 ASTER (should be >$20)
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

    // Track results
    let mut results = Vec::new();

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

    // Print summary
    print_summary(&results);

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

    // Step 3: Place SELL market order
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

fn print_summary(results: &[TestResult]) {
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
    println!("📊 Notes:");
    println!("   • All orders were executed as market orders");
    println!("   • Check your Pacifica account for actual fill prices and P&L");
    println!("   • Small price differences between buy/sell are expected");
    println!("   • Use the trade history endpoint to see actual executed prices\n");

    println!("💡 To check trade history:");
    println!("   Visit: https://app.pacifica.fi/");
    println!("   Or use: client.get_trade_history(...)\n");
}
