use extended_connector::{init_logging, RestClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_logging();

    // Load environment variables (optional)
    dotenv::dotenv().ok();
    let api_key = std::env::var("EXTENDED_API_KEY").ok();

    // Create REST client for mainnet
    println!("Creating REST client for Extended mainnet...");
    let client = RestClient::new_mainnet(api_key)?;

    // Example 1: Get orderbook for BTC-USD
    println!("\n=== Example 1: Get BTC-USD Orderbook ===");
    match client.get_orderbook("BTC-USD").await {
        Ok(orderbook) => {
            println!("Market: {}", orderbook.market);
            println!("Top 5 Bids:");
            for (i, bid) in orderbook.bid.iter().take(5).enumerate() {
                println!("  {}: {} @ {}", i + 1, bid.quantity, bid.price);
            }
            println!("\nTop 5 Asks:");
            for (i, ask) in orderbook.ask.iter().take(5).enumerate() {
                println!("  {}: {} @ {}", i + 1, ask.quantity, ask.price);
            }
        }
        Err(e) => {
            eprintln!("Error fetching orderbook: {}", e);
        }
    }

    // Example 2: Get best bid/ask for a single market
    println!("\n=== Example 2: Get Best Bid/Ask for ETH-USD ===");
    match client.get_bid_ask("ETH-USD").await {
        Ok(bid_ask) => {
            println!("{}", bid_ask);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    // Example 3: Get bid/ask for multiple markets concurrently
    println!("\n=== Example 3: Get Multiple Markets Concurrently ===");
    let markets = vec![
        "BTC-USD".to_string(),
        "ETH-USD".to_string(),
        "SOL-USD".to_string(),
    ];

    let results = client.get_multiple_bid_asks(&markets).await;

    for result in results {
        match result {
            Ok(bid_ask) => println!("{}", bid_ask),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}
