use extended_connector::{init_logging, PacificaTrading, PacificaCredentials, RestClient};
use std::collections::HashSet;

#[derive(Debug, Clone)]
struct OpportunityData {
    symbol: String,
    // Extended data
    extended_bid: f64,
    extended_ask: f64,
    extended_mid: f64,
    extended_spread_pct: f64,
    extended_funding_rate_apr: f64,
    extended_volume_24h: f64,
    // Pacifica data
    pacifica_bid: f64,
    pacifica_ask: f64,
    pacifica_mid: f64,
    pacifica_spread_pct: f64,
    pacifica_funding_rate_apr: f64,
    pacifica_volume_24h: f64,
    // Combined metrics
    total_volume_24h: f64,
    cross_spread_pct: f64,
    // Arbitrage metrics
    funding_diff_apr: f64,
    net_apr_long_extended: f64,
    net_apr_long_pacifica: f64,
    best_direction: String,
    best_net_apr: f64,
}

impl OpportunityData {
    fn passes_filters(&self, min_volume: f64, max_intra_spread: f64, max_cross_spread: f64) -> bool {
        self.extended_spread_pct <= max_intra_spread
            && self.pacifica_spread_pct <= max_intra_spread
            && self.cross_spread_pct <= max_cross_spread
            && self.total_volume_24h >= min_volume
    }

    fn format_volume(v: f64) -> String {
        if v >= 1_000_000.0 {
            format!("${:.1}M", v / 1_000_000.0)
        } else if v >= 1_000.0 {
            format!("${:.1}K", v / 1_000.0)
        } else {
            format!("${:.0}", v)
        }
    }

