# Extended DEX Connector & Funding Rate Arbitrage Bot

**An autonomous trading bot for delta neutral funding rate arbitrage between Extended DEX (Starknet) and Pacifica, plus a comprehensive Rust connector library for both exchanges.**

**New to Extended DEX?** [Sign up with my referral link](https://app.extended.exchange/join/FREQTRADE) and receive a **10% discount on commissions** for your first $50M in total trading volume.

## 🤖 Autonomous Trading Bot

The main feature is a **fully automated funding rate arbitrage bot** that:
- 🔍 Scans markets every 15 minutes for best opportunities
- 💰 Opens delta neutral positions (95% of available capital)
- ⏱️ Holds positions for 48 hours to capture funding payments
- 🔄 Automatically rotates to new opportunities
- 💾 Persists state for crash recovery
- 📊 Displays real-time PnL and position status

**Live Example**: Currently running with $99 capital, earning 9.53% APR on ETH funding rate differential.

## Quick Start: Running the Bot

### 1. Setup Credentials

Create `.env` file:
```bash
# Extended DEX
API_KEY=your_extended_api_key
STARK_PRIVATE=0x...
STARK_PUBLIC=0x...
VAULT_NUMBER=your_vault_number

# Pacifica
SOL_WALLET=your_solana_wallet
API_PUBLIC=your_pacifica_public_key
API_PRIVATE=your_pacifica_private_key
```

### 2. Configure Filters

Edit `config.json`:
```json
{
  "filters": {
    "min_combined_volume_usd": 50000000,
    "max_intra_exchange_spread_pct": 0.15,
    "max_cross_exchange_spread_pct": 0.25,
    "min_net_apr_pct": 5.0
  }
}
```

### 3. Run the Bot

```bash
cargo run
```

That's it! The bot will:
1. ✅ Load credentials and configuration
2. 🔍 Scan for best opportunity immediately
3. 📊 Open delta neutral position
4. ⏱️ Monitor every 15 minutes
5. 🔄 Rotate after 48 hours

## Features

### Autonomous Trading Bot
- ✅ **Delta Neutral Arbitrage** - Simultaneous long/short positions across exchanges
- ✅ **Opportunity Scanner** - Real-time scanning with multi-stage filtering
- ✅ **Position Management** - Automated opening, monitoring, and closing
- ✅ **State Persistence** - JSON-based state for crash recovery (`bot_state.json`)
- ✅ **Retry Logic** - 5 attempts with exponential backoff for reliable execution
- ✅ **Monitoring** - 15-minute status updates with PnL tracking
- ✅ **Smart Sizing** - 95% capital allocation with lot size rounding

### Extended DEX (Starknet)
- ✅ **REST API Client** - Orderbooks, markets, positions, account balance
- ✅ **WebSocket Client** - Real-time bid/ask price streams
- ✅ **Order Placement** - Market and limit orders with SNIP-12 signing
- ✅ **Position Management** - Open, close, and monitor positions
- ✅ **Funding Rates** - Latest rates for all markets with formatted display

### Pacifica
- ✅ **Trading Client** - REST API for order placement and management
- ✅ **Funding Rates** - Hourly settlement rates with predictions
- ✅ **WebSocket Trading** - Low-latency trading operations
- ✅ **Orderbook Streaming** - Real-time orderbook updates
- ✅ **Position Management** - Monitor and close positions

### Arbitrage Tools
- ✅ **Opportunity Finder** - Configurable filtering (volume, spreads, APR)
- ✅ **Parallel Fetching** - Concurrent API calls for fast scanning
- ✅ **Strategy Selection** - Automatic long/short direction optimization
- ✅ **APR Calculations** - Net annualized returns after spreads

### General
- ✅ **Type-Safe** - Strongly typed API responses with serde
- ✅ **Async/Await** - Built on tokio for high-performance operations
- ✅ **Error Handling** - Comprehensive error types with recovery
- ✅ **Logging** - Integrated tracing for debugging

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
extended_connector = { path = "." }
tokio = { version = "1.42", features = ["full"] }
```

## Configuration

### config.json

```json
{
  "filters": {
    "min_combined_volume_usd": 50000000,      // $50M minimum 24h volume
    "max_intra_exchange_spread_pct": 0.15,    // 0.15% max spread within exchange
    "max_cross_exchange_spread_pct": 0.25,    // 0.25% max price difference
    "min_net_apr_pct": 5.0                    // 5% minimum net APR
  },
  "display": {
    "max_opportunities_shown": 10,
    "show_filtered_out_count": true
  },
  "performance": {
    "fetch_timeout_seconds": 30,
    "rate_limit_delay_ms": 100
  }
}
```

### Bot Behavior

- **Monitoring Interval**: 15 minutes
- **Position Hold Time**: 48 hours
- **Capital Allocation**: 95% of minimum available
- **Retry Attempts**: 5 with exponential backoff
- **Order Type**: Market orders for reliable execution

## Examples

### Run the Bot

```bash
# Main bot (default)
cargo run

# Or explicitly
cargo run --example funding_bot
```

### Scan Opportunities

```bash
# Find best opportunities without trading
cargo run --example scan_opportunities
```

### Extended DEX Examples

```bash
cargo run --example rest_example
cargo run --example websocket_example
cargo run --example funding_rates_example
cargo run --example check_positions
cargo run --example check_balance
```

### Pacifica Examples

```bash
cargo run --example pacifica_funding_rates
cargo run --example pacifica_check_positions
```

### Analysis Examples

```bash
cargo run --example funding_arbitrage
cargo run --example check_spreads
cargo run --example check_cross_spreads
```

## Library Usage

### REST API - Get Orderbook

```rust
use extended_connector::RestClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::new_mainnet(None)?;
    let orderbook = client.get_orderbook("BTC-USD").await?;

    println!("Top bid: {:?}", orderbook.bid.first());
    println!("Top ask: {:?}", orderbook.ask.first());

    Ok(())
}
```

### WebSocket - Stream Prices

```rust
use extended_connector::WebSocketClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = WebSocketClient::new_mainnet(None);
    let mut rx = client.subscribe_orderbook("BTC-USD").await?;

    while let Some(bid_ask) = rx.recv().await {
        println!("{}", bid_ask);
    }

    Ok(())
}
```

### Opportunity Scanner

```rust
use extended_connector::{OpportunityFinder, OpportunityConfig, PacificaCredentials};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = OpportunityConfig::load("config.json")?;
    let pacifica_creds = PacificaCredentials::from_env()?;

    let finder = OpportunityFinder::new(
        Some("api_key".to_string()),
        pacifica_creds,
        config
    )?;

    let opportunities = finder.scan(Some("api_key".to_string())).await?;

    for opp in opportunities {
        println!("{}: {:.2}% APR", opp.symbol, opp.best_net_apr);
    }

    Ok(())
}
```

### Custom Bot

```rust
use extended_connector::{FundingBot, OpportunityConfig, PacificaCredentials};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let api_key = std::env::var("API_KEY").ok();
    let pacifica_creds = PacificaCredentials::from_env()?;
    let config = OpportunityConfig::load("config.json")?;

    let mut bot = FundingBot::new(
        api_key.clone(),
        pacifica_creds,
        config,
        std::env::var("STARK_PRIVATE")?,
        std::env::var("STARK_PUBLIC")?,
        std::env::var("VAULT_NUMBER")?
    )?;

    bot.run(api_key).await?;
    Ok(())
}
```

## Architecture

```
src/
├── main.rs                # Bot entry point (cargo run)
├── lib.rs                 # Library exports
├── error.rs               # Error types
├── types.rs               # Core data structures
├── rest.rs                # Extended REST API client
├── websocket.rs           # Extended WebSocket client
├── signature.rs           # Order signing utilities
├── opportunity.rs         # Opportunity scanner
├── trading.rs             # Position execution logic
├── bot.rs                 # Bot orchestration & state
├── snip12/                # SNIP-12 signing implementation
└── pacifica/              # Pacifica exchange integration
    ├── client.rs          # Orderbook WebSocket
    ├── trading.rs         # Trading REST API
    ├── ws_trading.rs      # Trading WebSocket
    └── types.rs           # Pacifica types

