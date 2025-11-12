use serde::{Deserialize, Serialize};
use std::fmt;

/// Websocket subscription message
#[derive(Debug, Serialize)]
pub struct SubscribeMessage {
    pub method: String,
    pub params: SubscribeParams,
}

/// Subscription parameters for orderbook
#[derive(Debug, Serialize)]
pub struct SubscribeParams {
    pub source: String,
    pub symbol: String,
    pub agg_level: u32,
}

/// Unsubscription message
#[derive(Debug, Serialize)]
pub struct UnsubscribeMessage {
    pub method: String,
    pub params: SubscribeParams,
}

/// Ping message for keepalive
#[derive(Debug, Serialize)]
pub struct PingMessage {
    pub method: String,
}

/// Generic websocket response
#[derive(Debug, Deserialize)]
pub struct WebSocketResponse {
    pub channel: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Orderbook stream response
#[derive(Debug, Deserialize)]
pub struct OrderbookResponse {
    pub channel: String,
    pub data: OrderbookData,
}

/// Orderbook data structure
#[derive(Debug, Clone, Deserialize)]
pub struct OrderbookData {
    #[serde(rename = "l")]
    pub levels: Vec<Vec<BookLevel>>, // [bids, asks]
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "t")]
    pub timestamp: u64,
}

/// Book level with aggregated orders
#[derive(Debug, Clone, Deserialize)]
pub struct BookLevel {
    #[serde(rename = "a")]
    pub amount: String, // Total amount in aggregation level
    #[serde(rename = "n")]
    pub num_orders: u32, // Number of orders in aggregation level
    #[serde(rename = "p")]
    pub price: String, // Price (highest for bids, lowest for asks)
}

/// Top of book data (best bid and ask)
#[derive(Debug, Clone)]
pub struct TopOfBook {
    pub symbol: String,
    pub best_bid: Option<BookLevel>,
    pub best_ask: Option<BookLevel>,
    pub timestamp: u64,
}

impl OrderbookData {
    /// Extract the top of book (best bid and ask)
    pub fn get_top_of_book(&self) -> TopOfBook {
        let best_bid = self.levels.get(0)
            .and_then(|bids| bids.first())
            .cloned();

        let best_ask = self.levels.get(1)
            .and_then(|asks| asks.first())
            .cloned();

        TopOfBook {
            symbol: self.symbol.clone(),
            best_bid,
            best_ask,
            timestamp: self.timestamp,
        }
    }
}

impl SubscribeMessage {
    pub fn new(symbol: String, agg_level: u32) -> Self {
        Self {
            method: "subscribe".to_string(),
            params: SubscribeParams {
                source: "book".to_string(),
                symbol,
                agg_level,
            },
        }
    }
}

impl UnsubscribeMessage {
    pub fn new(symbol: String, agg_level: u32) -> Self {
        Self {
            method: "unsubscribe".to_string(),
            params: SubscribeParams {
                source: "book".to_string(),
                symbol,
                agg_level,
            },
        }
    }
}

impl PingMessage {
    pub fn new() -> Self {
        Self {
            method: "ping".to_string(),
        }
    }
}

impl Default for PingMessage {
    fn default() -> Self {
        Self::new()
    }
}

/// Account order updates subscription parameters
#[derive(Debug, Serialize)]
pub struct AccountOrderUpdatesParams {
    pub source: String,
    pub account: String,
}

/// Account order updates subscription message
#[derive(Debug, Serialize)]
pub struct AccountOrderUpdatesSubscribe {
    pub method: String,
    pub params: AccountOrderUpdatesParams,
}

impl AccountOrderUpdatesSubscribe {
    pub fn new(account: String) -> Self {
        Self {
            method: "subscribe".to_string(),
            params: AccountOrderUpdatesParams {
                source: "account_order_updates".to_string(),
                account,
            },
        }
    }
}

/// Order event type
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OrderEvent {
    Make,
    StopCreated,
    FulfillMarket,
    FulfillLimit,
    Adjust,
    StopParentOrderFilled,
    StopTriggered,
    StopUpgrade,
    Cancel,
    ForceCancel,
    Expired,
    PostOnlyRejected,
    SelfTradePrevented,
}

