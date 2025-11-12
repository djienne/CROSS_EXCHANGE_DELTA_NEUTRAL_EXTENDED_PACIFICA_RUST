use extended_connector::{init_logging, MultiMarketSubscriber, WebSocketClient};
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_logging();

    // Load environment variables (optional)
    dotenv::dotenv().ok();
    let api_key = std::env::var("EXTENDED_API_KEY").ok();

    // Example 1: Stream best bid/ask for a single market
    println!("=== Example 1: Stream BTC-USD Best Bid/Ask ===\n");
    stream_single_market(api_key.clone()).await?;

    // Example 2: Stream multiple markets
    println!("\n=== Example 2: Stream Multiple Markets ===\n");
    stream_multiple_markets(api_key).await?;

    Ok(())
}

async fn stream_single_market(api_key: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let client = WebSocketClient::new_mainnet(api_key);

    println!("Connecting to BTC-USD orderbook stream...");
    let mut rx = client.subscribe_orderbook("BTC-USD").await?;

    println!("Connected! Streaming for 30 seconds...\n");

    // Stream for 30 seconds
    let stream_duration = Duration::from_secs(30);
    let deadline = tokio::time::Instant::now() + stream_duration;

    let mut count = 0;
    while tokio::time::Instant::now() < deadline {
        match timeout(Duration::from_secs(5), rx.recv()).await {
            Ok(Some(bid_ask)) => {
                count += 1;
                println!("[{}] {}", count, bid_ask);

                // Show spread
                if let (Some(bid), Some(ask)) = (&bid_ask.best_bid, &bid_ask.best_ask) {
                    if let (Ok(bid_price), Ok(ask_price)) =
                        (bid.parse::<f64>(), ask.parse::<f64>())
                    {
                        let spread = ask_price - bid_price;
                        let spread_bps = (spread / bid_price) * 10000.0;
                        println!("    Spread: ${:.2} ({:.2} bps)", spread, spread_bps);
                    }
                }
            }
            Ok(None) => {
                println!("Channel closed");
                break;
            }
            Err(_) => {
                println!("Timeout waiting for message");
            }
        }
    }

    println!("\nReceived {} updates", count);
    Ok(())
}

async fn stream_multiple_markets(
    api_key: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = WebSocketClient::new_mainnet(api_key);
    let mut subscriber = MultiMarketSubscriber::new(client);

    let markets = vec![
        "BTC-USD".to_string(),
        "ETH-USD".to_string(),
        "SOL-USD".to_string(),
    ];

    println!("Subscribing to markets: {:?}", markets);
    let mut rx = subscriber.subscribe_markets(markets).await?;

    println!("Connected! Streaming for 30 seconds...\n");

    // Stream for 30 seconds
    let stream_duration = Duration::from_secs(30);
    let deadline = tokio::time::Instant::now() + stream_duration;

    let mut counts = std::collections::HashMap::new();

    while tokio::time::Instant::now() < deadline {
        match timeout(Duration::from_secs(5), rx.recv()).await {
            Ok(Some(bid_ask)) => {
                *counts.entry(bid_ask.market.clone()).or_insert(0) += 1;
                println!("{}", bid_ask);
            }
            Ok(None) => {
                println!("Channel closed");
                break;
            }
            Err(_) => {
                println!("Timeout waiting for message");
            }
        }
    }

    println!("\n=== Update Counts ===");
    for (market, count) in counts {
        println!("{}: {} updates", market, count);
    }

    Ok(())
}
