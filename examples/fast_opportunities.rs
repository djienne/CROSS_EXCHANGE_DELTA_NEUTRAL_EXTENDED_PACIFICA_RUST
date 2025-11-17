use extended_connector::{init_logging, PacificaTrading, PacificaCredentials, RestClient};
use std::collections::HashSet;
use tokio::task::JoinSet;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct Config {
    filters: FilterConfig,
    display: DisplayConfig,
    performance: PerformanceConfig,
}

#[derive(Debug, Deserialize)]
struct FilterConfig {
    min_combined_volume_usd: f64,
    max_intra_exchange_spread_pct: f64,
    max_cross_exchange_spread_pct: f64,
}

#[derive(Debug, Deserialize)]
struct DisplayConfig {
    max_opportunities_shown: usize,
    show_filtered_out_count: bool,
}

#[derive(Debug, Deserialize)]
struct PerformanceConfig {
    fetch_timeout_seconds: u64,
    rate_limit_delay_ms: u64,
}

impl Config {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = "config.json";
        let config_str = fs::read_to_string(config_path)
            .map_err(|e| format!("Failed to read {}: {}", config_path, e))?;
        let config: Config = serde_json::from_str(&config_str)
            .map_err(|e| format!("Failed to parse {}: {}", config_path, e))?;
        Ok(config)
    }
}

#[derive(Debug, Clone)]
struct VolumeData {
    symbol: String,
    extended_volume: f64,
    pacifica_volume: f64,
    total_volume: f64,
}

