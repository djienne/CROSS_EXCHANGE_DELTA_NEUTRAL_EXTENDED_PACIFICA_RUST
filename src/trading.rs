/// Delta neutral position execution and management
use crate::{
    types::{OrderSide, Position},
    RestClient, PacificaTrading,
    pacifica::{types::PacificaPosition, trading::OrderSide as PacificaOrderSide},
};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeltaNeutralPosition {
    pub symbol: String,
    pub extended_position: Option<Position>,
    pub pacifica_position: Option<PacificaPosition>,
    pub opened_at: u64,
    pub target_notional_usd: f64,
}

#[derive(Debug)]
pub struct TradingError {
    pub message: String,
    pub recoverable: bool,
}

impl std::fmt::Display for TradingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TradingError: {}", self.message)
    }
}

impl std::error::Error for TradingError {}

impl TradingError {
    pub fn new(message: String, recoverable: bool) -> Self {
        Self { message, recoverable }
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Calculate position size based on available capital and lot size constraints
pub fn calculate_position_size(
    extended_free_collateral: f64,
    pacifica_free_collateral: f64,
    extended_lot_size: f64,
    pacifica_lot_size: f64,
    current_price: f64,
    max_position_size_usd: f64,
) -> f64 {
    // Take 95% of minimum available capital
    let min_capital = extended_free_collateral.min(pacifica_free_collateral);
    let available_notional = min_capital * 0.95;

    // Cap by max_position_size_usd
    let target_notional = available_notional.min(max_position_size_usd);

    // Convert to base currency size
    let base_size = target_notional / current_price;

    // Round to coarser lot_size
    let coarser_lot_size = extended_lot_size.max(pacifica_lot_size);

    // Round down to nearest lot_size
    let rounded_size = (base_size / coarser_lot_size).floor() * coarser_lot_size;

    rounded_size
}

/// Retry function with exponential backoff
pub async fn retry_with_backoff<F, Fut, T>(
    max_attempts: u32,
    operation_name: &str,
    mut operation: F,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 0;

    loop {
        attempt += 1;

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    info!("{} succeeded on attempt {}/{}", operation_name, attempt, max_attempts);
                }
                return Ok(result);
            }
            Err(e) => {
                if attempt >= max_attempts {
                    error!("{} failed after {} attempts: {}", operation_name, max_attempts, e);
                    return Err(e);
                }

                let delay_ms = 2u64.pow(attempt - 1) * 1000; // Exponential backoff: 1s, 2s, 4s, 8s, 16s
                warn!(
                    "{} failed (attempt {}/{}): {}. Retrying in {}ms...",
                    operation_name,
                    attempt,
                    max_attempts,
                    e,
                    delay_ms
                );

                sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }
}

/// Open a delta neutral position across Extended and Pacifica
///
/// Strategy:
/// - Long on exchange with higher funding rate (receiving funding)
/// - Short on exchange with lower funding rate (paying less funding)
///
/// Returns the opened positions if successful
pub async fn open_delta_neutral_position(
    symbol: &str,
    long_on_extended: bool,
    position_size_base: f64,  // Position size in base currency (e.g., BTC)
    current_price: f64,
    extended_client: &RestClient,
    pacifica_client: &mut PacificaTrading,
    extended_market_symbol: &str,  // e.g., "BTC-USD"
    pacifica_market_symbol: &str,   // e.g., "BTC"
    stark_private_key: &str,
    stark_public_key: &str,
    vault_id: &str,
    extended_lot_size: f64,  // Extended size increment (e.g., 0.001)
    pacifica_lot_size: f64,  // Pacifica lot size (e.g., 0.01)
) -> Result<DeltaNeutralPosition> {
    info!("Opening delta neutral position for {}", symbol);
    info!("Strategy: {} Extended / {} Pacifica",
        if long_on_extended { "Long" } else { "Short" },
        if long_on_extended { "Short" } else { "Long" }
    );

    if position_size_base <= 0.0 {
        return Err(Box::new(TradingError::new(
            "Invalid position size".to_string(),
            false,
        )));
    }

    // Round to each exchange's lot size to ensure consistency
    // This prevents rounding discrepancies when Extended recalculates from USD
    let extended_rounded_size = (position_size_base / extended_lot_size).round() * extended_lot_size;
    let pacifica_rounded_size = (position_size_base / pacifica_lot_size).round() * pacifica_lot_size;

    // Calculate notional from Extended's rounded size to ensure Extended gets exact amount after its own rounding
    let notional_usd = extended_rounded_size * current_price;

    info!("Opening position: {:.6} {} (${:.2})", position_size_base, symbol, notional_usd);
    info!("Exchange-specific rounding - Extended: {:.6}, Pacifica: {:.6}",
          extended_rounded_size, pacifica_rounded_size);

    // Step 1: Place first order (Extended)
    let extended_side = if long_on_extended { OrderSide::Buy } else { OrderSide::Sell };
    info!("Placing Extended order: {:?} {:.6} {} @ market", extended_side, extended_rounded_size, symbol);

    let extended_order = extended_client.place_market_order(
        extended_market_symbol,
        extended_side,
        notional_usd,  // Using notional calculated from extended_rounded_size
        stark_private_key,
        stark_public_key,
        vault_id,
        false, // reduce_only = false (opening position)
    ).await?;

    info!("Extended order placed: {:?}", extended_order);

    // Step 2: Place second order with retry (Pacifica)
    let pacifica_side = if long_on_extended { PacificaOrderSide::Sell } else { PacificaOrderSide::Buy };
    let slippage_percent = 0.5; // 0.5% slippage tolerance

    info!("Placing Pacifica order: {:?} {:.6} {} @ market (with {} retries)",
        pacifica_side, pacifica_rounded_size, symbol, 5);

    // Retry logic for Pacifica order (inline due to mutable reference)
    let mut pacifica_order = None;
    for attempt in 1..=5 {
        match pacifica_client.place_market_order(
            pacifica_market_symbol,
            pacifica_side,
            pacifica_rounded_size,  // Using Pacifica's rounded size
            slippage_percent,
            false
        ).await {
            Ok(order) => {
                if attempt > 1 {
                    info!("Pacifica order succeeded on attempt {}/5", attempt);
                }
                pacifica_order = Some(order);
                break;
            }
            Err(e) => {
                if attempt >= 5 {
                    error!("Pacifica order failed after 5 attempts: {}", e);
                    return Err(format!("Failed to place Pacifica order: {}", e).into());
                }
                let delay_ms = 2u64.pow(attempt - 1) * 1000;
                warn!("Pacifica order failed (attempt {}/5): {}. Retrying in {}ms...", attempt, e, delay_ms);
                sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }

    let pacifica_order = pacifica_order.ok_or_else(|| "Pacifica order failed".to_string())?;

    info!("Pacifica order placed: {:?}", pacifica_order);

    // Step 3: Fetch opened positions
    let extended_positions = extended_client.get_positions(None).await?;
    let extended_position = extended_positions.iter()
        .find(|p| p.market == extended_market_symbol)
        .cloned();

    let pacifica_positions = pacifica_client.get_positions().await?;
    let pacifica_position = pacifica_positions.iter()
        .find(|p| p.symbol == pacifica_market_symbol)
        .cloned();

    let opened_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    info!("✅ Delta neutral position opened successfully");

    Ok(DeltaNeutralPosition {
        symbol: symbol.to_string(),
        extended_position,
        pacifica_position,
        opened_at,
        target_notional_usd: notional_usd,
    })
}

/// Close a delta neutral position
pub async fn close_delta_neutral_position(
    position: &DeltaNeutralPosition,
    extended_client: &RestClient,
    pacifica_client: &mut PacificaTrading,
    stark_private_key: &str,
    stark_public_key: &str,
    vault_id: &str,
) -> Result<()> {
    info!("Closing delta neutral position for {}", position.symbol);

    let mut errors = Vec::new();

    // Close Extended position
    if let Some(ref ext_pos) = position.extended_position {
        info!("Closing Extended position: {} {:?}", ext_pos.market, ext_pos.side);

        // Retry logic for closing Extended position (inline due to ownership)
        for attempt in 1..=5 {
            match extended_client.close_position(
                ext_pos,
                stark_private_key,
                stark_public_key,
                vault_id
            ).await {
                Ok(order) => {
                    if attempt > 1 {
                        info!("Close Extended position succeeded on attempt {}/5", attempt);
                    }
                    info!("Extended position closed: {:?}", order);
                    break;
                }
                Err(e) => {
                    if attempt >= 5 {
                        error!("Failed to close Extended position after 5 attempts: {}", e);
                        errors.push(format!("Extended: {}", e));
                    } else {
                        let delay_ms = 2u64.pow(attempt - 1) * 1000;
                        warn!("Close Extended position failed (attempt {}/5): {}. Retrying in {}ms...", attempt, e, delay_ms);
                        sleep(Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }
    }

    // Close Pacifica position
    if let Some(ref pac_pos) = position.pacifica_position {
        info!("Closing Pacifica position: {} (size: {})", pac_pos.symbol, pac_pos.size());

        let slippage_percent = 0.5;

        // Retry logic for closing Pacifica position (inline due to mutable reference)
        for attempt in 1..=5 {
            match pacifica_client.close_position(pac_pos, slippage_percent).await {
                Ok(order) => {
                    if attempt > 1 {
                        info!("Close Pacifica position succeeded on attempt {}/5", attempt);
                    }
                    info!("Pacifica position closed: {:?}", order);
                    break;
                }
                Err(e) => {
                    if attempt >= 5 {
                        error!("Failed to close Pacifica position after 5 attempts: {}", e);
                        errors.push(format!("Pacifica: {}", e));
                    } else {
                        let delay_ms = 2u64.pow(attempt - 1) * 1000;
                        warn!("Close Pacifica position failed (attempt {}/5): {}. Retrying in {}ms...", attempt, e, delay_ms);
                        sleep(Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }
    }

    if !errors.is_empty() {
        return Err(Box::new(TradingError::new(
            format!("Failed to close some positions: {}", errors.join(", ")),
            true,
        )));
    }

    info!("✅ Delta neutral position closed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_position_size() {
        // Test basic calculation
        let extended_capital = 10000.0;
        let pacifica_capital = 12000.0;
        let extended_lot = 0.001;
        let pacifica_lot = 0.01;
        let price = 50000.0;
        let max_position_size = 1000.0;

        let size = calculate_position_size(
            extended_capital,
            pacifica_capital,
            extended_lot,
            pacifica_lot,
            price,
            max_position_size,
        );

        // Min capital = 10000, 95% = 9500, capped at 1000
        // Target = min(9500, 1000) = 1000
        // Base size = 1000 / 50000 = 0.02
        // Coarser lot = 0.01
        // Rounded = 0.02
        assert_eq!(size, 0.02);
    }

    #[test]
    fn test_calculate_position_size_rounds_down() {
        let extended_capital = 10000.0;
        let pacifica_capital = 10000.0;
        let extended_lot = 0.001;
        let pacifica_lot = 0.01;
        let price = 50000.0;
        let max_position_size = 10000.0; // High cap, won't affect result

        let size = calculate_position_size(
            extended_capital,
            pacifica_capital,
            extended_lot,
            pacifica_lot,
            price,
            max_position_size,
        );

        // 95% of 10000 = 9500
        // Base size = 9500 / 50000 = 0.19
        // Coarser lot = 0.01
        // Rounded down = 0.19 (already at lot boundary)
        assert_eq!(size, 0.19);
    }

    #[test]
    fn test_calculate_position_size_insufficient_capital() {
        let extended_capital = 100.0;
        let pacifica_capital = 100.0;
        let extended_lot = 0.01;
        let pacifica_lot = 0.01;
        let price = 50000.0;
        let max_position_size = 1000.0;

        let size = calculate_position_size(
            extended_capital,
            pacifica_capital,
            extended_lot,
            pacifica_lot,
            price,
            max_position_size,
        );

        // 95% of 100 = 95
        // Base size = 95 / 50000 = 0.0019
        // Coarser lot = 0.01
        // Rounded down = 0.0 (insufficient for one lot)
        assert_eq!(size, 0.0);
    }

    #[test]
    fn test_calculate_position_size_capped_by_max() {
        // Test that max_position_size_usd caps the position
        let extended_capital = 100000.0; // $100k available
        let pacifica_capital = 100000.0;
        let extended_lot = 0.001;
        let pacifica_lot = 0.001;
        let price = 50000.0; // $50k per unit
        let max_position_size = 1000.0; // Cap at $1k

        let size = calculate_position_size(
            extended_capital,
            pacifica_capital,
            extended_lot,
            pacifica_lot,
            price,
            max_position_size,
        );

        // 95% of 100k = 95k, but capped at 1k
        // Target notional = 1000
        // Base size = 1000 / 50000 = 0.02
        // Lot size = 0.001
        // Rounded = 0.02
        assert_eq!(size, 0.02);
    }
}
