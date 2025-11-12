use extended_connector::{init_logging, PacificaTrading, PacificaCredentials, RestClient};
use std::env;

#[derive(Debug)]
struct CrossSpreadData {
    symbol: String,
    extended_bid: f64,
    extended_ask: f64,
    extended_mid: f64,
    pacifica_bid: f64,
    pacifica_ask: f64,
    pacifica_mid: f64,
    price_diff: f64,      // Pacifica_mid - Extended_mid
    spread_pct: f64,      // (price_diff / Extended_mid) * 100
}

impl CrossSpreadData {
    fn cheaper_exchange(&self) -> &str {
        if self.extended_mid < self.pacifica_mid {
            "Extended"
        } else {
            "Pacifica"
        }
    }

    fn more_expensive_exchange(&self) -> &str {
        if self.extended_mid > self.pacifica_mid {
            "Extended"
        } else {
            "Pacifica"
        }
    }

    fn arbitrage_direction(&self) -> String {
        if self.spread_pct.abs() < 0.05 {
            "No opportunity (< 0.05%)".to_string()
        } else if self.spread_pct > 0.0 {
            format!("Buy Extended → Sell Pacifica (+{:.3}%)", self.spread_pct)
        } else {
            format!("Buy Pacifica → Sell Extended ({:.3}%)", self.spread_pct)
        }
    }