/// Order status
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

/// Order update data
#[derive(Debug, Clone, Deserialize)]
pub struct OrderUpdate {
    #[serde(rename = "i")]
    pub order_id: u64,
    #[serde(rename = "I")]
    pub client_order_id: Option<String>,
    #[serde(rename = "u")]
    pub account: String,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "d")]
    pub side: String, // "bid" or "ask"
    #[serde(rename = "p")]
    pub avg_filled_price: String,
    #[serde(rename = "ip")]
    pub initial_price: String,
    #[serde(rename = "a")]
    pub original_amount: String,
    #[serde(rename = "f")]
    pub filled_amount: String,
    #[serde(rename = "oe")]
    pub order_event: OrderEvent,
    #[serde(rename = "os")]
    pub order_status: OrderStatus,
    #[serde(rename = "ot")]
    pub order_type: String, // "limit" or "market"
    #[serde(rename = "sp")]
    pub stop_price: Option<String>,
    #[serde(rename = "si")]
    pub stop_parent_order_id: Option<String>,
    #[serde(rename = "r")]
    pub reduce_only: bool,
    #[serde(rename = "ut")]
    pub updated_at: u64, // milliseconds
    #[serde(rename = "ct")]
    pub created_at: u64, // milliseconds
}

/// Account order updates response
#[derive(Debug, Deserialize)]
pub struct AccountOrderUpdatesResponse {
    pub channel: String,
    pub data: Vec<OrderUpdate>,
}

/// Fill information extracted from order update
#[derive(Debug, Clone)]
pub enum FillEvent {
    PartialFill {
        order_id: u64,
        client_order_id: Option<String>,
        symbol: String,
        side: String,
        filled_amount: String,
        original_amount: String,
        avg_price: String,
        timestamp: u64,
    },
    FullFill {
        order_id: u64,
        client_order_id: Option<String>,
        symbol: String,
        side: String,
        filled_amount: String,
        avg_price: String,
        timestamp: u64,
    },
    Cancelled {
        order_id: u64,
        client_order_id: Option<String>,
        symbol: String,
        side: String,
        filled_amount: String,
        original_amount: String,
        reason: String,
        timestamp: u64,
    },
}

impl OrderUpdate {
    /// Convert order update to fill event if applicable
    pub fn to_fill_event(&self) -> Option<FillEvent> {
        match self.order_status {
            OrderStatus::PartiallyFilled => Some(FillEvent::PartialFill {
                order_id: self.order_id,
                client_order_id: self.client_order_id.clone(),
                symbol: self.symbol.clone(),
                side: self.side.clone(),
                filled_amount: self.filled_amount.clone(),
                original_amount: self.original_amount.clone(),
                avg_price: self.avg_filled_price.clone(),
                timestamp: self.updated_at,
            }),
            OrderStatus::Filled => Some(FillEvent::FullFill {
                order_id: self.order_id,
                client_order_id: self.client_order_id.clone(),
                symbol: self.symbol.clone(),
                side: self.side.clone(),
                filled_amount: self.filled_amount.clone(),
                avg_price: self.avg_filled_price.clone(),
                timestamp: self.updated_at,
            }),
            OrderStatus::Cancelled => {
                let reason = match self.order_event {
                    OrderEvent::Cancel => "user_cancelled",
                    OrderEvent::ForceCancel => "force_cancelled",
                    OrderEvent::Expired => "expired",
                    OrderEvent::PostOnlyRejected => "post_only_rejected",
                    OrderEvent::SelfTradePrevented => "self_trade_prevented",
                    _ => "unknown",
                };
                Some(FillEvent::Cancelled {
                    order_id: self.order_id,
                    client_order_id: self.client_order_id.clone(),
                    symbol: self.symbol.clone(),
                    side: self.side.clone(),
                    filled_amount: self.filled_amount.clone(),
                    original_amount: self.original_amount.clone(),
                    reason: reason.to_string(),
                    timestamp: self.updated_at,
                })
            },
            _ => None,
        }
    }
}

