use extended_connector::{init_logging, PacificaCredentials, PacificaTrading};
use extended_connector::pacifica::OrderSide;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║        PACIFICA ORDER TEST - $20 Notional Buy/Sell Test         ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Load credentials
    let credentials = PacificaCredentials::from_env()?;
    let mut client = PacificaTrading::new(credentials);

    // Test with SOL - small size for ~$20 notional
    let symbol = "SOL";
    let target_notional = 20.0; // $20 target

    println!("📊 Fetching market information for {}...", symbol);
    let market_info = client.get_market_info().await?;

    let info = market_info.get(symbol)
        .ok_or_else(|| format!("Market {} not found", symbol))?;

    // Clone values we need before releasing the borrow
    let lot_size_str = info.lot_size.clone();
    let tick_size_str = info.tick_size.clone();
    let symbol_str = info.symbol.clone();

    println!("✅ Market info retrieved:");
    println!("   Symbol: {}", symbol_str);
    println!("   Tick Size: {}", tick_size_str);
    println!("   Lot Size: {}", lot_size_str);

    // Get current price from orderbook
    println!("\n📊 Fetching current orderbook...");
    let orderbook = client.get_orderbook_rest(symbol, 1).await?;

    let best_bid = orderbook.bids.first()
        .ok_or_else(|| "No bids in orderbook")?
        .price.parse::<f64>()?;
    let best_ask = orderbook.asks.first()
        .ok_or_else(|| "No asks in orderbook")?
        .price.parse::<f64>()?;
    let mid_price = (best_bid + best_ask) / 2.0;

    println!("✅ Current market:");
    println!("   Best bid: ${:.2}", best_bid);
    println!("   Best ask: ${:.2}", best_ask);
    println!("   Mid price: ${:.2}", mid_price);

    // Calculate size for $20 notional
    let lot_size: f64 = lot_size_str.parse()?;
    let raw_size = target_notional / mid_price;
    let size = (raw_size / lot_size).floor() * lot_size;
    let actual_notional = size * mid_price;

    println!("\n💰 Position sizing:");
    println!("   Target notional: ${:.2}", target_notional);
    println!("   Calculated size: {}", size);
    println!("   Actual notional: ${:.2}", actual_notional);

    if size < lot_size {
        return Err(format!("Size too small! Minimum lot size is {}", lot_size).into());
    }

    println!("\n⚠️  WARNING: This will execute REAL market orders!");
    println!("   Symbol: {}", symbol);
    println!("   Size: {} {}", size, symbol);
    println!("   Notional: ${:.2}", actual_notional);
    println!("   Action: BUY → wait 5s → SELL");
    println!("\n⚠️  Type 'yes' to continue or anything else to cancel: ");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() != "yes" {
        println!("\n❌ Test cancelled by user.");
        return Ok(());
    }

    println!("\n✅ Starting test...\n");

    // ═══════════════════════════════════════════════════
    // STEP 1: Check initial position
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              STEP 1: CHECK INITIAL POSITION                      ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let initial_positions = client.get_positions().await?;
    let initial_sol_pos = initial_positions.iter()
        .find(|p| p.symbol == symbol);

    if let Some(pos) = initial_sol_pos {
        println!("⚠️  WARNING: Existing {} position found:", symbol);
        println!("   Side: {}", if pos.is_long() { "LONG" } else { "SHORT" });
        println!("   Amount: {}", pos.amount);
        println!("   Entry Price: ${}", pos.entry_price);
        println!("\n   This test will modify your existing position!\n");
    } else {
        println!("✅ No existing {} position. Starting fresh.\n", symbol);
    }

    // ═══════════════════════════════════════════════════
    // STEP 2: BUY
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              STEP 2: PLACE BUY ORDER                             ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("📈 Placing BUY market order for {} {}...", size, symbol);
    let buy_order = client
        .place_market_order(symbol, OrderSide::Buy, size, 1.0, false) // 1% slippage, not reduce_only
        .await?;

    let buy_order_id = buy_order.order_id.or(buy_order.i).unwrap_or(0);
    println!("✅ Buy order placed successfully!");
    println!("   Order ID: {}", buy_order_id);
    println!("   Client Order ID: {}", buy_order.client_order_id.as_deref().unwrap_or("N/A"));

    // ═══════════════════════════════════════════════════
    // STEP 3: WAIT
    // ═══════════════════════════════════════════════════
    println!("\n⏳ Waiting 5 seconds for order to fill...");
    sleep(Duration::from_secs(5)).await;

    // Check position after buy
    println!("\n📊 Checking position after BUY...");
    let positions_after_buy = client.get_positions().await?;
    let pos_after_buy = positions_after_buy.iter()
        .find(|p| p.symbol == symbol);

    if let Some(pos) = pos_after_buy {
        println!("✅ Position confirmed:");
        println!("   Side: {}", if pos.is_long() { "LONG" } else { "SHORT" });
        println!("   Amount: {}", pos.amount);
        println!("   Entry Price: ${}", pos.entry_price);
        println!("   Position Value: ${:.2}", pos.size() * pos.entry());
    } else {
        println!("⚠️  No position found - order may not have filled yet");
    }

    // ═══════════════════════════════════════════════════
    // STEP 4: SELL
    // ═══════════════════════════════════════════════════
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║              STEP 4: PLACE SELL ORDER                            ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("📉 Placing SELL market order for {} {}...", size, symbol);
    let sell_order = client
        .place_market_order(symbol, OrderSide::Sell, size, 1.0, false) // 1% slippage, not reduce_only
        .await?;

    let sell_order_id = sell_order.order_id.or(sell_order.i).unwrap_or(0);
    println!("✅ Sell order placed successfully!");
    println!("   Order ID: {}", sell_order_id);
    println!("   Client Order ID: {}", sell_order.client_order_id.as_deref().unwrap_or("N/A"));

    // ═══════════════════════════════════════════════════
    // STEP 5: VERIFY FLAT
    // ═══════════════════════════════════════════════════
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║              STEP 5: VERIFY POSITION CLOSED                      ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("⏳ Waiting 5 seconds for orders to settle...");
    sleep(Duration::from_secs(5)).await;

    let final_positions = client.get_positions().await?;
    let final_sol_pos = final_positions.iter()
        .find(|p| p.symbol == symbol);

    match (initial_sol_pos, final_sol_pos) {
        (None, None) => {
            println!("✅ SUCCESS: Position is FLAT!");
            println!("   Started with no position, ended with no position.");
        }
        (Some(initial), None) => {
            println!("✅ Position closed!");
            println!("   Initial: {} {}", initial.amount, if initial.is_long() { "LONG" } else { "SHORT" });
            println!("   Final: No position");
        }
        (None, Some(final_pos)) => {
            println!("⚠️  WARNING: Position still open!");
            println!("   Expected: No position");
            println!("   Actual: {} {} {}", final_pos.amount, symbol, if final_pos.is_long() { "LONG" } else { "SHORT" });
        }
        (Some(initial), Some(final_pos)) => {
            if initial.amount == final_pos.amount && initial.side == final_pos.side {
                println!("✅ Position unchanged (returned to initial state)");
                println!("   Amount: {}", final_pos.amount);
            } else {
                println!("⚠️  Position changed!");
                println!("   Initial: {} {}", initial.amount, if initial.is_long() { "LONG" } else { "SHORT" });
                println!("   Final: {} {}", final_pos.amount, if final_pos.is_long() { "LONG" } else { "SHORT" });
            }
        }
    }

    // ═══════════════════════════════════════════════════
    // SUMMARY
    // ═══════════════════════════════════════════════════
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║                        TEST SUMMARY                              ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("✅ Test completed successfully!");
    println!("   Symbol: {}", symbol);
    println!("   Size: {} {}", size, symbol);
    println!("   Notional: ${:.2}", actual_notional);
    println!("   Buy Order ID: {}", buy_order_id);
    println!("   Sell Order ID: {}", sell_order_id);
    println!("\n💡 Check Pacifica UI for actual fill prices and P&L:");
    println!("   https://app.pacifica.fi/\n");

    Ok(())
}