    fn quality_rating(&self) -> &str {
        let net_apr = self.best_net_apr;
        if net_apr >= 100.0 {
            "🚀 EXCELLENT"
        } else if net_apr >= 50.0 {
            "✅ VERY GOOD"
        } else if net_apr >= 20.0 {
            "✅ GOOD"
        } else if net_apr >= 10.0 {
            "⚠️  MODERATE"
        } else {
            "❌ WEAK"
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║      OPTIMIZED ARBITRAGE OPPORTUNITY FINDER                      ║");
    println!("║          (Parallel fetching + flexible filters)                  ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Filter parameters
    let min_volume = 10_000_000.0;  // $10M minimum
    let max_intra_spread = 0.50;     // 0.5% max intra-exchange spread (relaxed from 0.15%)
    let max_cross_spread = 1.0;      // 1.0% max cross-exchange spread (relaxed from 0.15%)

    println!("📊 Filter Settings:");
    println!("   • Min Combined Volume: ${:.0}M", min_volume / 1_000_000.0);
    println!("   • Max Intra-Exchange Spread: {:.2}%", max_intra_spread);
    println!("   • Max Cross-Exchange Spread: {:.2}%\n", max_cross_spread);

    // Load API keys
    dotenv::dotenv().ok();
    let extended_api_key = std::env::var("EXTENDED_API_KEY")
        .or_else(|_| std::env::var("API_KEY"))
        .ok();

    // Initialize clients
    let extended_client = if let Some(ref api_key) = extended_api_key {
        Some(RestClient::new_mainnet(Some(api_key.clone()))?)
    } else {
        println!("❌ Extended API key required\n");
        return Ok(());
    };

    let mut pacifica_client = match PacificaCredentials::from_env() {
        Ok(credentials) => Some(PacificaTrading::new(credentials)),
        Err(_) => {
            println!("❌ Pacifica credentials required\n");
            return Ok(());
        }
    };

    println!("📊 Step 1: Finding common markets...\n");

    // Get Extended markets
    let extended_markets = extended_client.as_ref().unwrap().get_all_markets().await?;
    let extended_symbols: HashSet<String> = extended_markets
        .iter()
        .filter_map(|m| m.name.strip_suffix("-USD").map(|s| s.to_string()))
        .collect();

    // Get Pacifica markets
    let pacifica_markets = pacifica_client.as_mut().unwrap().get_market_info().await?;
    let pacifica_symbols: HashSet<String> = pacifica_markets.keys().cloned().collect();

    // Find common symbols
    let common_symbols: Vec<String> = extended_symbols
        .intersection(&pacifica_symbols)
        .cloned()
        .collect();

    println!("   Extended: {} markets", extended_symbols.len());
    println!("   Pacifica: {} markets", pacifica_symbols.len());
    println!("   ✅ Common: {} markets\n", common_symbols.len());

    println!("📊 Step 2: Fetching market data...\n");
    let start_time = std::time::Instant::now();

    let mut opportunities: Vec<OpportunityData> = Vec::new();
    let mut filtered_out = 0;
    let mut processed = 0;

    for symbol in &common_symbols {
        processed += 1;
        print!("   [{}/{}] Fetching {}...\r", processed, common_symbols.len(), symbol);
        std::io::Write::flush(&mut std::io::stdout()).ok();

        if let Ok(Some(opp)) = fetch_market_data(
            symbol.clone(),
            extended_client.as_ref().unwrap(),
            pacifica_client.as_mut().unwrap()
        ).await {
            if opp.passes_filters(min_volume, max_intra_spread, max_cross_spread) {
                opportunities.push(opp);
            } else {
                filtered_out += 1;
            }
        }

        // Small delay to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    let elapsed = start_time.elapsed();
    println!("\n   ✅ Fetched all markets in {:.2}s\n", elapsed.as_secs_f64());

    println!("📊 Step 3: Filtering and ranking...\n");
    println!("   ✅ Passed filters: {}", opportunities.len());
    println!("   ⚠️  Filtered out: {}\n", filtered_out);

    if opportunities.is_empty() {
        println!("❌ No opportunities found matching criteria\n");
        return Ok(());
    }

    // Sort by net APR (highest first), then by volume
    opportunities.sort_by(|a, b| {
        b.best_net_apr
            .partial_cmp(&a.best_net_apr)
            .unwrap()
            .then_with(|| b.total_volume_24h.partial_cmp(&a.total_volume_24h).unwrap())
    });

    // Display summary table
    println!("╔════════════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                           FILTERED OPPORTUNITIES (Sorted by Net APR)                                ║");
    println!("╠════════════════════════════════════════════════════════════════════════════════════════════════════╣");
    println!("║ Symbol │  Total Vol  │ Ext Sprd │ Pac Sprd │ Cross Sprd │ Ext FR │ Pac FR │  Net APR  │ Quality ║");
    println!("╠════════════════════════════════════════════════════════════════════════════════════════════════════╣");

    for opp in &opportunities {
        println!(
            "║ {:6} │ {:>11} │  {:5.2}%   │  {:5.2}%   │   {:5.2}%    │ {:5.1}% │ {:5.1}% │ {:8.2}% │ {:>7} ║",
            opp.symbol,
            OpportunityData::format_volume(opp.total_volume_24h),
            opp.extended_spread_pct,
            opp.pacifica_spread_pct,
            opp.cross_spread_pct,
            opp.extended_funding_rate_apr,
            opp.pacifica_funding_rate_apr,
            opp.best_net_apr,
            if opp.best_net_apr >= 50.0 { "🚀" } else if opp.best_net_apr >= 20.0 { "✅" } else { "⚠️" }
        );
    }

    println!("╚════════════════════════════════════════════════════════════════════════════════════════════════════╝\n");

    // Show detailed top 5
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                   TOP 5 OPPORTUNITIES DETAIL                     ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    for (idx, opp) in opportunities.iter().take(5).enumerate() {
        println!("{}. {} - {}", idx + 1, opp.symbol, opp.quality_rating());
        println!("   💰 Net APR: {:.2}%", opp.best_net_apr);
        println!("   📊 Strategy: {}", opp.best_direction);
        println!("   📈 Volume: {} (Extended: {}, Pacifica: {})",
                 OpportunityData::format_volume(opp.total_volume_24h),
                 OpportunityData::format_volume(opp.extended_volume_24h),
                 OpportunityData::format_volume(opp.pacifica_volume_24h));
        println!("   📉 Spreads: Extended {:.3}%, Pacifica {:.3}%, Cross {:.3}%",
                 opp.extended_spread_pct, opp.pacifica_spread_pct, opp.cross_spread_pct);
        println!("   💸 Funding: Extended {:.2}% APR, Pacifica {:.2}% APR\n",
                 opp.extended_funding_rate_apr, opp.pacifica_funding_rate_apr);
    }

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                           SUMMARY                                ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let best_opportunity = opportunities.first().unwrap();
    println!("🏆 Best Opportunity: {}", best_opportunity.symbol);
    println!("   • Net APR: {:.2}%", best_opportunity.best_net_apr);
    println!("   • Strategy: {}", best_opportunity.best_direction);
    println!("   • Volume: {}", OpportunityData::format_volume(best_opportunity.total_volume_24h));
    println!("\n⚡ Performance: Fetched and analyzed {} markets in {:.2}s\n", common_symbols.len(), elapsed.as_secs_f64());

    Ok(())
}

async fn fetch_market_data(
    symbol: String,
    extended_client: &RestClient,
    pacifica_client: &mut PacificaTrading,
) -> Result<Option<OpportunityData>, Box<dyn std::error::Error + Send + Sync>> {
    let extended_market = format!("{}-USD", symbol);

    // Fetch Extended orderbook (top of book only by extracting first bid/ask)
    let (extended_bid, extended_ask, extended_mid, extended_spread_pct) =
        match extended_client.get_orderbook(&extended_market).await {
            Ok(orderbook) => {
                if let (Some(best_bid), Some(best_ask)) = (orderbook.bid.first(), orderbook.ask.first()) {
                    let bid = best_bid.price.parse::<f64>().unwrap_or(0.0);
                    let ask = best_ask.price.parse::<f64>().unwrap_or(0.0);
                    let mid = (bid + ask) / 2.0;
                    let spread_pct = if mid > 0.0 { ((ask - bid) / mid) * 100.0 } else { 999.0 };
                    (bid, ask, mid, spread_pct)
                } else {
                    return Ok(None);
                }
            }
            Err(_) => return Ok(None),
        };

    // Fetch Pacifica orderbook (1 level only)
    let (pacifica_bid, pacifica_ask, pacifica_mid, pacifica_spread_pct) =
        match pacifica_client.get_orderbook_rest(&symbol, 1).await {
            Ok(orderbook) => {
                if let (Some(best_bid), Some(best_ask)) = (orderbook.bids.first(), orderbook.asks.first()) {
                    let bid = best_bid.price.parse::<f64>().unwrap_or(0.0);
                    let ask = best_ask.price.parse::<f64>().unwrap_or(0.0);
                    let mid = (bid + ask) / 2.0;
                    let spread_pct = if mid > 0.0 { ((ask - bid) / mid) * 100.0 } else { 999.0 };
                    (bid, ask, mid, spread_pct)
                } else {
                    return Ok(None);
                }
            }
            Err(_) => return Ok(None),
        };

    if extended_mid == 0.0 || pacifica_mid == 0.0 {
        return Ok(None);
    }

    // Calculate cross-exchange spread
    let cross_spread_pct = ((pacifica_mid - extended_mid).abs() / extended_mid) * 100.0;

    // Fetch volumes (these are slower, but necessary)
    let extended_volume_24h = fetch_extended_volume(&extended_market).await.unwrap_or(0.0);
    let pacifica_volume_24h = fetch_pacifica_volume(&symbol).await.unwrap_or(0.0);
    let total_volume_24h = extended_volume_24h + pacifica_volume_24h;

    // Fetch funding rates
    let extended_funding_rate_apr = match extended_client.get_funding_rate(&extended_market).await {
        Ok(Some(funding_info)) => funding_info.rate_percentage * 3.0 * 365.0,
        _ => 0.0,
    };

    let pacifica_funding_rate_apr = match pacifica_client.get_funding_rate(&symbol).await {
        Ok(funding_rate) => funding_rate.rate_percentage * 24.0 * 365.0,
        Err(_) => 0.0,
    };

    // Calculate arbitrage metrics
    let funding_diff_apr = extended_funding_rate_apr - pacifica_funding_rate_apr;
    let net_apr_long_extended = -extended_funding_rate_apr + pacifica_funding_rate_apr;
    let net_apr_long_pacifica = -pacifica_funding_rate_apr + extended_funding_rate_apr;

    let (best_direction, best_net_apr) = if net_apr_long_extended > net_apr_long_pacifica {
        ("Long Extended / Short Pacifica".to_string(), net_apr_long_extended)
    } else {
        ("Long Pacifica / Short Extended".to_string(), net_apr_long_pacifica)
    };

    Ok(Some(OpportunityData {
        symbol,
        extended_bid,
        extended_ask,
        extended_mid,
        extended_spread_pct,
        extended_funding_rate_apr,
        extended_volume_24h,
        pacifica_bid,
        pacifica_ask,
        pacifica_mid,
        pacifica_spread_pct,
        pacifica_funding_rate_apr,
        pacifica_volume_24h,
        total_volume_24h,
        cross_spread_pct,
        funding_diff_apr,
        net_apr_long_extended,
        net_apr_long_pacifica,
        best_direction,
        best_net_apr,
    }))
}

async fn fetch_extended_volume(market: &str) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("https://api.starknet.extended.exchange/api/v1/info/markets/{}/stats", market);
    let response = reqwest::get(&url).await?;

    if response.status().is_success() {
        #[derive(serde::Deserialize)]
        struct StatsResponse {
            data: Option<MarketStats>,
        }
        #[derive(serde::Deserialize)]
        struct MarketStats {
            #[serde(rename = "dailyVolume")]
            daily_volume: String,
        }

        let stats: StatsResponse = response.json().await?;
        Ok(stats.data
            .and_then(|d| d.daily_volume.parse::<f64>().ok())
            .unwrap_or(0.0))
    } else {
        Ok(0.0)
    }
}

async fn fetch_pacifica_volume(symbol: &str) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("https://api.pacifica.fi/api/v1/info/markets/{}/stats", symbol);
    let response = reqwest::get(&url).await?;

    if response.status().is_success() {
        #[derive(serde::Deserialize)]
        struct StatsResponse {
            data: Option<PacificaStats>,
        }
        #[derive(serde::Deserialize)]
        struct PacificaStats {
            #[serde(rename = "dailyVolume")]
            daily_volume: String,
        }

        let stats: StatsResponse = response.json().await?;
        Ok(stats.data
            .and_then(|d| d.daily_volume.parse::<f64>().ok())
            .unwrap_or(0.0))
    } else {
        Ok(0.0)
    }
}

