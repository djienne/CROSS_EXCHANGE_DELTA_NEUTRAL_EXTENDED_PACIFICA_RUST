use extended_connector::{
    init_logging, FundingRateInfo, PacificaCredentials, PacificaFundingRate, PacificaTrading,
    RestClient,
};
use prettytable::{color, Attr, Cell, Row, Table};
use std::collections::HashMap;

/// Arbitrage opportunity between two exchanges
#[derive(Debug, Clone)]
struct ArbitrageOpportunity {
    symbol: String,
    /// Net funding rate earned (as percentage)
    net_funding_rate: f64,
    /// Annual percentage rate (hourly rate × 24 × 365)
    apr: f64,
    /// Which exchange to go long on
    long_exchange: String,
    /// Which exchange to go short on
    short_exchange: String,
    /// Funding rate on the long exchange (we pay this)
    long_rate: f64,
    /// Funding rate on the short exchange (we receive this)
    short_rate: f64,
}

impl ArbitrageOpportunity {
    fn new(
        symbol: String,
        pacifica_rate: f64,
        extended_rate: f64,
    ) -> Self {
        // Scenario 1: Long Pacifica, Short Extended
        let profit_pac_long = extended_rate - pacifica_rate;

        // Scenario 2: Long Extended, Short Pacifica
        let profit_ext_long = pacifica_rate - extended_rate;

        // Choose the more profitable scenario
        let (net_funding_rate, long_exchange, short_exchange, long_rate, short_rate) =
            if profit_pac_long >= profit_ext_long {
                (
                    profit_pac_long,
                    "Pacifica".to_string(),
                    "Extended".to_string(),
                    pacifica_rate,
                    extended_rate,
                )
            } else {
                (
                    profit_ext_long,
                    "Extended".to_string(),
                    "Pacifica".to_string(),
                    extended_rate,
                    pacifica_rate,
                )
            };

        // Calculate APR (hourly rate × 24 × 365)
        let apr = net_funding_rate * 24.0 * 365.0;

        Self {
            symbol,
            net_funding_rate,
            apr,
            long_exchange,
            short_exchange,
            long_rate,
            short_rate,
        }
    }
}

/// Map Extended market name to Pacifica symbol
fn map_extended_to_pacifica(extended_market: &str) -> Option<String> {
    // Extended uses format like "BTC-USD", "ETH-USD"
    // Pacifica uses format like "BTC", "ETH"
    if let Some(base) = extended_market.split('-').next() {
        Some(base.to_string())
    } else {
        None
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║   FUNDING RATE ARBITRAGE ANALYZER - Extended vs Pacifica        ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("Fetching funding rates from both exchanges...\n");

    // Fetch Extended funding rates
    println!("📊 Fetching Extended DEX funding rates...");
    let extended_client = RestClient::new_mainnet(None)?;
    let extended_rates = extended_client.get_all_funding_rates().await?;
    println!("   ✓ Retrieved {} markets from Extended", extended_rates.len());

    // Fetch Pacifica funding rates
    println!("📊 Fetching Pacifica funding rates...");
    let pacifica_creds = PacificaCredentials::from_env()?;
    let mut pacifica_client = PacificaTrading::new(pacifica_creds);
    let pacifica_rates = pacifica_client.get_all_funding_rates().await?;
    println!("   ✓ Retrieved {} markets from Pacifica", pacifica_rates.len());

    // Create lookup maps
    let mut extended_map: HashMap<String, &FundingRateInfo> = HashMap::new();
    for rate in &extended_rates {
        if let Some(symbol) = map_extended_to_pacifica(&rate.market) {
            extended_map.insert(symbol, rate);
        }
    }

    let mut pacifica_map: HashMap<String, &PacificaFundingRate> = HashMap::new();
    for rate in &pacifica_rates {
        pacifica_map.insert(rate.symbol.clone(), rate);
    }

    // Find common symbols and calculate arbitrage opportunities
    let mut opportunities: Vec<ArbitrageOpportunity> = Vec::new();

    for (symbol, pacifica_rate) in &pacifica_map {
        if let Some(extended_rate) = extended_map.get(symbol) {
            // Use reference rates (next for Pacifica, current for Extended)
            let pac_rate = pacifica_rate.reference_rate();
            let ext_rate = extended_rate.reference_rate();

            let opportunity = ArbitrageOpportunity::new(
                symbol.clone(),
                pac_rate,
                ext_rate,
            );

            opportunities.push(opportunity);
        }
    }

    println!("   ✓ Found {} common markets\n", opportunities.len());

    if opportunities.is_empty() {
        println!("No common markets found between exchanges.");
        return Ok(());
    }

    // Sort by net funding rate (highest profit first)
    opportunities.sort_by(|a, b| {
        b.net_funding_rate
            .partial_cmp(&a.net_funding_rate)
            .unwrap()
    });

    // Display top opportunities
    display_opportunities(&opportunities, 20);

    // Display summary statistics
    display_summary(&opportunities);

    Ok(())
}

fn display_opportunities(opportunities: &[ArbitrageOpportunity], limit: usize) {
    let mut table = Table::new();

    // Header
    table.add_row(Row::new(vec![
        Cell::new("Rank")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
        Cell::new("Symbol")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
        Cell::new("Strategy")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
        Cell::new("Net Funding")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
        Cell::new("APR")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
        Cell::new("Pay (Long)")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
        Cell::new("Receive (Short)")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
    ]));

    // Display up to 'limit' opportunities
    let display_count = opportunities.len().min(limit);

    for (idx, opp) in opportunities.iter().take(display_count).enumerate() {
        let rank = idx + 1;
        let is_profitable = opp.net_funding_rate > 0.0;

        // Strategy description
        let strategy = format!(
            "Long {} / Short {}",
            if opp.long_exchange == "Pacifica" { "PAC" } else { "EXT" },
            if opp.short_exchange == "Pacifica" { "PAC" } else { "EXT" }
        );

        // Color coding based on profitability
        let net_funding_cell = if is_profitable {
            Cell::new(&format!("{:>8.6}%", opp.net_funding_rate))
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(color::GREEN))
        } else {
            Cell::new(&format!("{:>8.6}%", opp.net_funding_rate))
                .with_style(Attr::ForegroundColor(color::RED))
        };

        let apr_cell = if is_profitable {
            Cell::new(&format!("{:>8.2}%", opp.apr))
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(color::GREEN))
        } else {
            Cell::new(&format!("{:>8.2}%", opp.apr))
                .with_style(Attr::ForegroundColor(color::RED))
        };

        let rank_cell = if rank <= 3 {
            Cell::new(&format!("#{}", rank))
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(color::YELLOW))
        } else {
            Cell::new(&format!("#{}", rank))
        };

        table.add_row(Row::new(vec![
            rank_cell,
            Cell::new(&opp.symbol).with_style(Attr::Bold),
            Cell::new(&strategy),
            net_funding_cell,
            apr_cell,
            Cell::new(&format!("{:.6}% ({})", opp.long_rate, &opp.long_exchange[..3])),
            Cell::new(&format!("{:.6}% ({})", opp.short_rate, &opp.short_exchange[..3])),
        ]));
    }

    println!("┌────────────────────────────────────────────────────────────────────────────────┐");
    println!("│                        TOP ARBITRAGE OPPORTUNITIES                             │");
    println!("└────────────────────────────────────────────────────────────────────────────────┘\n");

    table.printstd();
    println!();
}

