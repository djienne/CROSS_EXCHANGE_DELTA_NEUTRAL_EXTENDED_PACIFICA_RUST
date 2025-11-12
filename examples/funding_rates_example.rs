use extended_connector::{init_logging, FundingRateInfo, RestClient};
use prettytable::{color, Attr, Cell, Row, Table};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_logging();

    // Load environment variables (optional)
    dotenv::dotenv().ok();
    let api_key = std::env::var("EXTENDED_API_KEY").ok();

    // Create REST client for mainnet
    println!("Fetching funding rates for all markets on Extended mainnet...\n");
    let client = RestClient::new_mainnet(api_key)?;

    // Get all funding rates
    let mut funding_rates = client.get_all_funding_rates().await?;

    if funding_rates.is_empty() {
        println!("No funding rates available.");
        return Ok(());
    }

    // Sort by APR (highest first)
    funding_rates.sort_by(|a, b| b.calculate_apr().partial_cmp(&a.calculate_apr()).unwrap());

    // Create table
    let mut table = Table::new();

    // Add header with styling
    table.add_row(Row::new(vec![
        Cell::new("Market")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
        Cell::new("Funding Rate (%)")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
        Cell::new("APR (%)")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
        Cell::new("Next Funding Time")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
        Cell::new("Status")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
    ]));

    // Add separator
    table.add_row(Row::new(vec![
        Cell::new("─────────"),
        Cell::new("─────────────────"),
        Cell::new("─────────────"),
        Cell::new("───────────────────────"),
        Cell::new("────────"),
    ]));

    // Add data rows
    for rate in &funding_rates {
        let rate_cell = if rate.is_positive {
            Cell::new(&format!("{:>8.4}", rate.rate_percentage))
                .with_style(Attr::ForegroundColor(color::GREEN))
        } else {
            Cell::new(&format!("{:>8.4}", rate.rate_percentage))
                .with_style(Attr::ForegroundColor(color::RED))
        };

        let apr_cell = if rate.is_positive {
            Cell::new(&format!("{:>8.2}", rate.apr_percentage()))
                .with_style(Attr::ForegroundColor(color::GREEN))
        } else {
            Cell::new(&format!("{:>8.2}", rate.apr_percentage()))
                .with_style(Attr::ForegroundColor(color::RED))
        };

        let status_cell = if rate.is_positive {
            Cell::new("  +").with_style(Attr::ForegroundColor(color::GREEN))
        } else {
            Cell::new("  -").with_style(Attr::ForegroundColor(color::RED))
        };

        table.add_row(Row::new(vec![
            Cell::new(&rate.market),
            rate_cell,
            apr_cell,
            Cell::new(&rate.format_timestamp()),
            status_cell,
        ]));
    }

    // Print table
    table.printstd();

    // Print summary statistics
    print_summary(&funding_rates);

    Ok(())
}

fn print_summary(rates: &[FundingRateInfo]) {
    if rates.is_empty() {
        return;
    }

    let total = rates.len();
    let avg_rate = rates.iter().map(|r| r.rate_percentage).sum::<f64>() / total as f64;
    let avg_apr = rates.iter().map(|r| r.apr_percentage()).sum::<f64>() / total as f64;

    let max_rate = rates
        .iter()
        .max_by(|a, b| a.apr_percentage().partial_cmp(&b.apr_percentage()).unwrap())
        .unwrap();

    let min_rate = rates
        .iter()
        .min_by(|a, b| a.apr_percentage().partial_cmp(&b.apr_percentage()).unwrap())
        .unwrap();

    let positive_count = rates.iter().filter(|r| r.is_positive).count();
    let negative_count = total - positive_count;

    println!("\n┌──────────────────────────────────────────────────────────────┐");
    println!("│                      SUMMARY STATISTICS                      │");
    println!("├──────────────────────────────────────────────────────────────┤");
    println!("│  Total Markets:        {:>3}                                 │", total);
    println!("│  Average Rate:         {:>8.4}%                           │", avg_rate);
    println!("│  Average APR:          {:>8.2}%                           │", avg_apr);
    println!("│  Highest APR:          {:>8.2}%  ({})                 │",
             max_rate.apr_percentage(),
             max_rate.market);
    println!("│  Lowest APR:           {:>8.2}%  ({})                │",
             min_rate.apr_percentage(),
             min_rate.market);
    println!("│                                                              │");
    println!("│  Positive Rates:       {:>3} markets                         │", positive_count);
    println!("│  Negative Rates:       {:>3} markets                         │", negative_count);
    println!("└──────────────────────────────────────────────────────────────┘\n");

    // Additional insights
    println!("📊 Market Insights:");
    if positive_count > negative_count {
        println!("   • Majority of markets have positive funding rates");
        println!("   • Longs are paying shorts (bearish sentiment)");
    } else if negative_count > positive_count {
        println!("   • Majority of markets have negative funding rates");
        println!("   • Shorts are paying longs (bullish sentiment)");
    } else {
        println!("   • Markets are evenly split between positive and negative rates");
        println!("   • Neutral market sentiment");
    }

    if avg_apr.abs() > 10.0 {
        println!("   • Average APR is {}significant ({:.2}%)",
                 if avg_apr.abs() > 50.0 { "very " } else { "" },
                 avg_apr);
    } else {
        println!("   • Average APR is relatively low ({:.2}%)", avg_apr);
    }
}