examples/
├── funding_bot.rs         # Main bot example
├── scan_opportunities.rs  # Opportunity scanner
├── rest_example.rs        # Extended REST examples
├── websocket_example.rs   # Extended WebSocket examples
└── ...                    # More examples

config.json                # Bot configuration
bot_state.json            # Bot state (auto-generated)
```

## API Reference

### Bot Components

**OpportunityFinder**
- `new()` - Create finder with credentials and config
- `scan()` - Scan for opportunities with all filters
- `find_common_symbols()` - Markets available on both exchanges
- `fetch_volumes()` - Get 24h volumes in parallel

**FundingBot**
- `new()` - Create bot with credentials
- `run()` - Start main event loop (runs forever)
- `display_status()` - Show current position status
- `open_best_opportunity()` - Find and open position
- `close_current_position()` - Close active position

**Trading Functions**
- `open_delta_neutral_position()` - Open both legs with retry
- `close_delta_neutral_position()` - Close both positions
- `calculate_position_size()` - Size with lot rounding
- `retry_with_backoff()` - Exponential backoff wrapper

### RestClient

- `new_mainnet(api_key)` - Create client for mainnet
- `get_orderbook(market)` - Fetch full orderbook
- `get_bid_ask(market)` - Get best bid/ask
- `get_all_markets()` - List all markets
- `get_funding_rate(market)` - Get funding rate
- `get_all_funding_rates()` - All funding rates
- `get_positions(market)` - Get open positions
- `get_balance()` - Get account balance
- `place_market_order()` - Place market order
- `close_position()` - Close specific position

### PacificaTrading

- `new(creds)` - Create client with credentials
- `get_market_info()` - Get market specifications
- `get_funding_rate(symbol)` - Get funding rate
- `get_positions()` - Get open positions
- `place_market_order()` - Place market order
- `close_position()` - Close position

## Bot State

The bot maintains state in `bot_state.json`:

```json
{
  "current_position": {
    "symbol": "ETH",
    "extended_position": { "market": "ETH-USD", "size": "0.028", ... },
    "pacifica_position": { "symbol": "ETH", "amount": "0.028", ... },
    "opened_at": 1762449026,
    "target_notional_usd": 92.20
  },
  "last_rotation_time": 1762449026,
  "total_rotations": 1
}
```

This file enables crash recovery - if the bot restarts, it loads the previous state and continues monitoring.

## Testing

```bash
# Run all tests
cargo test

