use extended_connector::{init_logging, PacificaTrading, PacificaCredentials, RestClient};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    // Get symbols from command line args or use defaults
    let args: Vec<String> = env::args().collect();
    let symbols = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string(), "PUMP".to_string()]
    };

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              BID-ASK SPREAD ANALYZER - BOTH EXCHANGES            ║");
    println!("║          Finding tight spreads (≤ 0.15%) for opportunities       ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("Analyzing spreads for: {}\n", symbols.join(", "));

    // Load API keys
    dotenv::dotenv().ok();
    let extended_api_key = std::env::var("EXTENDED_API_KEY")
        .or_else(|_| std::env::var("API_KEY"))
        .ok();

    // Initialize clients
    let extended_client = if let Some(ref api_key) = extended_api_key {
        Some(RestClient::new_mainnet(Some(api_key.clone()))?)
    } else {
        None
    };

    let mut pacifica_client = match PacificaCredentials::from_env() {
        Ok(credentials) => Some(PacificaTrading::new(credentials)),
        Err(_) => None,
    };

    // Header
    println!("╔══════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║ Symbol  │ Exchange  │    Bid     │    Ask     │  Mid Price │  Spread %  │ Status  ║");
    println!("╠══════════════════════════════════════════════════════════════════════════════════════╣");

    for symbol in &symbols {
        let extended_market = format!("{}-USD", symbol);

        // ═══════════════════════════════════════════════════
        // EXTENDED DEX
        // ═══════════════════════════════════════════════════
        if let Some(ref client) = extended_client {
            match client.get_orderbook(&extended_market).await {
                Ok(orderbook) => {
                    if let (Some(best_bid), Some(best_ask)) = (orderbook.bid.first(), orderbook.ask.first()) {
                        let bid = best_bid.price.parse::<f64>().unwrap_or(0.0);
                        let ask = best_ask.price.parse::<f64>().unwrap_or(0.0);
                        let mid = (bid + ask) / 2.0;
                        let spread_pct = ((ask - bid) / mid) * 100.0;

                        let status = if spread_pct <= 0.15 {
                            "✅ GOOD"
                        } else {
                            "⚠️  WIDE"
                        };

                        println!("║ {:6}  │ Extended  │ ${:9.2} │ ${:9.2} │ ${:9.2} │   {:6.3}%  │ {:7} ║",
                                 symbol, bid, ask, mid, spread_pct, status);
                    } else {
                        println!("║ {:6}  │ Extended  │     -      │     -      │     -      │     -      │   N/A   ║", symbol);
                    }
                }
                Err(e) => {
                    println!("║ {:6}  │ Extended  │ Error: {:52} ║", symbol, format!("{}", e));
                }
            }
        }

        // ═══════════════════════════════════════════════════
        // PACIFICA
        // ═══════════════════════════════════════════════════
        if let Some(ref mut client) = pacifica_client {
            match client.get_orderbook_rest(symbol, 1).await {
                Ok(orderbook) => {
                    if let (Some(best_bid), Some(best_ask)) = (orderbook.bids.first(), orderbook.asks.first()) {
                        let bid = best_bid.price.parse::<f64>().unwrap_or(0.0);
                        let ask = best_ask.price.parse::<f64>().unwrap_or(0.0);
                        let mid = (bid + ask) / 2.0;
                        let spread_pct = ((ask - bid) / mid) * 100.0;

                        let status = if spread_pct <= 0.15 {
                            "✅ GOOD"
                        } else {
                            "⚠️  WIDE"
                        };

                        println!("║ {:6}  │ Pacifica  │ ${:9.2} │ ${:9.2} │ ${:9.2} │   {:6.3}%  │ {:7} ║",
                                 symbol, bid, ask, mid, spread_pct, status);
                    } else {
                        println!("║ {:6}  │ Pacifica  │     -      │     -      │     -      │     -      │   N/A   ║", symbol);
                    }
                }
                Err(e) => {
                    println!("║ {:6}  │ Pacifica  │ Error: {:52} ║", symbol, format!("{}", e));
                }
            }
        }

        // Separator between symbols
        if symbol != symbols.last().unwrap() {
            println!("╟──────────────────────────────────────────────────────────────────────────────────────╢");
        }
    }

    println!("╚══════════════════════════════════════════════════════════════════════════════════════╝\n");

    // Summary
    println!("📊 SPREAD ANALYSIS SUMMARY\n");
    println!("✅ GOOD spread: ≤ 0.15% (suitable for arbitrage/trading)");
    println!("⚠️  WIDE spread: > 0.15% (may reduce profitability)\n");

    println!("💡 Tips:");
    println!("   • Tighter spreads = lower transaction costs");
    println!("   • Compare spreads between exchanges for best execution");
    println!("   • Spreads can vary by time of day and market volatility\n");

    println!("🔧 Usage:");
    println!("   Default symbols: cargo run --example check_spreads");
    println!("   Custom symbols:  cargo run --example check_spreads BTC ETH SOL\n");

    Ok(())
}
