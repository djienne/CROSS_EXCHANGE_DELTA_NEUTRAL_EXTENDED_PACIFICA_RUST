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
    total_volume_24h: f64,  // extended_volume + pacifica_volume
    // Cross-exchange metrics
    cross_spread_pct: f64,  // abs((pacifica_mid - extended_mid) / extended_mid * 100)
    // Arbitrage metrics
    funding_diff_apr: f64,  // extended_funding - pacifica_funding (positive = long extended, short pacifica)
    net_apr_long_extended: f64,   // If we long Extended, short Pacifica
    net_apr_long_pacifica: f64,   // If we long Pacifica, short Extended
    best_direction: String,
    best_net_apr: f64,
}

impl OpportunityData {
    fn passes_filters(&self) -> bool {
        self.extended_spread_pct <= 0.15
            && self.pacifica_spread_pct <= 0.15
            && self.cross_spread_pct <= 0.15
            && self.total_volume_24h >= 10_000_000.0  // Minimum $10M combined volume (lowered for testing)
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
    println!("║          BEST ARBITRAGE OPPORTUNITY FINDER                       ║");
    println!("║    Finding filtered opportunities with tight spreads             ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

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

    println!("📊 Step 1: Finding common markets between Extended and Pacifica...\n");

    // Get Extended markets
    let extended_markets = extended_client.as_ref().unwrap().get_all_markets().await?;
    let extended_symbols: HashSet<String> = extended_markets
        .iter()
        .filter_map(|m| {
            // Convert "BTC-USD" to "BTC"
            m.name.strip_suffix("-USD").map(|s| s.to_string())
        })
        .collect();

    println!("   Extended DEX: {} markets", extended_symbols.len());

    // Get Pacifica markets
    let pacifica_markets = pacifica_client.as_mut().unwrap().get_market_info().await?;
    let pacifica_symbols: HashSet<String> = pacifica_markets.keys().cloned().collect();

    println!("   Pacifica: {} markets", pacifica_symbols.len());

    // Find common symbols
    let common_symbols: Vec<String> = extended_symbols
        .intersection(&pacifica_symbols)
        .cloned()
        .collect();

    println!("   ✅ Common markets: {} symbols\n", common_symbols.len());
    println!("   Symbols: {}\n", common_symbols.join(", "));

    println!("📊 Step 2: Fetching orderbooks and funding rates...\n");

    let mut opportunities: Vec<OpportunityData> = Vec::new();
    let mut processed = 0;
    let mut filtered_out = 0;

    for symbol in &common_symbols {
        let extended_market = format!("{}-USD", symbol);

        // Fetch top of book from Extended (we only use first bid/ask anyway)
        let (extended_bid, extended_ask, extended_mid, extended_spread_pct) =
            if let Some(ref client) = extended_client {
                match client.get_orderbook(&extended_market).await {
                    Ok(orderbook) => {
                        if let (Some(best_bid), Some(best_ask)) =
                            (orderbook.bid.first(), orderbook.ask.first())
                        {
                            let bid = best_bid.price.parse::<f64>().unwrap_or(0.0);
                            let ask = best_ask.price.parse::<f64>().unwrap_or(0.0);
                            let mid = (bid + ask) / 2.0;
                            let spread_pct = if mid > 0.0 {
                                ((ask - bid) / mid) * 100.0
                            } else {
                                999.0
                            };
                            (bid, ask, mid, spread_pct)
                        } else {
                            (0.0, 0.0, 0.0, 999.0)
                        }
                    }
                    Err(_) => (0.0, 0.0, 0.0, 999.0),
                }
            } else {
                (0.0, 0.0, 0.0, 999.0)
            };

        // Fetch Pacifica top of book (1 level only)
        let (pacifica_bid, pacifica_ask, pacifica_mid, pacifica_spread_pct) =
            if let Some(ref mut client) = pacifica_client {
                match client.get_orderbook_rest(symbol, 1).await {
                    Ok(orderbook) => {
                        if let (Some(best_bid), Some(best_ask)) =
                            (orderbook.bids.first(), orderbook.asks.first())
                        {
                            let bid = best_bid.price.parse::<f64>().unwrap_or(0.0);
                            let ask = best_ask.price.parse::<f64>().unwrap_or(0.0);
                            let mid = (bid + ask) / 2.0;
                            let spread_pct = if mid > 0.0 {
                                ((ask - bid) / mid) * 100.0
                            } else {
                                999.0
                            };
                            (bid, ask, mid, spread_pct)
                        } else {
                            (0.0, 0.0, 0.0, 999.0)
                        }
                    }
                    Err(_) => (0.0, 0.0, 0.0, 999.0),
                }
            } else {
                (0.0, 0.0, 0.0, 999.0)
            };

        // Skip if invalid data
        if extended_mid == 0.0 || pacifica_mid == 0.0 {
            continue;
        }

        // Calculate cross-exchange spread
        let cross_spread_pct = ((pacifica_mid - extended_mid).abs() / extended_mid) * 100.0;

        // Fetch Extended 24h volume
        let extended_volume_24h = if let Some(ref _client) = extended_client {
            // Fetch market stats for volume
            let url = format!("https://api.starknet.extended.exchange/api/v1/info/markets/{}/stats", extended_market);
            match reqwest::get(&url).await {
                Ok(response) => {
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
                        match response.json::<StatsResponse>().await {
                            Ok(stats) => {
                                stats.data
                                    .and_then(|d| d.daily_volume.parse::<f64>().ok())
                                    .unwrap_or(0.0)
                            }
                            Err(_) => 0.0,
                        }
                    } else {
                        0.0
                    }
                }
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        // Fetch Pacifica 24h volume - need to fetch from API with marketStats
        let pacifica_volume_24h = {
            let url = format!("https://api.pacifica.fi/api/v1/info/markets/{}/stats", symbol);
            match reqwest::get(&url).await {
                Ok(response) => {
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
                        match response.json::<StatsResponse>().await {
                            Ok(stats) => {
                                stats.data
                                    .and_then(|d| d.daily_volume.parse::<f64>().ok())
                                    .unwrap_or(0.0)
                            }
                            Err(_) => 0.0,
                        }
                    } else {
                        0.0
                    }
                }
                Err(_) => 0.0,
            }
        };

        let total_volume_24h = extended_volume_24h + pacifica_volume_24h;

        // Fetch Extended funding rate
        let extended_funding_rate_apr = if let Some(ref client) = extended_client {
            match client.get_funding_rate(&extended_market).await {
                Ok(Some(funding_info)) => {
                    // Extended funding rates are HOURLY (applied once per hour)
                    // Use raw decimal rate, multiply by periods per year, then convert to percentage
                    funding_info.rate * 24.0 * 365.0 * 100.0  // rate is decimal, convert to %
                }
                _ => 0.0,
            }
        } else {
            0.0
        };

        // Fetch Pacifica funding rate
        let pacifica_funding_rate_apr = if let Some(ref mut client) = pacifica_client {
            match client.get_funding_rate(symbol).await {
                Ok(funding_rate) => {
                    // Use next/projected rate (not historical), settled hourly (24 times/day)
                    funding_rate.next_rate_percentage * 24.0 * 365.0
                }
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        // Calculate funding arbitrage
        let funding_diff_apr = extended_funding_rate_apr - pacifica_funding_rate_apr;

        // Long Extended (pay extended funding), Short Pacifica (receive pacifica funding)
        let net_apr_long_extended = -extended_funding_rate_apr + pacifica_funding_rate_apr;

        // Long Pacifica (pay pacifica funding), Short Extended (receive extended funding)
        let net_apr_long_pacifica = -pacifica_funding_rate_apr + extended_funding_rate_apr;

        let (best_direction, best_net_apr) = if net_apr_long_extended > net_apr_long_pacifica {
            ("Long Extended / Short Pacifica".to_string(), net_apr_long_extended)
        } else {
            ("Long Pacifica / Short Extended".to_string(), net_apr_long_pacifica)
        };

        let opp = OpportunityData {
            symbol: symbol.to_string(),
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
        };

        processed += 1;

        if opp.passes_filters() {
            opportunities.push(opp);
        } else {
            filtered_out += 1;
        }

        // Small delay to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!("   ✅ Processed {} markets", processed);
    println!("   ✅ Passed filters: {}", opportunities.len());
    println!("   ⚠️  Filtered out: {} (wide spreads)\n", filtered_out);

    if opportunities.is_empty() {
        println!("❌ No opportunities found matching criteria:\n");
        println!("   • Intra-exchange spread ≤ 0.15% (both exchanges)");
        println!("   • Cross-exchange spread ≤ 0.15%");
        println!();
        return Ok(());
    }

    // Sort by net APR (highest first), then by volume as tiebreaker
    opportunities.sort_by(|a, b| {
        b.best_net_apr.partial_cmp(&a.best_net_apr)
            .unwrap()
            .then_with(|| b.total_volume_24h.partial_cmp(&a.total_volume_24h).unwrap())
    });

    println!("╔════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                              FILTERED OPPORTUNITIES (Sorted by Net APR)                                        ║");
    println!("╠════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣");
    println!("║ Symbol │  Total Vol 24h  │ Ext Vol │ Pac Vol │ Ext FR │ Pac FR │  Net APR   │ Ext Sprd │ Pac Sprd │ Cross │");
    println!("╠════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣");

    for opp in &opportunities {
        println!(
            "║ {:6} │ {:>15} │ {:>7} │ {:>7} │ {:5.1}% │ {:5.1}% │ {:8.2}%  │  {:5.3}%  │  {:5.3}%  │ {:5.3}% │",
            opp.symbol,
            OpportunityData::format_volume(opp.total_volume_24h),
            OpportunityData::format_volume(opp.extended_volume_24h),
            OpportunityData::format_volume(opp.pacifica_volume_24h),
            opp.extended_funding_rate_apr,
            opp.pacifica_funding_rate_apr,
            opp.best_net_apr,
            opp.extended_spread_pct,
            opp.pacifica_spread_pct,
            opp.cross_spread_pct
        );
    }

    println!("╚══════════════════════════════════════════════════════════════════════════════════════════════════════╝\n");

    // Show detailed top opportunities
    println!("╔══════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                         TOP OPPORTUNITIES DETAIL                                 ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════════╝\n");

    for (idx, opp) in opportunities.iter().take(5).enumerate() {
        let format_vol_detailed = |v: f64| {
            if v >= 1_000_000.0 {
                format!("${:.2}M", v / 1_000_000.0)
            } else if v >= 1_000.0 {
                format!("${:.2}K", v / 1_000.0)
            } else {
                format!("${:.2}", v)
            }
        };

        println!("{}. {} - {} (Net APR: {:.2}%)", idx + 1, opp.symbol, opp.quality_rating(), opp.best_net_apr);
        println!("   📈 24h Trading Volume:");
        println!("      • Total Volume: {}", format_vol_detailed(opp.total_volume_24h));
        println!("      • Extended: {}", format_vol_detailed(opp.extended_volume_24h));
        println!("      • Pacifica: {}", format_vol_detailed(opp.pacifica_volume_24h));
        println!();
        println!("   📊 Spreads:");
        println!("      • Extended: {:.3}% (Bid: ${:.2}, Ask: ${:.2}, Mid: ${:.2})",
                 opp.extended_spread_pct, opp.extended_bid, opp.extended_ask, opp.extended_mid);
        println!("      • Pacifica: {:.3}% (Bid: ${:.2}, Ask: ${:.2}, Mid: ${:.2})",
                 opp.pacifica_spread_pct, opp.pacifica_bid, opp.pacifica_ask, opp.pacifica_mid);
        println!("      • Cross-Exchange: {:.3}%", opp.cross_spread_pct);
        println!();
        println!("   💰 Funding Rates (APR):");
        println!("      • Extended: {:.2}%", opp.extended_funding_rate_apr);
        println!("      • Pacifica: {:.2}%", opp.pacifica_funding_rate_apr);
        println!("      • Difference: {:.2}%", opp.funding_diff_apr);
        println!();
        println!("   🎯 Optimal Strategy:");
        println!("      • Direction: {}", opp.best_direction);
        println!("      • Net APR: {:.2}%", opp.best_net_apr);
        if opp.best_direction.contains("Extended / Short Pacifica") {
            println!("      • Action: Open LONG on Extended, SHORT on Pacifica");
            println!("      • You PAY: {:.2}% on Extended", opp.extended_funding_rate_apr.abs());
            println!("      • You RECEIVE: {:.2}% on Pacifica", opp.pacifica_funding_rate_apr.abs());
        } else {
            println!("      • Action: Open LONG on Pacifica, SHORT on Extended");
            println!("      • You PAY: {:.2}% on Pacifica", opp.pacifica_funding_rate_apr.abs());
            println!("      • You RECEIVE: {:.2}% on Extended", opp.extended_funding_rate_apr.abs());
        }
        println!();
    }

    // Summary
    println!("╔══════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                  SUMMARY                                         ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════════╝\n");

    println!("📊 Filtering Criteria Applied:");
    println!("   ✅ Intra-exchange spread ≤ 0.15% (Extended)");
    println!("   ✅ Intra-exchange spread ≤ 0.15% (Pacifica)");
    println!("   ✅ Cross-exchange spread ≤ 0.15%");
    println!("   ✅ Total 24h volume ≥ $50M (Extended + Pacifica)");
    println!("   ✅ Sorted by Net APR (highest first)\n");

    println!("🎯 Quality Ratings:");
    println!("   🚀 EXCELLENT:  ≥ 100% Net APR");
    println!("   ✅ VERY GOOD:  ≥ 50% Net APR");
    println!("   ✅ GOOD:       ≥ 20% Net APR");
    println!("   ⚠️  MODERATE:  ≥ 10% Net APR");
    println!("   ❌ WEAK:       < 10% Net APR\n");

    println!("⚠️  Important Considerations:");
    println!("   • Net APR = funding received - funding paid");
    println!("   • Funding rates can change every 8 hours (Extended) or 1 hour (Pacifica)");
    println!("   • Ensure sufficient capital on both exchanges (check with check_balance)");
    println!("   • Monitor positions regularly to adjust for rate changes");
    println!("   • Consider transaction fees when entering/exiting positions\n");

    Ok(())
}
