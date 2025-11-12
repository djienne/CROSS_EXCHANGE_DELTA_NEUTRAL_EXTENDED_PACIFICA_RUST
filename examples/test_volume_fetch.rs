// Quick test to debug volume fetching

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test Extended volume fetch
    let url = "https://api.starknet.extended.exchange/api/v1/info/markets/BTC-USD/stats";
    println!("Fetching from: {}", url);

    let response = reqwest::get(url).await?;
    let status = response.status();
    println!("Status: {}", status);

    let text = response.text().await?;
    println!("Raw response:\n{}\n", text);

    // Try parsing
    #[derive(serde::Deserialize, Debug)]
    struct StatsResponse {
        status: Option<String>,
        data: Option<MarketStats>,
    }

    #[derive(serde::Deserialize, Debug)]
    struct MarketStats {
        #[serde(rename = "dailyVolume")]
        daily_volume: String,
    }

    let parsed: Result<StatsResponse, _> = serde_json::from_str(&text);
    match parsed {
        Ok(stats) => {
            println!("Parsed successfully!");
            println!("Stats: {:?}", stats);
            if let Some(data) = stats.data {
                println!("Daily volume string: {}", data.daily_volume);
                println!("Daily volume as f64: {}", data.daily_volume.parse::<f64>()?);
            }
        }
        Err(e) => {
            println!("Parse error: {}", e);
        }
    }

    // Test Pacifica
    println!("\n---\n");
    let pac_url = "https://api.pacifica.fi/api/v1/info/markets/BTC/stats";
    println!("Fetching from: {}", pac_url);

    let pac_response = reqwest::get(pac_url).await?;
    println!("Status: {}", pac_response.status());
    println!("Response: {}", pac_response.text().await?);

    Ok(())
}