#[derive(Debug, Clone)]
struct OpportunityData {
    symbol: String,
    extended_spread_pct: f64,
    pacifica_spread_pct: f64,
    cross_spread_pct: f64,
    extended_funding_rate_apr: f64,
    pacifica_funding_rate_apr: f64,
    total_volume_24h: f64,
    best_direction: String,
    best_net_apr: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║        ULTRA-FAST OPPORTUNITY FINDER                            ║");
    println!("║        (Volume filter first + parallel fetching)                ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Load configuration from config.json
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to load config.json: {}", e);
            eprintln!("💡 Make sure config.json exists in the project root\n");
            return Err(e);
        }
    };

    println!("📊 Filter Settings (from config.json):");
    println!("   • Min Combined Volume: ${:.0}M", config.filters.min_combined_volume_usd / 1_000_000.0);
    println!("   • Max Intra-Exchange Spread: {:.2}%", config.filters.max_intra_exchange_spread_pct);
    println!("   • Max Cross-Exchange Spread: {:.2}%", config.filters.max_cross_exchange_spread_pct);
    println!("   • Max Opportunities to Display: {}\n", config.display.max_opportunities_shown);

    let min_volume = config.filters.min_combined_volume_usd;
    let max_intra_spread = config.filters.max_intra_exchange_spread_pct;
    let max_cross_spread = config.filters.max_cross_exchange_spread_pct;

    // Load API keys
    dotenv::dotenv().ok();
    let extended_api_key = std::env::var("EXTENDED_API_KEY")
        .or_else(|_| std::env::var("API_KEY"))
        .ok();

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

    let extended_markets = extended_client.as_ref().unwrap().get_all_markets().await?;
    let extended_symbols: HashSet<String> = extended_markets
        .iter()
        .filter_map(|m| m.name.strip_suffix("-USD").map(|s| s.to_string()))
        .collect();

    let pacifica_markets = pacifica_client.as_mut().unwrap().get_market_info().await?;
    let pacifica_symbols: HashSet<String> = pacifica_markets.keys().cloned().collect();

    let common_symbols: Vec<String> = extended_symbols
        .intersection(&pacifica_symbols)
        .cloned()
        .collect();

    println!("   Extended: {} markets", extended_symbols.len());
    println!("   Pacifica: {} markets", pacifica_symbols.len());
    println!("   ✅ Common: {} markets\n", common_symbols.len());

    // STEP 2: Fetch volumes in PARALLEL (this is the key optimization!)
    println!("📊 Step 2: Fetching 24h volumes in parallel...\n");
    let start_volume_fetch = std::time::Instant::now();

    let mut volume_tasks = JoinSet::new();

    for symbol in &common_symbols {
        let symbol = symbol.clone();
        let api_key = extended_api_key.clone();
        volume_tasks.spawn(async move {
            let extended_vol = fetch_extended_volume_with_key(&format!("{}-USD", symbol), api_key).await.unwrap_or(0.0);
            let pacifica_vol = fetch_pacifica_volume(&symbol).await.unwrap_or(0.0);
            VolumeData {
                symbol,
                extended_volume: extended_vol,
                pacifica_volume: pacifica_vol,
                total_volume: extended_vol + pacifica_vol,
            }
        });
    }

    let mut volume_results = Vec::new();
    while let Some(result) = volume_tasks.join_next().await {
        if let Ok(vol_data) = result {
            volume_results.push(vol_data);
        }
    }

    volume_results.sort_by(|a, b| b.total_volume.partial_cmp(&a.total_volume).unwrap());

    let volume_fetch_time = start_volume_fetch.elapsed();
    println!("   ✅ Fetched all volumes in {:.2}s\n", volume_fetch_time.as_secs_f64());

    // STEP 3: Display volume data and filter
    println!("╔════════════════════════════════════════════════════════════════════════════╗");
    println!("║                    24H VOLUME DATA (All Common Markets)                    ║");
    println!("╠════════════════════════════════════════════════════════════════════════════╣");
    println!("║ Symbol │  Extended Vol  │  Pacifica Vol  │   Total Vol   │  Status         ║");
    println!("╠════════════════════════════════════════════════════════════════════════════╣");

    let mut high_volume_symbols = Vec::new();

    for vol_data in &volume_results {
        let status = if vol_data.total_volume >= min_volume {
            high_volume_symbols.push(vol_data.symbol.clone());
            "✅ PASS"
        } else {
            "❌ FAIL"
        };

        println!(
            "║ {:6} │  {:>12}  │  {:>12}  │  {:>12} │  {:14} ║",
            vol_data.symbol,
            format_volume(vol_data.extended_volume),
            format_volume(vol_data.pacifica_volume),
            format_volume(vol_data.total_volume),
            status
        );
    }

    println!("╚════════════════════════════════════════════════════════════════════════════╝\n");

    println!("   ✅ Passed volume filter (≥ ${:.0}M): {} markets", min_volume / 1_000_000.0, high_volume_symbols.len());
    println!("   ❌ Failed volume filter: {} markets\n", common_symbols.len() - high_volume_symbols.len());

    if high_volume_symbols.is_empty() {
        println!("❌ No markets with sufficient volume found!\n");
        println!("💡 Suggestion: Try lowering the minimum volume threshold\n");
        return Ok(());
    }

    // STEP 4: Fetch orderbooks + funding rates for high-volume markets in PARALLEL
    println!("📊 Step 3: Fetching orderbooks & funding rates for {} high-volume markets...\n", high_volume_symbols.len());
    let start_opp_fetch = std::time::Instant::now();

    let mut opp_tasks = Vec::new();

    for symbol in &high_volume_symbols {
        let symbol = symbol.clone();
        let api_key = extended_api_key.clone();
        let vol_data = volume_results.iter().find(|v| v.symbol == symbol).unwrap().clone();
        let pacifica_creds = PacificaCredentials::from_env()?;

        let task = tokio::spawn(async move {
            fetch_opportunity_data(symbol, api_key, pacifica_creds, vol_data).await
        });
        opp_tasks.push(task);
    }

    let mut opportunities = Vec::new();
    let mut filtered_out = 0;

    for task in opp_tasks {
        if let Ok(Ok(Some(opp))) = task.await {
            if opp.extended_spread_pct <= max_intra_spread
                && opp.pacifica_spread_pct <= max_intra_spread
                && opp.cross_spread_pct <= max_cross_spread
            {
                opportunities.push(opp);
            } else {
                filtered_out += 1;
            }
        }
    }

    let opp_fetch_time = start_opp_fetch.elapsed();
    println!("   ✅ Fetched detailed data in {:.2}s\n", opp_fetch_time.as_secs_f64());

    println!("   ✅ Passed spread filters: {}", opportunities.len());
    println!("   ⚠️  Filtered out (wide spreads): {}\n", filtered_out);

    if opportunities.is_empty() {
        println!("❌ No opportunities passed spread filters\n");
        println!("   Filters applied:");
        println!("   • Intra-exchange spread ≤ {:.2}%", max_intra_spread);
        println!("   • Cross-exchange spread ≤ {:.2}%\n", max_cross_spread);
        return Ok(());
    }

    // Sort by net APR
    opportunities.sort_by(|a, b| b.best_net_apr.partial_cmp(&a.best_net_apr).unwrap());

    // Display results
    println!("╔════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                        BEST ARBITRAGE OPPORTUNITIES                                        ║");
    println!("╠════════════════════════════════════════════════════════════════════════════════════════════╣");
    println!("║ Symbol │  Total Vol  │ Ext Sprd │ Pac Sprd │ Cross │ Net APR │      Strategy             ║");
    println!("╠════════════════════════════════════════════════════════════════════════════════════════════╣");

    for opp in &opportunities {
        println!(
            "║ {:6} │ {:>11} │  {:5.2}%   │  {:5.2}%   │ {:5.2}% │ {:6.1}% │ {:25} ║",
            opp.symbol,
            format_volume(opp.total_volume_24h),
            opp.extended_spread_pct,
            opp.pacifica_spread_pct,
            opp.cross_spread_pct,
            opp.best_net_apr,
            truncate(&opp.best_direction, 25)
        );
    }

    println!("╚════════════════════════════════════════════════════════════════════════════════════════════╝\n");

    // Summary
    let total_time = start_volume_fetch.elapsed();
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                          PERFORMANCE                             ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");
    println!("   ⚡ Total execution time: {:.2}s", total_time.as_secs_f64());
    println!("   ⚡ Volume fetch: {:.2}s ({} markets in parallel)", volume_fetch_time.as_secs_f64(), common_symbols.len());
    println!("   ⚡ Orderbook fetch: {:.2}s ({} markets in parallel)", opp_fetch_time.as_secs_f64(), high_volume_symbols.len());
    println!("   ⚡ Average per market: {:.2}s\n", total_time.as_secs_f64() / common_symbols.len() as f64);

    Ok(())
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

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len-3])
    }
}

