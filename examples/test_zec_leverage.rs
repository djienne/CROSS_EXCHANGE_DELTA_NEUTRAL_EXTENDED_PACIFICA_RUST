use extended_connector::{init_logging, PacificaCredentials, PacificaTrading, RestClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║            TEST: SET LEVERAGE TO 1X FOR ZEC                      ║");
    println!("║                Extended DEX & Pacifica                           ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Load API keys from environment
    dotenv::dotenv().ok();
    let extended_api_key = std::env::var("EXTENDED_API_KEY")
        .or_else(|_| std::env::var("API_KEY"))
        .ok();

    // ═══════════════════════════════════════════════════
    // EXTENDED DEX - ZEC-USD
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                       EXTENDED DEX                               ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    if let Some(ref api_key) = extended_api_key {
        let client = RestClient::new_mainnet(Some(api_key.clone()))?;
        println!("📊 Setting leverage to 1X for ZEC-USD on Extended DEX...\n");

        print!("   ZEC-USD ... ");
        match client.update_leverage("ZEC-USD", "1").await {
            Ok(leverage) => {
                println!("✅ Set to {}x", leverage);
            }
            Err(e) => {
                println!("❌ Failed: {}", e);
            }
        }
        println!();
    } else {
        println!("⚠️  No API key found for Extended DEX.");
        println!("   Set EXTENDED_API_KEY or API_KEY in .env file.\n");
    }

    // ═══════════════════════════════════════════════════
    // PACIFICA - ZEC
    // ═══════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                         PACIFICA                                 ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    match PacificaCredentials::from_env() {
        Ok(credentials) => {
            let client = PacificaTrading::new(credentials);
            println!("📊 Setting leverage to 1X for ZEC on Pacifica...\n");

            print!("   ZEC ... ");
            match client.update_leverage("ZEC", 1).await {
                Ok(()) => {
                    println!("✅ Set to 1x");
                }
                Err(e) => {
                    println!("❌ Failed: {}", e);
                }
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

    println!("✅ ZEC leverage test completed.\n");
    println!("🔧 Next steps:");
    println!("   1. Verify on Extended: cargo run --example check_positions");
    println!("   2. Verify on Pacifica: cargo run --example pacifica_check_positions\n");

    Ok(())
}
