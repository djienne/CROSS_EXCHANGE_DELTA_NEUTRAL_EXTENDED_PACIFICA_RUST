use dotenv::dotenv;
use extended_connector::{init_logging, ConnectorError, OrderSide, RestClient};
use std::env;
use std::io::{self, Write};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), ConnectorError> {
    init_logging();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║        EXTENDED DEX POSITION OPEN/CHECK/CLOSE TEST              ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Load environment variables
    dotenv().ok();

    // Get credentials from environment
    let api_key = env::var("API_KEY")
        .or_else(|_| env::var("EXTENDED_API_KEY"))
        .expect("API_KEY or EXTENDED_API_KEY must be set in .env");
    let stark_private_key = env::var("STARK_PRIVATE").expect("STARK_PRIVATE must be set in .env");
    let stark_public_key = env::var("STARK_PUBLIC").expect("STARK_PUBLIC must be set in .env");
    let vault_id = env::var("VAULT_NUMBER").expect("VAULT_NUMBER must be set in .env");
    let env_type = env::var("EXTENDED_ENV").unwrap_or_else(|_| "mainnet".to_string());

    println!("Environment: {}", env_type);
    println!("Market: ETH-USD");
    println!("Notional: $25.00 per trade");
    println!("Test: LONG position then SHORT position\n");

    // Create REST client
    let client = if env_type == "testnet" {
        RestClient::new_testnet(Some(api_key))?
    } else {
        RestClient::new_mainnet(Some(api_key))?
    };

    // Get account info
    println!("--- Account Information ---");
    let account_info = client.get_account_info().await?;
    println!("Account ID: {}", account_info.account_id);
    println!("L2 Vault: {}", account_info.l2_vault);
    println!("L2 Key: {}", account_info.l2_key);
    println!("Status: {}\n", account_info.status);

    // Verify vault ID matches
    if account_info.l2_vault != vault_id {
        println!(
            "WARNING: VAULT_NUMBER in .env ({}) doesn't match account vault ({})",
            vault_id, account_info.l2_vault
        );
        println!("Using vault from account info: {}\n", account_info.l2_vault);
    }

    let actual_vault_id = &account_info.l2_vault;

    // Get current ETH-USD price
    println!("--- Current Market Data ---");
    let bid_ask = client.get_bid_ask("ETH-USD").await?;
    println!("{}\n", bid_ask);

    let mid_price = if let (Some(bid), Some(ask)) = (&bid_ask.best_bid, &bid_ask.best_ask) {
        let bid_f: f64 = bid.parse().unwrap_or(0.0);
        let ask_f: f64 = ask.parse().unwrap_or(0.0);
        (bid_f + ask_f) / 2.0
    } else {
        return Err(ConnectorError::Other("Could not get bid/ask prices".to_string()));
    };

    let notional = 25.0;
    let approx_qty = notional / mid_price;

    println!("Mid Price: ${:.2}", mid_price);
    println!("Approximate Quantity: {:.6} ETH\n", approx_qty);

    // ═══════════════════════════════════════════════════
    // STEP 0: Check initial positions
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              STEP 0: CHECK INITIAL POSITIONS                     ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let initial_positions = client.get_positions(None).await?;

    if initial_positions.is_empty() {
        println!("✅ No existing positions. Account is flat.\n");
    } else {
        println!("⚠️  WARNING: Found {} existing position(s):", initial_positions.len());
        for pos in &initial_positions {
            println!("   {}", pos);
        }
        println!("\n⚠️  Please close existing positions before running this test.\n");

        print!("Type 'continue' to proceed anyway: ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");

        if input.trim().to_lowercase() != "continue" {
            println!("Cancelled.");
            return Ok(());
        }
        println!();
    }

    // Safety confirmation
    println!("⚠️  WARNING: You are about to execute REAL trades on {}!", env_type.to_uppercase());
    println!("    This will:");
    println!("    1. Open LONG position (BUY ${})", notional);
    println!("    2. Check position is detected");
    println!("    3. Close LONG position (SELL)");
    println!("    4. Verify position is flat");
    println!("    5. Open SHORT position (SELL ${})", notional);
    println!("    6. Check position is detected");
    println!("    7. Close SHORT position (BUY)");
    println!("    8. Verify position is flat");
    println!();
    print!("Type 'yes' to continue: ");
    io::stdout().flush().expect("Failed to flush stdout");

    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read input");

    if input.trim().to_lowercase() != "yes" {
        println!("Cancelled.");
        return Ok(());
    }
    println!();

    // ═══════════════════════════════════════════════════
    // TEST 1: LONG POSITION
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                     TEST 1: LONG POSITION                        ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Step 1: Open LONG position (BUY)
    println!("--- Step 1: Opening LONG Position ---");
    println!("📈 Placing BUY market order for ${:.2} of ETH-USD...\n", notional);

    let buy_order = match client
        .place_market_order(
            "ETH-USD",
            OrderSide::Buy,
            notional,
            &stark_private_key,
            &stark_public_key,
            actual_vault_id,
        )
        .await
    {
        Ok(order) => {
            println!("✓ Buy order placed successfully!");
            println!("  Order ID: {}", order.id);
            println!("  External ID: {}\n", order.external_id);
            order
        }
        Err(e) => {
            eprintln!("❌ Error placing BUY order: {}", e);
            return Err(e);
        }
    };

    // Step 2: Wait and check position
    println!("⏳ Waiting 5 seconds for order to fill...\n");
    sleep(Duration::from_secs(5)).await;

    println!("--- Step 2: Checking LONG Position ---");
    let positions_after_buy = client.get_positions(Some("ETH-USD")).await?;
    let eth_position = positions_after_buy.iter().find(|p| p.market == "ETH-USD");

    let position_size = if let Some(pos) = eth_position {
        println!("✅ LONG position detected!");
        println!("   {}\n", pos);

        // Verify it's a LONG position
        if !pos.is_long() {
            println!("⚠️  WARNING: Expected LONG but found SHORT!\n");
        }

        pos.size_f64()
    } else {
        println!("❌ ERROR: No ETH position found after BUY order!");
        println!("   Order ID: {}", buy_order.id);
        println!("   This may indicate the order didn't fill.\n");
        return Err(ConnectorError::Other("Position not detected after BUY".to_string()));
    };

    // Step 3: Close LONG position (SELL)
    println!("--- Step 3: Closing LONG Position ---");
    println!("📉 Placing SELL market order for {:.6} ETH...\n", position_size);

    let sell_notional = position_size * mid_price;

    let sell_order = match client
        .place_market_order(
            "ETH-USD",
            OrderSide::Sell,
            sell_notional,
            &stark_private_key,
            &stark_public_key,
            actual_vault_id,
        )
        .await
    {
        Ok(order) => {
            println!("✓ Sell order placed successfully!");
            println!("  Order ID: {}", order.id);
            println!("  External ID: {}\n", order.external_id);
            order
        }
        Err(e) => {
            eprintln!("❌ Error placing SELL order: {}", e);
            eprintln!("\n⚠️  WARNING: BUY order was successful, you may have an open LONG position!");
            return Err(e);
        }
    };

    // Step 4: Verify position is flat
    println!("⏳ Waiting 5 seconds for order to fill...\n");
    sleep(Duration::from_secs(5)).await;

    println!("--- Step 4: Verifying Position is Flat ---");
    let positions_after_sell = client.get_positions(Some("ETH-USD")).await?;
    let eth_position_after = positions_after_sell.iter().find(|p| p.market == "ETH-USD");

    if eth_position_after.is_none() {
        println!("✅ SUCCESS: LONG position closed, ETH position is FLAT!\n");
    } else {
        println!("⚠️  WARNING: ETH position still exists:");
        println!("   {}", eth_position_after.unwrap());
        println!("   This may indicate partial fills or size mismatch.\n");
    }

    println!("────────────────────────────────────────────────────────────────\n");
    println!("Waiting 3 seconds before SHORT test...\n");
    sleep(Duration::from_secs(3)).await;

    // ═══════════════════════════════════════════════════
    // TEST 2: SHORT POSITION
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                     TEST 2: SHORT POSITION                       ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Step 1: Open SHORT position (SELL)
    println!("--- Step 1: Opening SHORT Position ---");
    println!("📉 Placing SELL market order for ${:.2} of ETH-USD...\n", notional);

    let short_sell_order = match client
        .place_market_order(
            "ETH-USD",
            OrderSide::Sell,
            notional,
            &stark_private_key,
            &stark_public_key,
            actual_vault_id,
        )
        .await
    {
        Ok(order) => {
            println!("✓ Sell order placed successfully!");
            println!("  Order ID: {}", order.id);
            println!("  External ID: {}\n", order.external_id);
            order
        }
        Err(e) => {
            eprintln!("❌ Error placing SELL order: {}", e);
            return Err(e);
        }
    };

    // Step 2: Wait and check position
    println!("⏳ Waiting 5 seconds for order to fill...\n");
    sleep(Duration::from_secs(5)).await;

    println!("--- Step 2: Checking SHORT Position ---");
    let positions_after_short = client.get_positions(Some("ETH-USD")).await?;
    let eth_short_position = positions_after_short.iter().find(|p| p.market == "ETH-USD");

    let short_position_size = if let Some(pos) = eth_short_position {
        println!("✅ SHORT position detected!");
        println!("   {}\n", pos);

        // Verify it's a SHORT position
        if !pos.is_short() {
            println!("⚠️  WARNING: Expected SHORT but found LONG!\n");
        }

        pos.size_f64()
    } else {
        println!("❌ ERROR: No ETH position found after SELL order!");
        println!("   Order ID: {}", short_sell_order.id);
        println!("   This may indicate the order didn't fill.\n");
        return Err(ConnectorError::Other("Position not detected after SELL".to_string()));
    };

    // Step 3: Close SHORT position (BUY)
    println!("--- Step 3: Closing SHORT Position ---");
    println!("📈 Placing BUY market order for {:.6} ETH...\n", short_position_size);

    let buy_back_notional = short_position_size * mid_price;

    let buy_back_order = match client
        .place_market_order(
            "ETH-USD",
            OrderSide::Buy,
            buy_back_notional,
            &stark_private_key,
            &stark_public_key,
            actual_vault_id,
        )
        .await
    {
        Ok(order) => {
            println!("✓ Buy order placed successfully!");
            println!("  Order ID: {}", order.id);
            println!("  External ID: {}\n", order.external_id);
            order
        }
        Err(e) => {
            eprintln!("❌ Error placing BUY order: {}", e);
            eprintln!("\n⚠️  WARNING: SELL order was successful, you may have an open SHORT position!");
            return Err(e);
        }
    };

    // Step 4: Verify position is flat
    println!("⏳ Waiting 5 seconds for order to fill...\n");
    sleep(Duration::from_secs(5)).await;

    println!("--- Step 4: Verifying Position is Flat ---");
    let final_positions = client.get_positions(Some("ETH-USD")).await?;
    let eth_final_position = final_positions.iter().find(|p| p.market == "ETH-USD");

    if eth_final_position.is_none() {
        println!("✅ SUCCESS: SHORT position closed, ETH position is FLAT!\n");
    } else {
        println!("⚠️  WARNING: ETH position still exists:");
        println!("   {}", eth_final_position.unwrap());
        println!("   This may indicate partial fills or size mismatch.\n");
    }

    // ═══════════════════════════════════════════════════
    // SUMMARY
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                        TEST SUMMARY                              ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("Test Results:");
    println!("  Market: ETH-USD");
    println!("  Notional per trade: ${:.2}", notional);
    println!();

    println!("LONG Position Test:");
    println!("  ✓ Buy Order ID: {}", buy_order.id);
    println!("  ✓ Position Detected: YES");
    println!("  ✓ Sell Order ID: {}", sell_order.id);
    println!("  ✓ Position Closed: {}", if eth_position_after.is_none() { "YES" } else { "NO" });
    println!();

    println!("SHORT Position Test:");
    println!("  ✓ Sell Order ID: {}", short_sell_order.id);
    println!("  ✓ Position Detected: YES");
    println!("  ✓ Buy Order ID: {}", buy_back_order.id);
    println!("  ✓ Position Closed: {}", if eth_final_position.is_none() { "YES" } else { "NO" });
    println!();

    let long_success = eth_position_after.is_none();
    let short_success = eth_final_position.is_none();

    if long_success && short_success {
        println!("✅ ALL TESTS PASSED!");
        println!("   Both LONG and SHORT positions were successfully opened, detected, and closed.\n");
    } else {
        println!("⚠️  SOME TESTS HAD ISSUES");
        if !long_success {
            println!("   - LONG position not fully closed");
        }
        if !short_success {
            println!("   - SHORT position not fully closed");
        }
        println!();
    }

    println!("💡 Check detailed positions:");
    println!("   cargo run --example check_positions\n");

    println!("💡 View trade history on Extended UI:");
    println!("   https://app.extended.exchange/\n");

    Ok(())
}