async fn fetch_extended_volume_with_key(market: &str, api_key: Option<String>) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("https://api.starknet.extended.exchange/api/v1/info/markets/{}/stats", market);

    let client = reqwest::Client::new();
    let mut request = client.get(&url)
        .header("User-Agent", "extended-connector/0.1.0");

    if let Some(key) = api_key {
        request = request.header("X-Api-Key", key);
    }

    let response = request.send().await?;

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
    // Use kline endpoint to get 24h volume
    // Volume is in BASE currency (e.g., BTC), so we need to convert to USD

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;
    let start = now - (24 * 60 * 60 * 1000); // 24 hours ago

    let url = format!(
        "https://api.pacifica.fi/api/v1/kline?symbol={}&interval=1d&start_time={}&end_time={}",
        symbol, start, now
    );

    let response = reqwest::get(&url).await?;

    if response.status().is_success() {
        #[derive(serde::Deserialize)]
        struct KlineResponse {
            success: bool,
            data: Option<Vec<Candle>>,
        }

        #[derive(serde::Deserialize)]
        struct Candle {
            #[serde(rename = "v")]
            volume: String,  // Volume in BASE currency (e.g., BTC)
            #[serde(rename = "c")]
            close: String,   // Close price in USD
        }

        let kline: KlineResponse = response.json().await?;

        if let Some(candles) = kline.data {
            if let Some(candle) = candles.first() {
                // Convert BASE volume to USD: volume_btc * price_usd
                let vol_base = candle.volume.parse::<f64>().unwrap_or(0.0);
                let price = candle.close.parse::<f64>().unwrap_or(0.0);
                return Ok(vol_base * price);
            }
        }
    }

    Ok(0.0)
}

