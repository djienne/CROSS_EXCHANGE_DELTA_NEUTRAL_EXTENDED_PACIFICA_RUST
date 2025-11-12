use extended_connector::{init_logging, PacificaCredentials, PacificaTrading};
use prettytable::{color, Attr, Cell, Row, Table};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_logging();

    // Load credentials from environment
    let credentials = PacificaCredentials::from_env()?;

    // Create Pacifica trading client
    println!("Fetching funding rates for all markets on Pacifica...\n");
    let mut client = PacificaTrading::new(credentials);

    // Get all funding rates
    let funding_rates = client.get_all_funding_rates().await?;

    if funding_rates.is_empty() {
        println!("No funding rates available.");
        return Ok(());
    }

    // Create table
    let mut table = Table::new();

    // Add header with styling
    table.add_row(Row::new(vec![
        Cell::new("Symbol")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
        Cell::new("Current Rate (%)")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
        Cell::new("Next Rate (%)")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
        Cell::new("Hourly APR (%)")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
        Cell::new("Status")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_CYAN)),
    ]));

    // Add separator
    table.add_row(Row::new(vec![
        Cell::new("──────────"),
        Cell::new("──────────────────"),
        Cell::new("────────────────"),
        Cell::new("──────────────────"),
        Cell::new("────────"),
    ]));

    // Add data rows
    for rate in &funding_rates {
        let is_positive = rate.rate_percentage >= 0.0;

        // Calculate estimated APR (hourly * 24 * 365)
        let apr = rate.rate_percentage * 24.0 * 365.0;

        let current_rate_cell = if is_positive {
            Cell::new(&format!("{:>10.6}", rate.rate_percentage))
                .with_style(Attr::ForegroundColor(color::GREEN))
        } else {
            Cell::new(&format!("{:>10.6}", rate.rate_percentage))
                .with_style(Attr::ForegroundColor(color::RED))
        };

        let next_rate_cell = if rate.next_rate_percentage >= 0.0 {
            Cell::new(&format!("{:>10.6}", rate.next_rate_percentage))
                .with_style(Attr::ForegroundColor(color::GREEN))
        } else {
            Cell::new(&format!("{:>10.6}", rate.next_rate_percentage))
                .with_style(Attr::ForegroundColor(color::RED))
        };

        let apr_cell = if is_positive {
            Cell::new(&format!("{:>10.2}", apr))
                .with_style(Attr::ForegroundColor(color::GREEN))
        } else {
            Cell::new(&format!("{:>10.2}", apr))
                .with_style(Attr::ForegroundColor(color::RED))
        };

        let status_cell = if is_positive {
            Cell::new("  +").with_style(Attr::ForegroundColor(color::GREEN))
        } else {
            Cell::new("  -").with_style(Attr::ForegroundColor(color::RED))
        };

        table.add_row(Row::new(vec![
            Cell::new(&rate.symbol),
            current_rate_cell,
            next_rate_cell,
            apr_cell,
            status_cell,
        ]));
    }

    // Print table
    table.printstd();

    // Print summary statistics
    print_summary(&funding_rates);

    Ok(())
}

fn print_summary(rates: &[extended_connector::PacificaFundingRate]) {
    if rates.is_empty() {
        return;
    }

    let total = rates.len();
    let avg_rate = rates.iter().map(|r| r.rate_percentage).sum::<f64>() / total as f64;
    let avg_apr = avg_rate * 24.0 * 365.0; // Convert hourly to APR

    let max_rate = rates
        .iter()
        .max_by(|a, b| a.rate_percentage.partial_cmp(&b.rate_percentage).unwrap())
        .unwrap();

    let min_rate = rates
        .iter()
        .min_by(|a, b| a.rate_percentage.partial_cmp(&b.rate_percentage).unwrap())
        .unwrap();

    let positive_count = rates.iter().filter(|r| r.rate_percentage >= 0.0).count();
    let negative_count = total - positive_count;

    println!("\n┌──────────────────────────────────────────────────────────────┐");
    println!("│                      SUMMARY STATISTICS                      │");
    println!("├──────────────────────────────────────────────────────────────┤");
    println!("│  Total Markets:        {:>3}                                 │", total);
    println!("│  Average Rate:         {:>8.6}%                         │", avg_rate);
    println!("│  Average APR:          {:>8.2}%                           │", avg_apr);
    println!("│  Highest Rate:         {:>8.6}%  ({})                 │",
             max_rate.rate_percentage,
             max_rate.symbol);
    println!("│  Lowest Rate:          {:>8.6}%  ({})                │",
             min_rate.rate_percentage,
             min_rate.symbol);
    println!("│                                                              │");
    println!("│  Positive Rates:       {:>3} markets                         │", positive_count);
    println!("│  Negative Rates:       {:>3} markets                         │", negative_count);
    println!("└──────────────────────────────────────────────────────────────┘\n");

    // Additional insights
    println!("📊 Market Insights:");
    println!("   • Pacifica uses hourly funding rate settlements");
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
