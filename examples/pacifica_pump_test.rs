use extended_connector::{init_logging, PacificaCredentials, PacificaTrading};
use extended_connector::pacifica::OrderSide;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              PACIFICA PUMP TEST - $20 Notional                   ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Load credentials
    let credentials = PacificaCredentials::from_env()?;
    let mut client = PacificaTrading::new(credentials);

    // From previous error: 300 PUMP = $1.18
    // Price per PUMP ≈ $0.00393
    // For $20: 20 / 0.00393 ≈ 5,089 PUMP
    // Let's use 5,100 PUMP to ensure >$20
    let pump_size = 5100.0;
    let slippage = 1.0;

    println!("Test Parameters:");
    println!("  Symbol: PUMP");
    println!("  Size: {} PUMP", pump_size);
    println!("  Estimated Value: ~$20");
    println!("  Slippage: {}%", slippage);
    println!();

    println!("⚠️  WARNING: This will execute REAL market orders!");
    println!("⚠️  Type 'yes' to continue: ");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() != "yes" {
        println!("\n❌ Test cancelled.");
        return Ok(());
    }

    println!("\n✅ Starting PUMP test...\n");

    // ═══════════════════════════════════════════════════
    // STEP 1: Check initial positions
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              STEP 1: CHECK INITIAL POSITIONS                     ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let initial_positions = client.get_positions().await?;
    let pump_position_before = initial_positions.iter()
        .find(|p| p.symbol == "PUMP");

    if let Some(pos) = pump_position_before {
        println!("⚠️  WARNING: Existing PUMP position found:");
        println!("   {}\n", pos);
    } else {
        println!("✅ No existing PUMP position.\n");
    }

    // ═══════════════════════════════════════════════════
    // STEP 2: Buy PUMP
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              STEP 2: BUY PUMP                                    ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("📈 Placing BUY market order for {} PUMP...", pump_size);
    let buy_order = client
        .place_market_order("PUMP", OrderSide::Buy, pump_size, slippage)
        .await?;

    let buy_order_id = buy_order.order_id.or(buy_order.i).unwrap_or(0);
    println!("   ✓ Buy order placed successfully");
    println!("   Order ID: {}", buy_order_id);
    println!("   Client Order ID: {}\n", buy_order.client_order_id.as_deref().unwrap_or("N/A"));

    // ═══════════════════════════════════════════════════
    // STEP 3: Wait
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              STEP 3: WAIT                                        ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("⏳ Waiting 5 seconds...\n");
    sleep(Duration::from_secs(5)).await;

    // ═══════════════════════════════════════════════════
    // STEP 4: Sell PUMP
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              STEP 4: SELL PUMP                                   ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("📉 Placing SELL market order for {} PUMP...", pump_size);
    let sell_order = client
        .place_market_order("PUMP", OrderSide::Sell, pump_size, slippage)
        .await?;

    let sell_order_id = sell_order.order_id.or(sell_order.i).unwrap_or(0);
    println!("   ✓ Sell order placed successfully");
    println!("   Order ID: {}", sell_order_id);
    println!("   Client Order ID: {}\n", sell_order.client_order_id.as_deref().unwrap_or("N/A"));

    // ═══════════════════════════════════════════════════
    // STEP 5: Verify position is flat
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              STEP 5: VERIFY POSITION IS FLAT                     ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("⏳ Waiting 5 seconds for orders to settle...");
    sleep(Duration::from_secs(5)).await;

    let final_positions = client.get_positions().await?;
    let pump_position_after = final_positions.iter()
        .find(|p| p.symbol == "PUMP");

    if let Some(pos) = pump_position_after {
        println!("⚠️  WARNING: PUMP position still exists:");
        println!("   {}", pos);
        println!("\n❌ POSITION IS NOT FLAT!");
        println!("   Size: {}", pos.amount);
        println!("   This may indicate partial fills or size mismatch.\n");
    } else {
        println!("✅ SUCCESS: PUMP position is FLAT!");
        println!("   No open PUMP position detected.\n");
    }

    // ═══════════════════════════════════════════════════
    // SUMMARY
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                        SUMMARY                                   ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("Test Results:");
    println!("  Symbol: PUMP");
    println!("  Size: {} PUMP", pump_size);
    println!("  Buy Order ID: {}", buy_order_id);
    println!("  Sell Order ID: {}", sell_order_id);
    println!();

    if pump_position_after.is_none() {
        println!("✅ Test PASSED: Position is flat");
    } else {
        println!("❌ Test FAILED: Position remains open");
    }

    println!("\n💡 Check detailed positions:");
    println!("   cargo run --example pacifica_check_positions\n");

    println!("💡 View trade history:");
    println!("   https://app.pacifica.fi/\n");

    Ok(())
}
