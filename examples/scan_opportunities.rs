/// Simple example demonstrating the OpportunityFinder module
///
/// This loads config from config.json and scans for arbitrage opportunities
use extended_connector::{
    init_logging, OpportunityConfig, OpportunityFinder, PacificaCredentials,
    opportunity::{format_volume, truncate},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║           ARBITRAGE OPPORTUNITY SCANNER                          ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Load configuration
    let config = OpportunityConfig::load("config.json")?;
    println!("✅ Loaded config from config.json");
    println!("   • Min Volume: ${:.0}M", config.filters.min_combined_volume_usd / 1_000_000.0);
    println!("   • Max Intra Spread: {:.2}%", config.filters.max_intra_exchange_spread_pct);
    println!("   • Max Cross Spread: {:.2}%", config.filters.max_cross_exchange_spread_pct);
    println!("   • Min Net APR: {:.1}%", config.filters.min_net_apr_pct);
    println!("   • Max Position Size: ${:.0}\n", config.trading.max_position_size_usd);

    // Load credentials
    dotenv::dotenv().ok();
    let extended_api_key = std::env::var("EXTENDED_API_KEY")
        .or_else(|_| std::env::var("API_KEY"))
        .ok();
    let pacifica_creds = PacificaCredentials::from_env()?;

    // Create opportunity finder
    let finder = OpportunityFinder::new(extended_api_key.clone(), pacifica_creds, config.clone())?;
    println!("✅ Initialized OpportunityFinder\n");

    // Scan for opportunities
    println!("🔍 Scanning markets...");
    let start_time = std::time::Instant::now();
    let scan_result = finder.scan(extended_api_key).await?;
    let elapsed = start_time.elapsed();

    println!("✅ Scan complete in {:.2}s\n", elapsed.as_secs_f64());

    // Display comprehensive summary table
    scan_result.display_summary(&config.filters);

    // Display results
    if scan_result.opportunities.is_empty() {
        println!("❌ No opportunities found matching criteria\n");
        return Ok(());
    }

    let opportunities = &scan_result.opportunities;

    println!("╔════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                          BEST ARBITRAGE OPPORTUNITIES                                      ║");
    println!("╠════════════════════════════════════════════════════════════════════════════════════════════╣");
    println!("║ Symbol │  Total Vol  │ Ext Sprd │ Pac Sprd │ Cross │ Net APR │      Strategy             ║");
    println!("╠════════════════════════════════════════════════════════════════════════════════════════════╣");

    for opp in opportunities.iter().take(config.display.max_opportunities_shown) {
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

    // Show top 3 in detail
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                      TOP 3 OPPORTUNITIES                         ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    for (idx, opp) in opportunities.iter().take(3).enumerate() {
        println!("{}. {} - {}", idx + 1, opp.symbol, opp.quality_rating());
        println!("   💰 Net APR: {:.2}%", opp.best_net_apr);
        println!("   📊 Strategy: {}", opp.best_direction);
        println!("   📈 Volume: {} (Ext: {}, Pac: {})",
                 format_volume(opp.total_volume_24h),
                 format_volume(opp.extended_volume_24h),
                 format_volume(opp.pacifica_volume_24h));
        println!("   📉 Spreads: Ext {:.3}%, Pac {:.3}%, Cross {:.3}%\n",
                 opp.extended_spread_pct, opp.pacifica_spread_pct, opp.cross_spread_pct);
    }

    println!("⚡ Found {} opportunities in {:.2}s", opportunities.len(), elapsed.as_secs_f64());

    Ok(())
}