# Run bot in different terminal to monitor
cargo run

# Check bot state
cat bot_state.json

# Scan without trading
cargo run --example scan_opportunities
```

## Supported Markets

Common trading pairs:
- BTC-USD / BTC
- ETH-USD / ETH
- SOL-USD / SOL
- And 20+ more pairs available on both exchanges

## Dependencies

- `tokio` - Async runtime
- `tokio-tungstenite` - WebSocket client
- `reqwest` - HTTP client
- `serde` / `serde_json` - Serialization
- `tracing` - Logging framework
- `dotenv` - Environment variables
- `starknet-crypto` - SNIP-12 signing
- `ed25519-dalek` - Pacifica authentication

## Performance

- **Opportunity Scanning**: ~3-10 seconds for all markets
- **Order Execution**: ~1-2 seconds per exchange
- **Monitoring Interval**: 15 minutes
- **State Persistence**: <100ms

## Documentation

- [Extended API Documentation](https://docs.extended.exchange)
- [Extended Exchange](https://app.extended.exchange)
- [Pacifica API](https://api.pacifica.fi)
- [CLAUDE.md](./CLAUDE.md) - Development guide

## License

MIT

## Contributing

Contributions welcome! Please open an issue or PR.

## Disclaimer

This software is for educational purposes. Use at your own risk. Always test with small amounts first. No guarantees of profitability.