fn display_summary(opportunities: &[ArbitrageOpportunity]) {
    let total = opportunities.len();
    let profitable = opportunities.iter().filter(|o| o.net_funding_rate > 0.0).count();
    let unprofitable = total - profitable;

    let best = opportunities.first().unwrap();
    let worst = opportunities.last().unwrap();

    let avg_net_funding: f64 = opportunities.iter().map(|o| o.net_funding_rate).sum::<f64>() / total as f64;
    let avg_apr: f64 = opportunities.iter().map(|o| o.apr).sum::<f64>() / total as f64;

    // Count strategies
    let pac_long_count = opportunities.iter().filter(|o| o.long_exchange == "Pacifica").count();
    let ext_long_count = total - pac_long_count;

    println!("┌──────────────────────────────────────────────────────────────────────────┐");
    println!("│                         ARBITRAGE SUMMARY                                │");
    println!("├──────────────────────────────────────────────────────────────────────────┤");
    println!("│  Total Markets Analyzed:     {:>3}                                       │", total);
    println!("│  Profitable Opportunities:   {:>3} ({:.1}%)                            │",
             profitable, (profitable as f64 / total as f64) * 100.0);
    println!("│  Unprofitable:               {:>3} ({:.1}%)                            │",
             unprofitable, (unprofitable as f64 / total as f64) * 100.0);
    println!("│                                                                          │");
    println!("│  Average Net Funding:        {:>8.6}%                                │", avg_net_funding);
    println!("│  Average APR:                {:>8.2}%                                  │", avg_apr);
    println!("│                                                                          │");
    println!("│  Best Opportunity:           {} ({:.6}% net, {:.2}% APR)        │",
             best.symbol, best.net_funding_rate, best.apr);
    println!("│  Strategy:                   Long {} / Short {}                    │",
             best.long_exchange, best.short_exchange);
    println!("│                                                                          │");
    println!("│  Worst Opportunity:          {} ({:.6}% net, {:.2}% APR)       │",
             worst.symbol, worst.net_funding_rate, worst.apr);
    println!("│                                                                          │");
    println!("│  Strategy Distribution:                                                  │");
    println!("│    Long Pacifica / Short Extended:  {:>3} markets                        │", pac_long_count);
    println!("│    Long Extended / Short Pacifica:  {:>3} markets                        │", ext_long_count);
    println!("└──────────────────────────────────────────────────────────────────────────┘\n");

    println!("💡 Key Insights:");
    println!("   • Pacifica uses NEXT funding rate (predicted for next settlement)");
    println!("   • Extended uses CURRENT funding rate");
    println!("   • Positive net funding = profitable arbitrage opportunity");
    println!("   • Strategy shows where to go LONG vs SHORT for maximum profit");
    println!("   • APR assumes rates remain constant (for reference only)");

    if profitable > unprofitable {
        println!("\n✅ Market Condition: FAVORABLE for funding rate arbitrage");
        println!("   Significant rate differences exist between exchanges");
    } else {
        println!("\n⚠️  Market Condition: LIMITED arbitrage opportunities");
        println!("   Funding rates are relatively aligned between exchanges");
    }

    if avg_net_funding > 0.01 {
        println!("\n🎯 Action: Consider deploying capital to top opportunities");
    } else if avg_net_funding < -0.01 {
        println!("\n⚠️  Action: Avoid arbitrage positions, rates are unfavorable");
    } else {
        println!("\n📊 Action: Monitor for better opportunities");
    }
}
