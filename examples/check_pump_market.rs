use dotenv::dotenv;
use extended_connector::RestClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let api_key = env::var("API_KEY").ok();
    let env_type = env::var("EXTENDED_ENV").unwrap_or_else(|_| "mainnet".to_string());

    println!("=== PUMP-USD Market Information ===\n");

    let client = if env_type == "testnet" {
        RestClient::new_testnet(api_key)?
    } else {
        RestClient::new_mainnet(api_key)?
    };

    // Get PUMP market config
    println!("Fetching PUMP-USD market configuration...");
    let config = client.get_market_config("PUMP-USD").await?;

    println!("\n--- Trading Configuration ---");
    println!("Minimum Order Size: {} PUMP", config.trading_config.min_order_size);
    println!("Min Size Change (increment): {} PUMP", config.trading_config.min_order_size_change);
    println!("Min Price Change: {} (precision)", config.trading_config.min_price_change);

    // Calculate price precision
    let price_precision = config.trading_config.get_price_precision();
    println!("Price Precision: {} decimal places", price_precision);

    // Get current price
    println!("\n--- Current Market Data ---");
    let bid_ask = client.get_bid_ask("PUMP-USD").await?;
    println!("{}", bid_ask);

    // Calculate minimum order value in USD
    if let Some(ask) = &bid_ask.best_ask {
        let ask_price: f64 = ask.parse().unwrap_or(0.0);
        let min_size: f64 = config.trading_config.min_order_size.parse().unwrap_or(0.0);
        let min_notional = ask_price * min_size;

        println!("\n--- Minimum Order Calculation ---");
        println!("Min PUMP amount: {} PUMP", min_size);
        println!("Current ask price: ${:.6}", ask_price);
        println!("Minimum order value: ${:.2}", min_notional);

        if min_notional <= 20.0 {
            println!("\n✓ $20 order is SUFFICIENT for PUMP-USD (min: ${:.2})", min_notional);
        } else {
            println!("\n✗ $20 order is TOO SMALL for PUMP-USD (min: ${:.2})", min_notional);
        }
    }

    Ok(())
}