// ═══════════════════════════════════════════════════
// WebSocket Trading Operations
// ═══════════════════════════════════════════════════

/// WebSocket cancel all orders request
#[derive(Debug, Serialize)]
pub struct WsCancelAllOrdersRequest {
    /// Request ID (UUID)
    pub id: String,
    /// Request parameters
    pub params: WsCancelAllOrdersParams,
}

/// Parameters for cancel all orders
#[derive(Debug, Serialize)]
pub struct WsCancelAllOrdersParams {
    pub cancel_all_orders: WsCancelAllOrdersData,
}

/// Cancel all orders data payload
#[derive(Debug, Serialize)]
pub struct WsCancelAllOrdersData {
    /// User's wallet address
    pub account: String,
    /// Agent wallet address (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_wallet: Option<String>,
    /// Cryptographic signature
    pub signature: String,
    /// Current timestamp in milliseconds
    pub timestamp: i64,
    /// Signature expiry in milliseconds
    pub expiry_window: i64,
    /// Whether to cancel orders for all symbols
    pub all_symbols: bool,
    /// Whether to exclude reduce-only orders
    pub exclude_reduce_only: bool,
    /// Trading pair symbol (required if all_symbols is false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

/// WebSocket cancel all orders response
#[derive(Debug, Deserialize)]
pub struct WsCancelAllOrdersResponse {
    /// Status code
    pub code: u16,
    /// Response data
    pub data: WsCancelAllOrdersResponseData,
    /// Request ID (matches request)
    pub id: String,
    /// Response timestamp
    pub t: i64,
    /// Response type
    #[serde(rename = "type")]
    pub response_type: String,
}

/// Cancel all orders response data
#[derive(Debug, Deserialize)]
pub struct WsCancelAllOrdersResponseData {
    /// Number of orders successfully cancelled
    pub cancelled_count: u32,
}

/// Generic WebSocket trading error response
#[derive(Debug, Deserialize)]
pub struct WsErrorResponse {
    pub code: u16,
    pub error: Option<String>,
    pub id: String,
    pub t: i64,
    #[serde(rename = "type")]
    pub response_type: String,
}

// ═══════════════════════════════════════════════════
// Funding Rates
// ═══════════════════════════════════════════════════

/// Funding rate information for a market
#[derive(Debug, Clone)]
pub struct PacificaFundingRate {
    /// Market symbol (e.g., "BTC", "ETH")
    pub symbol: String,
    /// Current funding rate (decimal string)
    pub funding_rate: String,
    /// Next predicted funding rate (decimal string)
    pub next_funding_rate: String,
    /// Funding rate as percentage
    pub rate_percentage: f64,
    /// Next funding rate as percentage
    pub next_rate_percentage: f64,
}

impl PacificaFundingRate {
    /// Create from market info data
    pub fn from_market_info(symbol: String, funding_rate: String, next_funding_rate: String) -> Self {
        let rate_pct = funding_rate.parse::<f64>().unwrap_or(0.0) * 100.0;
        let next_rate_pct = next_funding_rate.parse::<f64>().unwrap_or(0.0) * 100.0;

        Self {
            symbol,
            funding_rate,
            next_funding_rate,
            rate_percentage: rate_pct,
            next_rate_percentage: next_rate_pct,
        }
    }

    /// Get the reference funding rate (next rate for Pacifica)
    /// This is the rate that will be used for the next settlement
    pub fn reference_rate(&self) -> f64 {
        self.next_rate_percentage
    }

    /// Get the reference funding rate as decimal (not percentage)
    pub fn reference_rate_decimal(&self) -> f64 {
        self.next_funding_rate.parse::<f64>().unwrap_or(0.0)
    }
}

impl fmt::Display for PacificaFundingRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {:.6}% (next: {:.6}%)",
            self.symbol,
            self.rate_percentage,
            self.next_rate_percentage
        )
    }
}

