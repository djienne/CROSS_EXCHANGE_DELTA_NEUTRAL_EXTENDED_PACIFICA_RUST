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

    println!("=== Extended DEX Market Order Test ===\n");
    println!("Environment: {}", env_type);
    println!("Market: SOL-USD");
    println!("Notional: $20.00");
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

    // Get current SOL-USD price
    println!("--- Current Market Data ---");
    let bid_ask = client.get_bid_ask("SOL-USD").await?;
    println!("{}\n", bid_ask);

    // Calculate order quantity
    let mid_price = if let (Some(bid), Some(ask)) = (&bid_ask.best_bid, &bid_ask.best_ask) {
        let bid_f: f64 = bid.parse().unwrap_or(0.0);
        let ask_f: f64 = ask.parse().unwrap_or(0.0);
        (bid_f + ask_f) / 2.0
    } else {
        return Err(ConnectorError::Other("Could not get bid/ask prices".to_string()));
    };

    let notional = 20.0;
    let approx_qty = notional / mid_price;

    println!("Mid Price: ${:.2}", mid_price);
    println!("Approximate Quantity: {:.6} SOL\n", approx_qty);

    // Safety confirmation
    println!("⚠️  WARNING: You are about to execute REAL trades on {}!", env_type.to_uppercase());
    println!("    This will:");
    println!("    1. BUY ${:.2} of SOL-USD", notional);
    println!("    2. Wait 5 seconds");
    println!("    3. SELL ${:.2} of SOL-USD", notional);
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
    let buy_order = match client
        .place_market_order(
            "SOL-USD",
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

    // Step 2: Place market SELL order to close position
    println!("--- Step 2: Placing Market SELL Order ---");
    let sell_order = match client
        .place_market_order(
            "SOL-USD",
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
            println!("  External ID: {}", order.external_id);
            println!();
            order
        }
        Err(e) => {
            eprintln!("❌ Error placing SELL order: {}", e);
            eprintln!("\nNote: BUY order was successful, but SELL failed.");
            eprintln!("You may have an open SOL position. Check your Extended account.");
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
    println!("✓ Position should be neutral (bought then sold)");
    println!("✓ Total notional traded: ${:.2}", notional * 2.0);
    println!("\nNote: Check your Extended account to verify order fills and final position.");

    Ok(())
}
