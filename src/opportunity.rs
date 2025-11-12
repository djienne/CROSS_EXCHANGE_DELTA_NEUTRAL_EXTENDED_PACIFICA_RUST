/// Opportunity finding and filtering for cross-exchange arbitrage
use crate::{PacificaTrading, PacificaCredentials, RestClient};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use tokio::task::JoinSet;
use tracing::info;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub filters: FilterConfig,
    pub trading: TradingConfig,
    pub display: DisplayConfig,
    pub performance: PerformanceConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FilterConfig {
    pub min_combined_volume_usd: f64,
    pub max_intra_exchange_spread_pct: f64,
    pub max_cross_exchange_spread_pct: f64,
    pub min_net_apr_pct: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TradingConfig {
    pub max_position_size_usd: f64,
    pub hold_time_hours: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DisplayConfig {
    pub max_opportunities_shown: usize,
    pub show_filtered_out_count: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PerformanceConfig {
    pub fetch_timeout_seconds: u64,
    pub rate_limit_delay_ms: u64,
}

impl Config {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config_str = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path, e))?;
        let config: Config = serde_json::from_str(&config_str)
            .map_err(|e| format!("Failed to parse {}: {}", path, e))?;

        // Validate configuration parameters
        config.validate()?;

        Ok(config)
    }

    /// Validate configuration parameters for sanity
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate filters
        if self.filters.min_combined_volume_usd < 0.0 {
            return Err("min_combined_volume_usd must be non-negative".into());
        }
        if self.filters.min_combined_volume_usd > 1_000_000_000_000.0 {
            return Err("min_combined_volume_usd is unrealistically high (>$1T)".into());
        }

        if self.filters.max_intra_exchange_spread_pct < 0.0 {
            return Err("max_intra_exchange_spread_pct must be non-negative".into());
        }
        if self.filters.max_intra_exchange_spread_pct > 100.0 {
            return Err("max_intra_exchange_spread_pct cannot exceed 100%".into());
        }

        if self.filters.max_cross_exchange_spread_pct < 0.0 {
            return Err("max_cross_exchange_spread_pct must be non-negative".into());
        }
        if self.filters.max_cross_exchange_spread_pct > 100.0 {
            return Err("max_cross_exchange_spread_pct cannot exceed 100%".into());
        }

        if self.filters.min_net_apr_pct < -1000.0 {
            return Err("min_net_apr_pct is unrealistically low (<-1000%)".into());
        }
        if self.filters.min_net_apr_pct > 100000.0 {
            return Err("min_net_apr_pct is unrealistically high (>100,000%)".into());
        }

        // Validate trading config
        if self.trading.max_position_size_usd <= 0.0 {
            return Err("max_position_size_usd must be positive".into());
        }
        if self.trading.max_position_size_usd > 10_000_000.0 {
            return Err("max_position_size_usd is very high (>$10M). Please verify this is intentional.".into());
        }
        if self.trading.hold_time_hours == 0 {
            return Err("hold_time_hours must be positive".into());
        }
        if self.trading.hold_time_hours > 720 {
            return Err("hold_time_hours is very high (>30 days). Please verify this is intentional.".into());
        }

        // Validate performance config
        if self.performance.fetch_timeout_seconds == 0 {
            return Err("fetch_timeout_seconds must be positive".into());
        }
        if self.performance.fetch_timeout_seconds > 600 {
            return Err("fetch_timeout_seconds is very high (>10 minutes)".into());
        }

        Ok(())
    }

    pub fn default_config() -> Self {
        Config {
            filters: FilterConfig {
                min_combined_volume_usd: 10_000_000.0,
                max_intra_exchange_spread_pct: 0.15,
                max_cross_exchange_spread_pct: 0.25,
                min_net_apr_pct: 5.0,
            },
            trading: TradingConfig {
                max_position_size_usd: 1000.0,
                hold_time_hours: 48,
            },
            display: DisplayConfig {
                max_opportunities_shown: 10,
                show_filtered_out_count: true,
            },
            performance: PerformanceConfig {
                fetch_timeout_seconds: 30,
                rate_limit_delay_ms: 100,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VolumeData {
    pub symbol: String,
    pub extended_volume: f64,
    pub pacifica_volume: f64,
    pub total_volume: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Opportunity {
    pub symbol: String,
    pub extended_spread_pct: f64,
    pub pacifica_spread_pct: f64,
    pub cross_spread_pct: f64,
    pub extended_funding_rate_apr: f64,
    pub pacifica_funding_rate_apr: f64,
    pub total_volume_24h: f64,
    pub extended_volume_24h: f64,
    pub pacifica_volume_24h: f64,
    pub best_direction: String,
    pub best_net_apr: f64,
}

#[derive(Debug, Clone)]
pub struct FilterStats {
    pub total_common_symbols: usize,
    pub filtered_by_volume: usize,
    pub filtered_by_spread: usize,
    pub filtered_by_apr: usize,
    pub passed_filters: usize,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub opportunities: Vec<Opportunity>,
    pub all_candidates: Vec<OpportunityCandidate>,
    pub stats: FilterStats,
}

#[derive(Debug, Clone)]
pub struct OpportunityCandidate {
    pub opportunity: Opportunity,
    pub filter_result: FilterResult,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterResult {
    Passed,
    FailedVolume,
    FailedIntraSpread,
    FailedCrossSpread,
    FailedApr,
}

impl ScanResult {
    /// Display comprehensive scan summary table
    pub fn display_summary(&self, config: &FilterConfig) {
        info!("╔════════════════════════════════════════════════════════════════════════════════╗");
        info!("║                          {}                              ║",
            "OPPORTUNITY SCAN SUMMARY");
        info!("╠════════════════════════════════════════════════════════════════════════════════╣");

        // Filter statistics
        info!("║ Markets Scanned:     {:>59} ║", self.stats.total_common_symbols.to_string());
        info!("║ Passed All Filters:  {:>59} ║", self.stats.passed_filters.to_string());
        info!("║ Filtered (Volume):   {:>59} ║", self.stats.filtered_by_volume.to_string());
        info!("║ Filtered (Spread):   {:>59} ║", self.stats.filtered_by_spread.to_string());
        info!("║ Filtered (APR):      {:>59} ║", self.stats.filtered_by_apr.to_string());
        info!("╠════════════════════════════════════════════════════════════════════════════════╣");

        // Filter criteria
        info!("║                             {}                                    ║",
            "FILTER CRITERIA");
        info!("╠════════════════════════════════════════════════════════════════════════════════╣");
        info!("║ Min Volume:          {:>59} ║", format_volume(config.min_combined_volume_usd));
        info!("║ Max Intra Spread:    {:>58}% ║", format!("{}", config.max_intra_exchange_spread_pct));
        info!("║ Max Cross Spread:    {:>58}% ║", format!("{}", config.max_cross_exchange_spread_pct));
        info!("║ Min Net APR:         {:>58}% ║", format!("{}", config.min_net_apr_pct));

        if !self.opportunities.is_empty() {
            info!("╠════════════════════════════════════════════════════════════════════════════════╣");
            info!("║                        {}                          ║",
                "OPPORTUNITIES (PASSED FILTERS)");
            info!("╠════════════════════════════════════════════════════════════════════════════════╣");
            info!("║ {} │ {}   │ {} │ {}           │ {} │ {} │ {}   ║",
                "Sym", "Volume", "Net APR", "Strategy",
                "Ext FR", "Pac FR", "Spreads");
            info!("╠════════════════════════════════════════════════════════════════════════════════╣");

            for opp in &self.opportunities {
                let sym = format!("{:4}", truncate(&opp.symbol, 4));
                let vol = format!("{:>8}", format_volume(opp.total_volume_24h));
                let apr_formatted = format!("{:>6.1}%", opp.best_net_apr);

                let strategy = if opp.best_direction.contains("Long Extended") {
                    "L.Ext/S.Pac"
                } else {
                    "L.Pac/S.Ext"
                };

                let ext_fr = format!("{:>5.1}%", opp.extended_funding_rate_apr);
                let pac_fr = format!("{:>5.1}%", opp.pacifica_funding_rate_apr);
                let spreads = format!("{:.2}/{:.2}/{:.2}",
                    opp.extended_spread_pct,
                    opp.pacifica_spread_pct,
                    opp.cross_spread_pct
                );

                info!("║ {} │ {} │ {} │ {:18} │ {} │ {} │ {:9} ║",
                    sym, vol, apr_formatted, strategy, ext_fr, pac_fr, spreads);
            }
        }

        if self.stats.filtered_by_volume + self.stats.filtered_by_spread + self.stats.filtered_by_apr > 0 {
            info!("╠════════════════════════════════════════════════════════════════════════════════╣");
            info!("║                      {}                           ║",
                "FILTERED OUT (TOP 10 BY VOLUME)");
            info!("╠════════════════════════════════════════════════════════════════════════════════╣");
            info!("║ {} │ {}   │ {} │ {}              │ {}                      ║",
                "Sym", "Volume", "Net APR",
                "Reason", "Detail");
            info!("╠════════════════════════════════════════════════════════════════════════════════╣");

            let mut filtered: Vec<_> = self.all_candidates.iter()
                .filter(|c| c.filter_result != FilterResult::Passed)
                .collect();
            filtered.sort_by(|a, b|
                b.opportunity.total_volume_24h
                    .partial_cmp(&a.opportunity.total_volume_24h)
                    .unwrap()
            );

            for candidate in filtered.iter().take(10) {
                let opp = &candidate.opportunity;
                let sym = format!("{:4}", truncate(&opp.symbol, 4));
                let vol = format!("{:>8}", format_volume(opp.total_volume_24h));
                let apr = format!("{:>6.1}%", opp.best_net_apr);

                let (reason, detail) = match candidate.filter_result {
                    FilterResult::FailedVolume => {
                        ("Volume too low", format_volume(opp.total_volume_24h))
                    },
                    FilterResult::FailedIntraSpread => {
                        ("Spread too wide", format!("E:{:.2}% P:{:.2}%",
                            opp.extended_spread_pct, opp.pacifica_spread_pct))
                    },
                    FilterResult::FailedCrossSpread => {
                        ("Cross spread", format!("{:.2}%", opp.cross_spread_pct))
                    },
                    FilterResult::FailedApr => {
                        ("APR too low", format!("{:.1}%", opp.best_net_apr))
                    },
                    FilterResult::Passed => continue,
                };

                info!("║ {} │ {} │ {} │ {:19} │ {:<27} ║",
                    sym, vol, apr, reason, detail);
            }
        }

        info!("╚════════════════════════════════════════════════════════════════════════════════╝");
    }
}

impl Opportunity {
    pub fn passes_filters(&self, config: &FilterConfig) -> bool {
        self.extended_spread_pct <= config.max_intra_exchange_spread_pct
            && self.pacifica_spread_pct <= config.max_intra_exchange_spread_pct
            && self.cross_spread_pct <= config.max_cross_exchange_spread_pct
            && self.total_volume_24h >= config.min_combined_volume_usd
            && self.best_net_apr >= config.min_net_apr_pct
    }

    pub fn check_filters(&self, config: &FilterConfig) -> FilterResult {
        if self.total_volume_24h < config.min_combined_volume_usd {
            return FilterResult::FailedVolume;
        }
        if self.extended_spread_pct > config.max_intra_exchange_spread_pct
            || self.pacifica_spread_pct > config.max_intra_exchange_spread_pct {
            return FilterResult::FailedIntraSpread;
        }
        if self.cross_spread_pct > config.max_cross_exchange_spread_pct {
            return FilterResult::FailedCrossSpread;
        }
        if self.best_net_apr < config.min_net_apr_pct {
            return FilterResult::FailedApr;
        }
        FilterResult::Passed
    }

    pub fn quality_rating(&self) -> &str {
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

// Utility functions for display
pub fn format_volume(v: f64) -> String {
    if v >= 1_000_000.0 {
        format!("${:.1}M", v / 1_000_000.0)
    } else if v >= 1_000.0 {
        format!("${:.1}K", v / 1_000.0)
    } else {
        format!("${:.0}", v)
    }
}

pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

pub struct OpportunityFinder {
    extended_client: RestClient,
    pacifica_creds: PacificaCredentials,
    config: Config,
}

impl OpportunityFinder {
    pub fn new(
        extended_api_key: Option<String>,
        pacifica_creds: PacificaCredentials,
        config: Config,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let extended_client = RestClient::new_mainnet(extended_api_key)?;

        Ok(Self {
            extended_client,
            pacifica_creds,
            config,
        })
    }

    /// Find common symbols between Extended and Pacifica
    pub async fn find_common_symbols(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let extended_markets = self.extended_client.get_all_markets().await?;
        let extended_symbols: HashSet<String> = extended_markets
            .iter()
            .filter_map(|m| m.name.strip_suffix("-USD").map(|s| s.to_string()))
            .collect();

        let mut pacifica_client = PacificaTrading::new(self.pacifica_creds.clone());
        let pacifica_markets = pacifica_client.get_market_info().await?;
        let pacifica_symbols: HashSet<String> = pacifica_markets.keys().cloned().collect();

        let common: Vec<String> = extended_symbols
            .intersection(&pacifica_symbols)
            .cloned()
            .collect();

        Ok(common)
    }

    /// Fetch 24h volumes for all symbols in parallel
    pub async fn fetch_volumes(&self, symbols: &[String], extended_api_key: Option<String>) -> Result<Vec<VolumeData>, Box<dyn std::error::Error>> {
        let mut volume_tasks = JoinSet::new();

        for symbol in symbols {
            let symbol = symbol.clone();
            let api_key = extended_api_key.clone();
            volume_tasks.spawn(async move {
                let extended_vol = fetch_extended_volume_with_key(&format!("{}-USD", symbol), api_key)
                    .await
                    .unwrap_or(0.0);
                let pacifica_vol = fetch_pacifica_volume(&symbol).await.unwrap_or(0.0);
                VolumeData {
                    symbol,
                    extended_volume: extended_vol,
                    pacifica_volume: pacifica_vol,
                    total_volume: extended_vol + pacifica_vol,
                }
            });
        }

        let mut results = Vec::new();
        while let Some(result) = volume_tasks.join_next().await {
            if let Ok(vol_data) = result {
                results.push(vol_data);
            }
        }

        // Sort by total volume descending
        results.sort_by(|a, b| b.total_volume.partial_cmp(&a.total_volume).unwrap());

        Ok(results)
    }

    /// Find opportunities for high-volume symbols
    pub async fn find_opportunities(
        &self,
        symbols: &[String],
        volumes: &[VolumeData],
        extended_api_key: Option<String>,
    ) -> Result<Vec<OpportunityCandidate>, Box<dyn std::error::Error>> {
        let mut opp_tasks = Vec::new();

        for symbol in symbols {
            let symbol = symbol.clone();
            let api_key = extended_api_key.clone();
            let vol_data = volumes
                .iter()
                .find(|v| v.symbol == symbol)
                .unwrap()
                .clone();
            let pacifica_creds = self.pacifica_creds.clone();
            let config = self.config.filters.clone();

            let task = tokio::spawn(async move {
                if let Ok(Some(opp)) = fetch_opportunity_data(symbol, api_key, pacifica_creds, vol_data).await {
                    let filter_result = opp.check_filters(&config);
                    Some(OpportunityCandidate {
                        opportunity: opp,
                        filter_result,
                    })
                } else {
                    None
                }
            });
            opp_tasks.push(task);
        }

        let mut candidates = Vec::new();
        for task in opp_tasks {
            if let Ok(Some(candidate)) = task.await {
                candidates.push(candidate);
            }
        }

        // Sort by net APR descending
        candidates.sort_by(|a, b|
            b.opportunity.best_net_apr
                .partial_cmp(&a.opportunity.best_net_apr)
                .unwrap()
        );

        Ok(candidates)
    }

    /// Complete workflow: find common symbols, fetch volumes, filter, and find opportunities
    pub async fn scan(&self, extended_api_key: Option<String>) -> Result<ScanResult, Box<dyn std::error::Error>> {
        // Find common symbols
        let common_symbols = self.find_common_symbols().await?;
        let total_common = common_symbols.len();

        // Fetch volumes in parallel
        let volumes = self.fetch_volumes(&common_symbols, extended_api_key.clone()).await?;

        // Count volume-filtered symbols
        let filtered_by_volume = volumes
            .iter()
            .filter(|v| v.total_volume < self.config.filters.min_combined_volume_usd)
            .count();

        // Filter by volume
        let high_volume_symbols: Vec<String> = volumes
            .iter()
            .filter(|v| v.total_volume >= self.config.filters.min_combined_volume_usd)
            .map(|v| v.symbol.clone())
            .collect();

        // Find opportunities for high-volume symbols
        let all_candidates = self
            .find_opportunities(&high_volume_symbols, &volumes, extended_api_key)
            .await?;

        // Split into passed and failed
        let opportunities: Vec<Opportunity> = all_candidates
            .iter()
            .filter(|c| c.filter_result == FilterResult::Passed)
            .map(|c| c.opportunity.clone())
            .collect();

        // Count filter failures
        let mut filtered_by_spread = 0;
        let mut filtered_by_apr = 0;

        for candidate in &all_candidates {
            match candidate.filter_result {
                FilterResult::FailedIntraSpread | FilterResult::FailedCrossSpread => {
                    filtered_by_spread += 1;
                }
                FilterResult::FailedApr => {
                    filtered_by_apr += 1;
                }
                _ => {}
            }
        }

        let stats = FilterStats {
            total_common_symbols: total_common,
            filtered_by_volume,
            filtered_by_spread,
            filtered_by_apr,
            passed_filters: opportunities.len(),
        };

        Ok(ScanResult {
            opportunities,
            all_candidates,
            stats,
        })
    }
}

// Helper functions (same as before)
async fn fetch_extended_volume_with_key(
    market: &str,
    api_key: Option<String>,
) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!(
        "https://api.starknet.extended.exchange/api/v1/info/markets/{}/stats",
        market
    );

    let client = reqwest::Client::new();
    let mut request = client
        .get(&url)
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
        Ok(stats
            .data
            .and_then(|d| d.daily_volume.parse::<f64>().ok())
            .unwrap_or(0.0))
    } else {
        Ok(0.0)
    }
}

async fn fetch_pacifica_volume(
    symbol: &str,
) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;
    let start = now - (24 * 60 * 60 * 1000);

    let url = format!(
        "https://api.pacifica.fi/api/v1/kline?symbol={}&interval=1d&start_time={}&end_time={}",
        symbol, start, now
    );

    let response = reqwest::get(&url).await?;

    if response.status().is_success() {
        #[derive(serde::Deserialize)]
        struct KlineResponse {
            data: Option<Vec<Candle>>,
        }

        #[derive(serde::Deserialize)]
        struct Candle {
            #[serde(rename = "v")]
            volume: String,
            #[serde(rename = "c")]
            close: String,
        }

        let kline: KlineResponse = response.json().await?;

        if let Some(candles) = kline.data {
            if let Some(candle) = candles.first() {
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
) -> Result<Option<Opportunity>, Box<dyn std::error::Error + Send + Sync>> {
    let extended_market = format!("{}-USD", symbol);

    let extended_client = RestClient::new_mainnet(extended_api_key)?;
    let mut pacifica_client = PacificaTrading::new(pacifica_creds);

    // Fetch orderbooks
    let (_ext_bid, _ext_ask, ext_mid, ext_spread) =
        match extended_client.get_orderbook(&extended_market).await {
            Ok(ob) => {
                if let (Some(b), Some(a)) = (ob.bid.first(), ob.ask.first()) {
                    let bid = b.price.parse::<f64>().unwrap_or(0.0);
                    let ask = a.price.parse::<f64>().unwrap_or(0.0);
                    let mid = (bid + ask) / 2.0;
                    let spread = if mid > 0.0 {
                        ((ask - bid) / mid) * 100.0
                    } else {
                        999.0
                    };
                    (bid, ask, mid, spread)
                } else {
                    return Ok(None);
                }
            }
            Err(_) => return Ok(None),
        };

    let (_pac_bid, _pac_ask, pac_mid, pac_spread) =
        match pacifica_client.get_orderbook_rest(&symbol, 1).await {
            Ok(ob) => {
                if let (Some(b), Some(a)) = (ob.bids.first(), ob.asks.first()) {
                    let bid = b.price.parse::<f64>().unwrap_or(0.0);
                    let ask = a.price.parse::<f64>().unwrap_or(0.0);
                    let mid = (bid + ask) / 2.0;
                    let spread = if mid > 0.0 {
                        ((ask - bid) / mid) * 100.0
                    } else {
                        999.0
                    };
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
    let ext_funding_apr = match extended_client.get_funding_rate(&extended_market).await {
        Ok(Some(fr)) => fr.rate_percentage * 3.0 * 365.0,
        _ => 0.0,
    };

    let pac_funding_apr = match pacifica_client.get_funding_rate(&symbol).await {
        Ok(fr) => fr.rate_percentage * 24.0 * 365.0,
        Err(_) => 0.0,
    };

    let net_apr_long_ext = -ext_funding_apr + pac_funding_apr;
    let net_apr_long_pac = -pac_funding_apr + ext_funding_apr;

    let (best_direction, best_net_apr) = if net_apr_long_ext > net_apr_long_pac {
        (
            "Long Extended / Short Pacifica".to_string(),
            net_apr_long_ext,
        )
    } else {
        (
            "Long Pacifica / Short Extended".to_string(),
            net_apr_long_pac,
        )
    };

    Ok(Some(Opportunity {
        symbol,
        extended_spread_pct: ext_spread,
        pacifica_spread_pct: pac_spread,
        cross_spread_pct: cross_spread,
        extended_funding_rate_apr: ext_funding_apr,
        pacifica_funding_rate_apr: pac_funding_apr,
        total_volume_24h: vol_data.total_volume,
        extended_volume_24h: vol_data.extended_volume,
        pacifica_volume_24h: vol_data.pacifica_volume,
        best_direction,
        best_net_apr,
    }))
}
