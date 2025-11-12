pub mod error;
pub mod rest;
pub mod signature;
pub mod snip12;
pub mod types;
pub mod websocket;
pub mod pacifica;
pub mod opportunity;
pub mod trading;
pub mod bot;

// Re-export commonly used types
pub use error::{ConnectorError, Result};
pub use rest::RestClient;
pub use types::{Balance, BidAsk, FundingRateInfo, MarketInfo, OrderBook, OrderSide, OrderResponse, Position, PositionSide};
pub use websocket::{MultiMarketSubscriber, WebSocketClient};

// Re-export Pacifica types
pub use pacifica::{
    PacificaTrading, PacificaCredentials, PacificaAccountInfo, PacificaFundingRate, PacificaMarketInfo,
    PacificaPosition, OrderbookClient, OrderbookConfig, FillDetectionClient,
    FillDetectionConfig, PacificaWsTrading, TradeHistoryItem,
};

// Re-export Opportunity types
pub use opportunity::{
    Config as OpportunityConfig, Opportunity, OpportunityFinder, VolumeData,
    ScanResult, FilterStats, FilterConfig, TradingConfig, OpportunityCandidate, FilterResult,
};

// Re-export Trading types
pub use trading::{
    DeltaNeutralPosition, TradingError, calculate_position_size,
    open_delta_neutral_position, close_delta_neutral_position, retry_with_backoff,
};

// Re-export Bot types
pub use bot::{BotState, FundingBot};

/// Initialize logging for the library
pub fn init_logging() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_line_number(true)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_exports() {
        // Just verify that main exports are accessible
        let _ = RestClient::new_mainnet(None);
        let _ = WebSocketClient::new_mainnet(None);
    }
}
