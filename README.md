# Extended DEX Connector & Funding Rate Arbitrage Bot

**An autonomous trading bot for delta neutral funding rate arbitrage between Extended DEX (Starknet) and Pacifica, plus a comprehensive Rust connector library for both exchanges.**

---

## Prerequisites

Before installation, ensure you have:

- **Rust 1.70+** - [Install Rust](https://rustup.rs/)
- **Python 3.8+** - Required for order signing
- **pip** - Python package manager
- **Extended DEX Account** - [Sign up here](https://app.extended.exchange/join/FREQTRADE) (10% commission discount on first $50M volume)
- **Pacifica Account** - [Sign up here](https://app.pacifica.fi/) with referral codes:
  ```
  411J9J7CYNFZN3SX  2K7D40A9H53M2TJT  S1G3A2063Q7410BV
  5KH0XDRD8BDGTBCT  S1YHDS2GWAJTWJ4M  7KB69JEC3BSA6GTR
  ```
- **Funded Accounts** - Collateral on both exchanges (USDC recommended)
- **API Keys** - Generated from both exchange dashboards

---

## 🤖 Autonomous Trading Bot

The main feature is a **fully automated funding rate arbitrage bot** that:
- 🔍 Scans markets every 15 minutes for best opportunities
- 💰 Opens delta neutral positions (95% of available capital)
- ⏱️ Holds positions for 48 hours to capture funding payments
- 🔄 Automatically rotates to new opportunities
- 💾 Persists state for crash recovery
- 📊 Displays real-time PnL and position status

**Example**: Bot can run autonomously with small capital amounts to capture funding rate differentials. Results vary based on market conditions.

---

## Installation & Quick Start

### 1. Clone and Setup Repository

```bash
git clone <repository-url>
cd DELTRA_NEUTRAL_EXTENDED_PACIFICA_RUST
```

### 2. Install Python Dependencies

The bot requires Python for Starknet SNIP-12 order signing. Set up a virtual environment (recommended):

```bash
# Create virtual environment
python3 -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install dependencies
pip install -r requirements.txt

# Install Extended DEX Python SDK
cd python_sdk-starknet
pip install -e .
cd ..
```

**Why Python?** The Extended DEX uses SNIP-12 signatures which require the official Python SDK for compatibility. The Rust code calls a Python subprocess for signing.

### 3. Setup Credentials

**SECURITY NOTE:** Never commit your `.env` file to git. It's already in `.gitignore`.

Copy the example and fill in your credentials:

```bash
cp .env.example .env
```

Edit `.env` with your actual credentials:
```bash
# Extended DEX (from https://app.extended.exchange/)
API_KEY=your_api_key_here
STARK_PRIVATE=0xYOUR_PRIVATE_KEY
STARK_PUBLIC=0xYOUR_PUBLIC_KEY
VAULT_NUMBER=123456

# Pacifica (from https://app.pacifica.fi/)
SOL_WALLET=YourSolanaWalletAddress
API_PUBLIC=YourAgentWalletPublicKey
API_PRIVATE=YourAgentWalletPrivateKey
```

**How to get credentials:**
- **Extended DEX**: Dashboard → API Keys → Generate New Key
- **Pacifica**: Settings → API Management → Create Agent Wallet

### 4. Configure Trading Parameters

Edit `config.json` to set your risk parameters:
```json
{
  "filters": {
    "min_combined_volume_usd": 20000000,
    "max_intra_exchange_spread_pct": 0.15,
    "max_cross_exchange_spread_pct": 0.25,
    "min_net_apr_pct": 5.0
  },
  "trading": {
    "max_position_size_usd": 1000.0
  }
}
```

**Parameter Guide:**
- `min_combined_volume_usd`: Minimum 24h trading volume (lower = more opportunities, higher slippage risk)
- `max_intra_exchange_spread_pct`: Maximum bid-ask spread within each exchange (tighter = better execution)
- `max_cross_exchange_spread_pct`: Maximum price difference between exchanges (prevents arbitrage execution risk)
- `min_net_apr_pct`: Minimum net APR after costs (higher = fewer but more profitable opportunities)
- `max_position_size_usd`: Cap on position size per exchange (risk management)

### 5. Build and Run

**First, test the build:**
```bash
cargo build --release
```

**Run the bot:**
```bash
cargo run --release
```

Or run directly:
```bash
./target/release/extended_connector
```

**The bot will:**
1. ✅ Load credentials and configuration
2. 🔍 Scan for best opportunity immediately
3. 📊 Open delta neutral position (if opportunity found)
4. ⏱️ Monitor every 15 minutes and display status
5. 🔄 Automatically rotate to new opportunities after 48 hours

**To stop the bot:** Press `Ctrl+C` (will attempt to close positions gracefully)

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

---

## Troubleshooting

### Bot Won't Start

**Problem: "STARK_PRIVATE must be set in .env"**
- **Solution**: Ensure `.env` file exists and contains all required credentials
- Check: `ls -la .env` to verify file exists
- Verify format matches `.env.example`

**Problem: "Failed to spawn Python process"**
- **Solution**: Python not in PATH or wrong version
- Check: `python --version` (should be 3.8+)
- Try: `python3` instead of `python` in your PATH

**Problem: "No module named 'fast_stark_crypto'"**
- **Solution**: Python dependencies not installed correctly
- Run: `pip install -r requirements.txt`
- Then: `cd python_sdk-starknet && pip install -e . && cd ..`

**Problem: "No opportunities found matching criteria"**
- **Solution**: Filters too restrictive or low market volatility
- Try: Lower `min_combined_volume_usd` to 10000000 (10M)
- Try: Increase `max_intra_exchange_spread_pct` to 0.25
- Try: Lower `min_net_apr_pct` to 3.0

### Bot Crashes or Stops

**Problem: Bot exits with "Insufficient capital to open position"**
- **Solution**: Not enough free collateral on one or both exchanges
- Check balances on both exchange dashboards
- Ensure at least $100-200 free collateral on each
- Lower `max_position_size_usd` in config.json

**Problem: "Failed to close Extended position after 5 attempts"**
- **Solution**: Network issues or API rate limiting
- Check: Extended DEX status page
- Wait 5-10 minutes and restart bot
- Bot will attempt to close on next cycle

**Problem: Bot shows position but can't find it on exchange**
- **Solution**: Check `bot_state.json` and verify on exchange dashboards
- If position doesn't exist, manually edit or delete `bot_state.json`
- **CAUTION**: Only delete state file if you're certain no positions are open

### Order Placement Issues

**Problem: "Python signing failed"**
- **Solution**: Issue with order signing script
- Check: `python scripts/sign_order.py` for errors
- Verify: STARK keys are correct (compare with exchange dashboard)
- Try: Regenerate API keys on Extended DEX

**Problem: Orders rejected by exchange**
- **Solution**: Insufficient margin, invalid size, or API key permissions
- Check: API key has trading permissions enabled
- Check: Lot size meets exchange minimum (typically 0.001-0.01)
- Check: Free collateral > position size + fees

### Performance Issues

**Problem: Bot scans taking > 30 seconds**
- **Solution**: Network latency or API rate limits
- Increase: `fetch_timeout_seconds` to 60 in config.json
- Increase: `rate_limit_delay_ms` to 200-500
- Check: Your internet connection speed

**Problem: High memory usage**
- **Solution**: Normal for Rust async runtime, but can optimize
- Try: Running with `--release` flag (optimization enabled)
- Monitor: `htop` or Task Manager for actual usage

### Data / State Issues

**Problem: Bot keeps opening same position**
- **Solution**: State file corruption
- Stop bot (Ctrl+C)
- Backup: `cp bot_state.json bot_state.json.backup`
- Delete: `rm bot_state.json`
- Restart bot

**Problem: PnL not displaying correctly**
- **Solution**: Exchange API may be slow to update
- Wait: 1-2 minutes after position opens
- Check: Positions directly on exchange dashboards
- Verify: Both positions are actually open

### Getting Help

If problems persist:

1. **Check Logs**: Bot outputs detailed logs to console
2. **Enable Verbose Logging**: Set `RUST_LOG=debug` environment variable
3. **Check Exchange Status**:
   - Extended: https://status.extended.exchange (if available)
   - Pacifica: https://status.pacifica.fi (if available)
4. **Review Bot State**: `cat bot_state.json | jq` (if jq installed)
5. **GitHub Issues**: Open an issue with:
   - Error messages (redact private keys!)
   - Bot configuration (redact credentials!)
   - Steps to reproduce

**Emergency Stop:**
- Press `Ctrl+C` to stop bot
- Bot will attempt graceful shutdown
- Manually close positions on exchange dashboards if needed
- Check both exchanges to verify no open positions remain

---

## Documentation

- [Extended API Documentation](https://docs.extended.exchange)
- [Extended Exchange](https://app.extended.exchange)
- [Pacifica API](https://api.pacifica.fi)
- [CLAUDE.md](./CLAUDE.md) - Development guide

## License

MIT

## Contributing

Contributions welcome! Please open an issue or PR.

---

## ⚠️ IMPORTANT SECURITY & RISK WARNINGS

**READ THIS BEFORE USING:**

- 🔴 **FINANCIAL RISK**: Trading cryptocurrency involves substantial risk of loss. Only use funds you can afford to lose.
- 🔴 **NO GUARANTEES**: Past performance does not guarantee future results. The bot can lose money.
- 🔴 **ALPHA SOFTWARE**: This is experimental software. Bugs may exist that could result in financial loss.
- 🔴 **API KEY SECURITY**: Never share your API keys or private keys. Store credentials securely.
- 🔴 **START SMALL**: Always test with minimal amounts first (e.g., $10-50) before scaling up.
- 🔴 **ACTIVE MONITORING**: Do not run unattended with significant capital. Monitor regularly.
- 🔴 **YOUR RESPONSIBILITY**: You are solely responsible for any financial losses incurred.

**RECOMMENDED APPROACH:**
1. Start with $10-50
2. Monitor actively for 1-2 weeks
3. Gradually increase position sizes only after confidence is established

---

## Disclaimer

This software is for educational purposes. Use at your own risk. Always test with small amounts first. No guarantees of profitability.