    fn opportunity_status(&self) -> &str {
        let abs_spread = self.spread_pct.abs();
        if abs_spread >= 0.20 {
            "🚀 EXCELLENT"
        } else if abs_spread >= 0.10 {
            "✅ GOOD"
        } else if abs_spread >= 0.05 {
            "⚠️  SMALL"
        } else {
            "❌ NONE"
        }
    }
}

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
    println!("║           CROSS-EXCHANGE SPREAD ANALYZER                         ║");
    println!("║     Finding arbitrage opportunities between Extended & Pacifica  ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("Analyzing cross-exchange spreads for: {}\n", symbols.join(", "));

    // Load API keys
    dotenv::dotenv().ok();
    let extended_api_key = std::env::var("EXTENDED_API_KEY")
        .or_else(|_| std::env::var("API_KEY"))
        .ok();

    // Initialize clients
    let extended_client = if let Some(ref api_key) = extended_api_key {
        Some(RestClient::new_mainnet(Some(api_key.clone()))?)
    } else {
        println!("⚠️  Extended API key not found - Extended data will be unavailable\n");
        None
    };

    let mut pacifica_client = match PacificaCredentials::from_env() {
        Ok(credentials) => Some(PacificaTrading::new(credentials)),
        Err(_) => {
            println!("⚠️  Pacifica credentials not found - Pacifica data will be unavailable\n");
            None
        }
    };

    if extended_client.is_none() || pacifica_client.is_none() {
        println!("❌ Both exchanges required for cross-spread analysis\n");
        return Ok(());
    }

    let mut spread_data_list: Vec<CrossSpreadData> = Vec::new();

    // Collect data for all symbols
    for symbol in &symbols {
        let extended_market = format!("{}-USD", symbol);

        // Fetch Extended orderbook
        let (extended_bid, extended_ask, extended_mid) = if let Some(ref client) = extended_client {
            match client.get_orderbook(&extended_market).await {
                Ok(orderbook) => {
                    if let (Some(best_bid), Some(best_ask)) = (orderbook.bid.first(), orderbook.ask.first()) {
                        let bid = best_bid.price.parse::<f64>().unwrap_or(0.0);
                        let ask = best_ask.price.parse::<f64>().unwrap_or(0.0);
                        let mid = (bid + ask) / 2.0;
                        (bid, ask, mid)
                    } else {
                        (0.0, 0.0, 0.0)
                    }
                }
                Err(e) => {
                    println!("⚠️  Failed to fetch {} from Extended: {}", symbol, e);
                    (0.0, 0.0, 0.0)
                }
            }
        } else {
            (0.0, 0.0, 0.0)
        };

        // Fetch Pacifica orderbook
        let (pacifica_bid, pacifica_ask, pacifica_mid) = if let Some(ref mut client) = pacifica_client {
            match client.get_orderbook_rest(symbol, 1).await {
                Ok(orderbook) => {
                    if let (Some(best_bid), Some(best_ask)) = (orderbook.bids.first(), orderbook.asks.first()) {
                        let bid = best_bid.price.parse::<f64>().unwrap_or(0.0);
                        let ask = best_ask.price.parse::<f64>().unwrap_or(0.0);
                        let mid = (bid + ask) / 2.0;
                        (bid, ask, mid)
                    } else {
                        (0.0, 0.0, 0.0)
                    }
                }
                Err(e) => {
                    println!("⚠️  Failed to fetch {} from Pacifica: {}", symbol, e);
                    (0.0, 0.0, 0.0)
                }
            }
        } else {
            (0.0, 0.0, 0.0)
        };

        // Calculate cross-exchange spread
        if extended_mid > 0.0 && pacifica_mid > 0.0 {
            let price_diff = pacifica_mid - extended_mid;
            let spread_pct = (price_diff / extended_mid) * 100.0;

            spread_data_list.push(CrossSpreadData {
                symbol: symbol.clone(),
                extended_bid,
                extended_ask,
                extended_mid,
                pacifica_bid,
                pacifica_ask,
                pacifica_mid,
                price_diff,
                spread_pct,
            });
        }
    }

    // Sort by absolute spread (best opportunities first)
    spread_data_list.sort_by(|a, b| {
        b.spread_pct.abs().partial_cmp(&a.spread_pct.abs()).unwrap()
    });

    // Display results
    println!("╔═══════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║ Symbol │  Extended Mid  │  Pacifica Mid  │   Price Diff   │  Spread %  │      Status       ║");
    println!("╠═══════════════════════════════════════════════════════════════════════════════════════════════╣");

    for data in &spread_data_list {
        let status = data.opportunity_status();
        println!("║ {:6} │  ${:11.2} │  ${:11.2} │  ${:11.2}  │  {:7.3}%   │ {:17} ║",
                 data.symbol,
                 data.extended_mid,
                 data.pacifica_mid,
                 data.price_diff,
                 data.spread_pct,
                 status);
    }

    println!("╚═══════════════════════════════════════════════════════════════════════════════════════════════╝\n");

    // Detailed arbitrage opportunities
    println!("╔═══════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                         ARBITRAGE OPPORTUNITIES                                   ║");
    println!("╚═══════════════════════════════════════════════════════════════════════════════════╝\n");

    for data in &spread_data_list {
        if data.spread_pct.abs() >= 0.05 {
            println!("📊 {} - {}", data.symbol, data.opportunity_status());
            println!("   Cheaper: {} @ ${:.2}", data.cheaper_exchange(),
                     if data.extended_mid < data.pacifica_mid { data.extended_mid } else { data.pacifica_mid });
            println!("   More Expensive: {} @ ${:.2}", data.more_expensive_exchange(),
                     if data.extended_mid > data.pacifica_mid { data.extended_mid } else { data.pacifica_mid });
            println!("   💡 {}", data.arbitrage_direction());
            println!("   📈 Potential Profit: {:.3}% (before fees)\n", data.spread_pct.abs());
        }
    }

    if spread_data_list.iter().all(|d| d.spread_pct.abs() < 0.05) {
        println!("✅ No significant arbitrage opportunities found (all spreads < 0.05%)\n");
    }

    // Summary
    println!("╔═══════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                              INTERPRETATION                                       ║");
    println!("╚═══════════════════════════════════════════════════════════════════════════════════╝\n");

    println!("📊 Cross-Exchange Spread:");
    println!("   • Positive spread: Pacifica is MORE EXPENSIVE → Buy Extended, Sell Pacifica");
    println!("   • Negative spread: Extended is MORE EXPENSIVE → Buy Pacifica, Sell Extended");
    println!("   • Price Diff = Pacifica Mid - Extended Mid\n");

    println!("🎯 Opportunity Levels:");
    println!("   🚀 EXCELLENT: ≥ 0.20% spread (strong arbitrage opportunity)");
    println!("   ✅ GOOD:      ≥ 0.10% spread (viable arbitrage)");
    println!("   ⚠️  SMALL:    ≥ 0.05% spread (marginal after fees)");
    println!("   ❌ NONE:      < 0.05% spread (not profitable)\n");

    println!("⚠️  Important:");
    println!("   • Consider trading fees (~0.04% maker on both exchanges)");
    println!("   • Account for slippage when executing trades");
    println!("   • Ensure sufficient capital on both exchanges");
    println!("   • Monitor funding rates (cost of holding positions)\n");

    println!("🔧 Usage:");
    println!("   Default symbols: cargo run --example check_cross_spreads");
    println!("   Custom symbols:  cargo run --example check_cross_spreads BTC ETH SOL\n");

    Ok(())
}
