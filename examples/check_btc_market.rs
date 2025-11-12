use dotenv::dotenv;
use extended_connector::RestClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let api_key = env::var("API_KEY").ok();
    let env_type = env::var("EXTENDED_ENV").unwrap_or_else(|_| "mainnet".to_string());

    println!("=== BTC-USD Market Information ===\n");

    let client = if env_type == "testnet" {
        RestClient::new_testnet(api_key)?
    } else {
        RestClient::new_mainnet(api_key)?
    };

    // Get BTC market config
    println!("Fetching BTC-USD market configuration...");
    let config = client.get_market_config("BTC-USD").await?;

    println!("\n--- Trading Configuration ---");
    println!("Minimum Order Size: {} BTC", config.trading_config.min_order_size);
    println!("Min Size Change (increment): {} BTC", config.trading_config.min_order_size_change);

    // Get current price
    println!("\n--- Current Market Data ---");
    let bid_ask = client.get_bid_ask("BTC-USD").await?;
    println!("{}", bid_ask);

    // Calculate minimum order value in USD
    if let Some(ask) = &bid_ask.best_ask {
        let ask_price: f64 = ask.parse().unwrap_or(0.0);
        let min_size: f64 = config.trading_config.min_order_size.parse().unwrap_or(0.0);
        let min_notional = ask_price * min_size;

        println!("\n--- Minimum Order Calculation ---");
        println!("Min BTC amount: {} BTC", min_size);
        println!("Current ask price: ${:.2}", ask_price);
        println!("Minimum order value: ${:.2}", min_notional);
        println!("\nTo place a BTC order, you need at least ${:.2} notional value", min_notional);
    }

    Ok(())
}
