use dotenv::dotenv;
use extended_connector::{ConnectorError, OrderSide, RestClient};
use std::env;
use std::io::{self, Write};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), ConnectorError> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_line_number(true)
        .init();

    // Load environment variables
    dotenv().ok();

    // Get credentials from environment
    let api_key = env::var("API_KEY").expect("API_KEY must be set in .env");
    let stark_private_key = env::var("STARK_PRIVATE").expect("STARK_PRIVATE must be set in .env");
    let stark_public_key = env::var("STARK_PUBLIC").expect("STARK_PUBLIC must be set in .env");
    let vault_id = env::var("VAULT_NUMBER").expect("VAULT_NUMBER must be set in .env");
    let env_type = env::var("EXTENDED_ENV").unwrap_or_else(|_| "mainnet".to_string());

    println!("=== Extended DEX BTC Market Order Test ===\n");
    println!("Environment: {}", env_type);
    println!("Market: BTC-USD");
    println!("Notional: $15.00 (slightly above minimum $10.30)");
    println!("Strategy: Buy then Sell (neutral position)\n");

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

    // Get current BTC-USD price
    println!("--- Current Market Data ---");
    let bid_ask = client.get_bid_ask("BTC-USD").await?;
    println!("{}\n", bid_ask);

    // Calculate order quantity
    let mid_price = if let (Some(bid), Some(ask)) = (&bid_ask.best_bid, &bid_ask.best_ask) {
        let bid_f: f64 = bid.parse().unwrap_or(0.0);
        let ask_f: f64 = ask.parse().unwrap_or(0.0);
        (bid_f + ask_f) / 2.0
    } else {
        return Err(ConnectorError::Other("Could not get bid/ask prices".to_string()));
    };

    let notional = 15.0;
    let approx_qty = notional / mid_price;

    println!("Mid Price: ${:.2}", mid_price);
    println!("Approximate Quantity: {:.8} BTC", approx_qty);
    println!("(Minimum required: 0.0001 BTC = ${:.2})\n", mid_price * 0.0001);

    // Safety confirmation
    println!("⚠️  WARNING: You are about to execute REAL trades on {}!", env_type.to_uppercase());
    println!("    This will:");
    println!("    1. BUY ${:.2} of BTC-USD", notional);
    println!("    2. Wait 5 seconds");
    println!("    3. SELL ${:.2} of BTC-USD", notional);
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

    // Step 1: Place market BUY order
    println!("--- Step 1: Placing Market BUY Order ---");
    let _buy_order = match client
        .place_market_order(
            "BTC-USD",
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
            println!("  External ID: {}", order.external_id);
            println!();
            order
        }
        Err(e) => {
            eprintln!("❌ Error placing BUY order: {}", e);
            eprintln!("\nPossible issues:");
            eprintln!("  - Order size too small (minimum requirements not met)");
            eprintln!("  - Insufficient balance");
            eprintln!("  - Invalid credentials");
            eprintln!("  - Network issues");
            return Err(e);
        }
    };

    // Wait for order to be processed
    println!("Waiting 5 seconds for order to fill...");
    sleep(Duration::from_secs(5)).await;
    println!();

    // Step 2: Query current position to get exact size
    println!("--- Step 2: Querying Current Position ---");
    let positions = client.get_positions(Some("BTC-USD")).await?;

    let btc_position = positions.iter().find(|p| p.market == "BTC-USD");

    let sell_quantity = if let Some(pos) = btc_position {
        println!("Found BTC position:");
        println!("  Side: {:?}", pos.side);
        println!("  Size: {} BTC", pos.size);
        println!("  Value: ${}", pos.value);
        println!();
        pos.size.parse::<f64>().unwrap_or(0.0)
    } else {
        println!("No BTC position found, using notional-based quantity");
        let qty = notional / mid_price;
        println!("  Calculated quantity: {:.4} BTC\n", qty);
        qty
    };

    // Step 3: Place market SELL order to close position using exact quantity
    println!("--- Step 3: Placing Market SELL Order ---");
    println!("Selling exact position size: {:.4} BTC\n", sell_quantity);

    // Calculate notional from exact quantity for the order
    let sell_notional = sell_quantity * mid_price;

    let _sell_order = match client
        .place_market_order(
            "BTC-USD",
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
            println!("  External ID: {}", order.external_id);
            println!();
            order
        }
        Err(e) => {
            eprintln!("❌ Error placing SELL order: {}", e);
            eprintln!("\nNote: BUY order was successful, but SELL failed.");
            eprintln!("You may have an open BTC position. Check your Extended account.");
            eprintln!("\nPossible issues:");
            eprintln!("  - Order size too small (minimum requirements not met)");
            eprintln!("  - Insufficient position to sell");
            eprintln!("  - Invalid credentials");
            eprintln!("  - Network issues");
            return Err(e);
        }
    };

    println!("--- Test Summary ---");
    println!("✓ Both orders executed successfully");
    println!("✓ Position closed using exact position size (no dust)");
    println!("✓ BUY notional: ${:.2}", notional);
    println!("✓ SELL notional: ${:.2}", sell_notional);
    println!("\nNote: Check your Extended account to verify order fills and confirm zero BTC position.");

    Ok(())
}