async fn fetch_opportunity_data(
    symbol: String,
    extended_api_key: Option<String>,
    pacifica_creds: PacificaCredentials,
    vol_data: VolumeData,
) -> Result<Option<OpportunityData>, Box<dyn std::error::Error + Send + Sync>> {
    let extended_market = format!("{}-USD", symbol);

    // Create clients within this task
    let extended_client = RestClient::new_mainnet(extended_api_key)?;
    let mut pacifica_client = PacificaTrading::new(pacifica_creds);

    // Fetch orderbooks
    let (_ext_bid, _ext_ask, ext_mid, ext_spread) = match extended_client.get_orderbook(&extended_market).await {
        Ok(ob) => {
            if let (Some(b), Some(a)) = (ob.bid.first(), ob.ask.first()) {
                let bid = b.price.parse::<f64>().unwrap_or(0.0);
                let ask = a.price.parse::<f64>().unwrap_or(0.0);
                let mid = (bid + ask) / 2.0;
                let spread = if mid > 0.0 { ((ask - bid) / mid) * 100.0 } else { 999.0 };
                (bid, ask, mid, spread)
            } else {
                return Ok(None);
            }
        }
        Err(_) => return Ok(None),
    };

    let (_pac_bid, _pac_ask, pac_mid, pac_spread) = match pacifica_client.get_orderbook_rest(&symbol, 1).await {
        Ok(ob) => {
            if let (Some(b), Some(a)) = (ob.bids.first(), ob.asks.first()) {
                let bid = b.price.parse::<f64>().unwrap_or(0.0);
                let ask = a.price.parse::<f64>().unwrap_or(0.0);
                let mid = (bid + ask) / 2.0;
                let spread = if mid > 0.0 { ((ask - bid) / mid) * 100.0 } else { 999.0 };
                (bid, ask, mid, spread)
            } else {
                return Ok(None);
            }
        }
        Err(_) => return Ok(None),
    };

    if ext_mid == 0.0 || pac_mid == 0.0 {
        return Ok(None);
    }

    let cross_spread = ((pac_mid - ext_mid).abs() / ext_mid) * 100.0;

    // Fetch funding rates
    // Extended funding rates are HOURLY (applied once per hour)
    // Use raw decimal rate, multiply by periods per year, then convert to percentage
    let ext_funding_apr = match extended_client.get_funding_rate(&extended_market).await {
        Ok(Some(fr)) => fr.rate * 24.0 * 365.0 * 100.0,  // rate is decimal, convert to %
        _ => 0.0,
    };

    let pac_funding_apr = match pacifica_client.get_funding_rate(&symbol).await {
        Ok(fr) => fr.next_rate_percentage * 24.0 * 365.0,  // Use next/projected rate (not historical)
        Err(_) => 0.0,
    };

    let net_apr_long_ext = -ext_funding_apr + pac_funding_apr;
    let net_apr_long_pac = -pac_funding_apr + ext_funding_apr;

    let (best_direction, best_net_apr) = if net_apr_long_ext > net_apr_long_pac {
        ("Long Extended / Short Pacifica".to_string(), net_apr_long_ext)
    } else {
        ("Long Pacifica / Short Extended".to_string(), net_apr_long_pac)
    };

    Ok(Some(OpportunityData {
        symbol,
        extended_spread_pct: ext_spread,
        pacifica_spread_pct: pac_spread,
        cross_spread_pct: cross_spread,
        extended_funding_rate_apr: ext_funding_apr,
        pacifica_funding_rate_apr: pac_funding_apr,
        total_volume_24h: vol_data.total_volume,
        best_direction,
        best_net_apr,
    }))
}

