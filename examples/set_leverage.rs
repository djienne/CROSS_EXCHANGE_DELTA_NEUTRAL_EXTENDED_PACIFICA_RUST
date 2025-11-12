use extended_connector::{init_logging, PacificaCredentials, PacificaTrading, RestClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║               SET LEVERAGE TO 1X - BOTH EXCHANGES                ║");
    println!("║            Extended DEX & Pacifica - BTC, ETH, SOL, PUMP         ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Load API keys from environment
    dotenv::dotenv().ok();
    let extended_api_key = std::env::var("EXTENDED_API_KEY")
        .or_else(|_| std::env::var("API_KEY"))
        .ok();

    // Markets to set leverage for
    let extended_markets = vec!["BTC-USD", "ETH-USD", "SOL-USD", "PUMP-USD"];
    let pacifica_symbols = vec!["BTC", "ETH", "SOL", "PUMP"];

    // ═══════════════════════════════════════════════════
    // EXTENDED DEX
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                       EXTENDED DEX                               ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    if let Some(ref api_key) = extended_api_key {
        let client = RestClient::new_mainnet(Some(api_key.clone()))?;
        println!("📊 Setting leverage to 1X for Extended DEX markets...\n");

        for market in &extended_markets {
            print!("   {} ... ", market);
            match client.update_leverage(market, "1").await {
                Ok(leverage) => {
                    println!("✅ Set to {}x", leverage);
                }
                Err(e) => {
                    println!("❌ Failed: {}", e);
                }
            }
        }
        println!();
    } else {
        println!("⚠️  No API key found for Extended DEX.");
        println!("   Set EXTENDED_API_KEY or API_KEY in .env file.\n");
    }

    // ═══════════════════════════════════════════════════
    // PACIFICA
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                         PACIFICA                                 ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    match PacificaCredentials::from_env() {
        Ok(credentials) => {
            let client = PacificaTrading::new(credentials);
            println!("📊 Setting leverage to 1X for Pacifica symbols...\n");

            for symbol in &pacifica_symbols {
                print!("   {} ... ", symbol);
                match client.update_leverage(symbol, 1).await {
                    Ok(()) => {
                        println!("✅ Set to 1x");
                    }
                    Err(e) => {
                        println!("❌ Failed: {}", e);
                    }
                }
                // Small delay to avoid rate limiting
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
            println!();
        }
        Err(e) => {
            println!("⚠️  Pacifica credentials not configured: {}", e);
            println!("   Set SOL_WALLET, API_PUBLIC, API_PRIVATE in .env file.\n");
        }
    }

    // ═══════════════════════════════════════════════════
    // SUMMARY
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                          SUMMARY                                 ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    println!("✅ Leverage setting completed for both exchanges.\n");
    println!("📝 Important Notes:");
    println!("   • Leverage is set per-market (Extended) or per-symbol (Pacifica)");
    println!("   • 1X leverage = no leverage = 1:1 capital to position size");
    println!("   • This is the safest setting for trading");
    println!("   • You can verify leverage with position checks\n");

    println!("🔧 Verification:");
    println!("   Extended: Check positions with cargo run --example check_positions");
    println!("   Pacifica: Check positions with cargo run --example pacifica_check_positions\n");

    Ok(())
}