/// Market information with funding rates from /api/v1/info
#[derive(Debug, Clone, Deserialize)]
pub struct PacificaMarketInfo {
    pub symbol: String,
    pub tick_size: String,
    #[serde(default)]
    pub min_tick: Option<String>,
    #[serde(default)]
    pub max_tick: Option<String>,
    pub lot_size: String,
    #[serde(default)]
    pub max_leverage: Option<i32>,
    #[serde(default)]
    pub isolated_only: Option<bool>,
    #[serde(default)]
    pub min_order_size: Option<String>,
    #[serde(default)]
    pub max_order_size: Option<String>,
    pub funding_rate: String,
    pub next_funding_rate: String,
}

// ═══════════════════════════════════════════════════
// Positions
// ═══════════════════════════════════════════════════

/// Position information from /api/v1/positions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacificaPosition {
    pub symbol: String,
    pub side: String,  // "bid" for long, "ask" for short
    pub amount: String,
    pub entry_price: String,
    #[serde(default)]
    pub margin: String,
    pub funding: String,
    pub isolated: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl PacificaPosition {
    /// Check if position is long
    pub fn is_long(&self) -> bool {
        self.side == "bid"
    }

    /// Check if position is short
    pub fn is_short(&self) -> bool {
        self.side == "ask"
    }

    /// Get position size as float
    pub fn size(&self) -> f64 {
        self.amount.parse().unwrap_or(0.0)
    }

    /// Get entry price as float
    pub fn entry(&self) -> f64 {
        self.entry_price.parse().unwrap_or(0.0)
    }
}

impl fmt::Display for PacificaPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let side_str = if self.is_long() { "LONG" } else { "SHORT" };
        write!(
            f,
            "{} {} {} @ ${} (funding: ${})",
            self.symbol,
            side_str,
            self.amount,
            self.entry_price,
            self.funding
        )
    }
}

/// Pacifica account balance and margin information
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PacificaAccountInfo {
    #[serde(rename = "ae")]
    pub account_equity: String,
    #[serde(rename = "as")]
    pub available_to_spend: String,
    #[serde(rename = "aw")]
    pub available_to_withdraw: String,
    #[serde(rename = "b")]
    pub balance: String,
    #[serde(rename = "mu")]
    pub margin_used: String,
    #[serde(rename = "cm")]
    pub maintenance_margin: String,
    #[serde(rename = "f")]
    pub fee_tier: i64,
    #[serde(rename = "oc")]
    pub orders_count: i64,
    #[serde(rename = "pc")]
    pub positions_count: i64,
    #[serde(rename = "sc")]
    pub stop_orders_count: i64,
    #[serde(rename = "pb", default)]
    pub pending_balance: Option<String>,
    #[serde(rename = "t")]
    pub timestamp: u64,
}

impl PacificaAccountInfo {
    /// Get account equity as f64
    pub fn account_equity_f64(&self) -> f64 {
        self.account_equity.parse().unwrap_or(0.0)
    }

    /// Get available to spend (available capital for trading) as f64
    pub fn available_to_spend_f64(&self) -> f64 {
        self.available_to_spend.parse().unwrap_or(0.0)
    }

    /// Get available to withdraw as f64
    pub fn available_to_withdraw_f64(&self) -> f64 {
        self.available_to_withdraw.parse().unwrap_or(0.0)
    }

    /// Get balance as f64
    pub fn balance_f64(&self) -> f64 {
        self.balance.parse().unwrap_or(0.0)
    }

    /// Get margin used as f64
    pub fn margin_used_f64(&self) -> f64 {
        self.margin_used.parse().unwrap_or(0.0)
    }

    /// Get maintenance margin as f64
    pub fn maintenance_margin_f64(&self) -> f64 {
        self.maintenance_margin.parse().unwrap_or(0.0)
    }

    /// Get pending balance as f64
    pub fn pending_balance_f64(&self) -> f64 {
        self.pending_balance
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0)
    }
}

impl fmt::Display for PacificaAccountInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Equity: ${} | Available: ${} | Balance: ${} | Margin Used: ${} | Fee Tier: {}",
            self.account_equity,
            self.available_to_spend,
            self.balance,
            self.margin_used,
            self.fee_tier
        )
    }
}
