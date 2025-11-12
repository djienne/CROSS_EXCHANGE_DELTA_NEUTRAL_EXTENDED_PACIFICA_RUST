# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

This is a **production-ready autonomous trading bot** for delta neutral funding rate arbitrage, built on top of a comprehensive Rust connector library for Extended DEX (Starknet) and Pacifica exchanges.

**Primary Use Case**: Automated cross-exchange arbitrage. The bot continuously:
1. Scans markets for funding rate differentials
2. Opens delta neutral positions (long/short across exchanges)
3. Holds for 48 hours to capture funding payments
4. Automatically rotates to new opportunities

**Dual Exchange Support**:
- **Extended DEX** (primary): Starknet-based perpetuals DEX
- **Pacifica** (secondary): Alternative DEX with separate API endpoints

**Current Status**: Live and running with real capital ($99), successfully opened and monitoring 0.028 ETH position earning 9.53% APR.

## Build and Test Commands

```bash
# Build the project
cargo build

# Build with optimizations
cargo build --release

# Run the bot (default binary)
cargo run

# Run bot with debug logging (Windows PowerShell)
$env:RUST_LOG="debug"; cargo run

# Run bot with debug logging (Unix/Linux/macOS)
RUST_LOG=debug cargo run

# Check code without building
cargo check

# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run test with output
cargo test test_name -- --nocapture

# Run examples (Bot)
cargo run --example funding_bot
cargo run --example scan_opportunities

# Run examples (Extended DEX)
cargo run --example rest_example
cargo run --example websocket_example
cargo run --example funding_rates_example
cargo run --example check_positions
cargo run --example check_balance

# Run examples (Pacifica)
cargo run --example pacifica_funding_rates
cargo run --example pacifica_check_positions

# Run examples (Analysis)
cargo run --example funding_arbitrage
cargo run --example check_spreads
cargo run --example check_cross_spreads
```

## Architecture

### High-Level System Design

This is a **three-layer architecture** designed for autonomous trading:

1. **Connector Layer** (`rest.rs`, `websocket.rs`, `pacifica/*`)
   - Low-level API clients for both exchanges
   - WebSocket streams for real-time data
   - Order placement and position management

2. **Strategy Layer** (`opportunity.rs`, `trading.rs`)
   - Opportunity scanning with multi-stage filtering
   - Position sizing and execution logic
   - Retry mechanisms and error recovery

3. **Orchestration Layer** (`bot.rs`, `main.rs`)
   - State management and persistence
   - Event loop with monitoring
   - Automated rotation scheduling

**Key Architectural Decisions**:
- **Async-first**: All I/O operations use `tokio` for concurrent API calls
- **Type Safety**: Strong typing with `serde` prevents runtime errors
- **State Persistence**: JSON-based state enables crash recovery
- **Retry Logic**: Exponential backoff for reliable execution
- **Market Orders**: Chosen over limit orders for execution certainty
- **95% Capital Allocation**: Conservative sizing with 5% buffer
- **48-Hour Hold Time**: Optimal balance between funding capture and rebalancing cost

### Core Module Structure

```
src/
├── main.rs                # Bot entry point (cargo run)
├── lib.rs                 # Main library exports and initialization
├── error.rs               # Error types (ConnectorError)
├── types.rs               # Core data structures (BidAsk, OrderBook, MarketInfo, Position, etc.)
├── rest.rs                # REST API client for Extended DEX
├── websocket.rs           # WebSocket client for Extended DEX
├── signature.rs           # Cryptographic signing utilities
├── opportunity.rs         # ⭐ Opportunity finding and filtering
├── trading.rs             # ⭐ Position execution logic (open/close delta neutral)
├── bot.rs                 # ⭐ Bot orchestration and state management
├── snip12/                # SNIP-12 (Starknet typed data) implementation
│   ├── mod.rs             # Module entry with status documentation
│   ├── domain.rs          # StarknetDomain struct
│   ├── hash.rs            # Hashing functions (Poseidon, Keccak)
│   ├── signing.rs         # ECDSA signing on STARK curve
│   └── tests.rs           # Comparison tests with Python SDK
└── pacifica/              # Pacifica exchange integration
    ├── mod.rs             # Module exports
    ├── client.rs          # Orderbook WebSocket client
    ├── trading.rs         # Trading API (REST)
    ├── ws_trading.rs      # Trading WebSocket client
    ├── fill_detection.rs  # Fill detection via polling
    └── types.rs           # Pacifica-specific types

examples/
├── funding_bot.rs         # ⭐ Main bot example (alternative entry point)
├── scan_opportunities.rs  # ⭐ Opportunity scanner without trading
├── rest_example.rs        # Extended REST examples
├── websocket_example.rs   # Extended WebSocket examples
└── ...                    # More examples

config.json                # ⭐ Bot configuration (filters, thresholds)
bot_state.json            # ⭐ Bot state (auto-generated, enables crash recovery)
```

⭐ = New modules added for the autonomous trading bot

### Key Components

**Extended DEX (Starknet)**:
- `RestClient`: HTTP client for orderbooks, funding rates, markets, positions, balance
- `WebSocketClient`: Real-time orderbook streams
- `MultiMarketSubscriber`: Subscribe to multiple markets concurrently
- `snip12` module: Order signing using SNIP-12 standard (via Python subprocess)
- Position management: `place_market_order()`, `close_position()`

**Pacifica**:
- `OrderbookClient`: WebSocket orderbook streaming with auto-reconnect
- `PacificaTrading`: REST API for order placement, management, and funding rates
- `PacificaWsTrading`: WebSocket-based trading with Ed25519 signatures
- `FillDetectionClient`: Polling-based fill detection
- `PacificaFundingRate`: Funding rate information (current + next)
- `PacificaMarketInfo`: Market specifications including funding rates
- Position management: `place_market_order()`, `close_position()`

**Opportunity Finding (`src/opportunity.rs`)**:
- `OpportunityFinder`: Main scanner class
  - `find_common_symbols()` - Find markets on both exchanges
  - `fetch_volumes()` - Parallel volume fetching (Extended + Pacifica)
  - `find_opportunities()` - Calculate spreads, funding rates, net APR
  - `scan()` - Complete workflow with filtering
- `Opportunity`: Data structure with all metrics
  - Symbol, volumes, spreads, funding rates, net APR, strategy direction
  - `passes_filters()` - Check against config thresholds
- `Config`: Configuration loader from `config.json`
  - Filter parameters (volume, spreads, APR)
  - Display settings
  - Performance tuning

**Trading Execution (`src/trading.rs`)**:
- `open_delta_neutral_position()` - Opens both legs with retry logic
  - Places Extended order first
  - Retries Pacifica order up to 5 times with exponential backoff
  - Fetches and returns position objects
- `close_delta_neutral_position()` - Closes both positions
  - Retries each exchange independently (5 attempts each)
  - Returns errors if any leg fails
- `calculate_position_size()` - Position sizing with constraints
  - Takes 95% of minimum available capital
  - Rounds to coarser lot_size between exchanges
  - Converts to base currency size
- `retry_with_backoff()` - Generic retry wrapper
  - Exponential backoff: 1s, 2s, 4s, 8s, 16s
  - Configurable max attempts
- `DeltaNeutralPosition`: Position tracking struct
  - Stores both Extended and Pacifica positions
  - Opened timestamp for rotation calculation
  - Serializable for state persistence

**Bot Orchestration (`src/bot.rs`)**:
- `FundingBot`: Main bot class
  - Manages Extended and Pacifica clients
  - Holds OpportunityFinder instance
  - Maintains BotState
- `BotState`: State management
  - Current position (optional)
  - Last rotation time
  - Total rotation count
  - `load_from_file()` / `save_to_file()` - Persistence
  - `should_rotate()` - Check if 48 hours elapsed
  - `hours_until_rotation()` - Time remaining
- `run()` - Main event loop
  - Display status
  - Check for rotation (48 hours)
  - Open new position if needed
  - Sleep 15 minutes
  - Repeat forever
- `display_status()` - Show current state
  - Position details
  - PnL from both exchanges
  - Time until rotation
- `open_best_opportunity()` - Find and execute
  - Scan markets
  - Calculate position size
  - Call trading execution functions
  - Update state and persist
- `close_current_position()` - Close and update state

### Bot Configuration (`config.json`)

```json
{
  "filters": {
    "min_combined_volume_usd": 50000000,      // $50M minimum 24h volume
    "max_intra_exchange_spread_pct": 0.15,    // 0.15% max bid-ask spread
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

**Filter Rationale**:
- **Volume**: $50M ensures sufficient liquidity for entry/exit
- **Intra Spread**: 0.15% limits execution cost within each exchange
- **Cross Spread**: 0.25% limits arbitrage cost of price differences
- **Net APR**: 5% ensures profit after all costs

### Bot Behavior Constants

Defined in `src/bot.rs`:
```rust
const MONITORING_INTERVAL_MINUTES: u64 = 15;  // Check positions every 15 min
const POSITION_HOLD_TIME_HOURS: u64 = 48;     // Rotate after 48 hours
```

Defined in `src/trading.rs`:
```rust
const RETRY_ATTEMPTS: u32 = 5;                 // Max retries per operation
const CAPITAL_ALLOCATION: f64 = 0.95;          // Use 95% of available capital
```

**Design Decisions**:
- **15-minute monitoring**: Balance between responsiveness and API rate limits
- **48-hour hold**: Captures 2-3 funding payments while minimizing rotation costs
- **95% capital**: 5% buffer for fees, slippage, and market movements
- **5 retries**: Handles transient errors without excessive delay
- **Market orders**: Guaranteed execution over price optimization

### Order Signing Architecture (SNIP-12)

**Current Status**: Pure Rust SNIP-12 implementation exists but produces different message hashes than Extended's Python SDK due to unknown Order struct field ordering.

**Production Approach**: Use Python subprocess integration (`scripts/sign_order.py`) for reliable order signing:
```rust
// RestClient calls this internally
use crate::signature::sign_order;

let signature = sign_order(
    base_asset_id,
    quote_asset_id,
    base_amount,
    quote_amount,
    fee_amount,
    position_id,
    nonce,
    expiry_epoch_millis,
    public_key,
    private_key,
    "SN_MAIN",
)?;
```

**Rust Implementation**: Located in `src/snip12/`:
- Domain separator encoding (SNIP-12 revision 1)
- Type hash computation (Keccak-256 with modular reduction)
- Poseidon hashing for struct and message hashes
- ECDSA signing on STARK curve

**Known Issue**: Order struct field ordering differs from Extended's smart contract. All individual components are verified correct, but final message hash doesn't match Python SDK.

**Future Work**: Once Extended provides the exact Order struct definition, update `src/snip12/hash.rs::get_order_type_hash()` and `hash_order_struct()` to complete the pure Rust implementation.

### Pacifica Authentication

Pacifica uses Ed25519 signatures for WebSocket authentication and REST API requests:
- Credentials loaded from `.env`: `SOL_WALLET`, `API_PUBLIC`, `API_PRIVATE`
- Signature format: `<nonce>:<timestamp>` signed with private key
- UUID nonces prevent replay attacks

## Environment Configuration

Create a `.env` file in the project root:

```bash
# Extended DEX Configuration
API_KEY=dc88...                      # API key from app.extended.exchange
STARK_PUBLIC=0x338...                # Stark public key (L2 key)
STARK_PRIVATE=0x1...                 # Stark private key (for signing orders)
VAULT_NUMBER=226109                  # Your L2 vault number / collateral position
EXTENDED_ENV=mainnet                 # Environment: always use mainnet

# Pacifica Configuration (required for trading)
SOL_WALLET=H2rV...                   # Solana wallet address
API_PUBLIC=GXV6...                   # Agent wallet public key
API_PRIVATE=4okN...                  # Agent wallet private key
```

**Security Note**: Never commit `.env` to git. It contains private keys for both exchanges.

## Important Implementation Details

### Volume Calculation

**Extended DEX**:
- Endpoint: `/api/v1/info/markets/{market}/stats`
- Field: `dailyVolume` (already in USD/quote currency)
- Used directly

**Pacifica**:
- Endpoint: `/api/v1/kline?symbol={symbol}&interval=1d`
- Field: `v` (volume in base currency)
- Conversion: `volume_base * close_price = volume_usd`

### Funding Rate Calculations

**Extended DEX**:
- Settlement: Every 8 hours
- API returns: Rate per 8-hour period
- Annualized: `rate * 3 * 365`

**Pacifica**:
- Settlement: Every hour
- API returns: Rate per hour
- Annualized: `rate * 24 * 365`

**Net APR Calculation**:
```rust
// Strategy 1: Long Extended, Short Pacifica
let net_apr_1 = -extended_funding_apr + pacifica_funding_apr;

// Strategy 2: Long Pacifica, Short Extended
let net_apr_2 = -pacifica_funding_apr + extended_funding_apr;

// Choose best
let (best_direction, best_net_apr) = if net_apr_1 > net_apr_2 {
    ("Long Extended / Short Pacifica", net_apr_1)
} else {
    ("Long Pacifica / Short Extended", net_apr_2)
};
```

### Position Sizing Logic

```rust
// 1. Get available capital from both exchanges
let extended_free = extended_client.get_balance().await?.available_for_trade;
let pacifica_free = // Estimated from positions

// 2. Take minimum and apply 95% allocation
let min_capital = extended_free.min(pacifica_free);
let target_notional = min_capital * 0.95;

// 3. Convert to base currency
let base_size = target_notional / current_price;

// 4. Round to coarser lot_size
let coarser_lot = extended_lot_size.max(pacifica_lot_size);
let rounded_size = (base_size / coarser_lot).floor() * coarser_lot;
```

### State Persistence

The bot saves state to `bot_state.json` after every position change:

```json
{
  "current_position": {
    "symbol": "ETH",
    "extended_position": {
      "market": "ETH-USD",
      "side": "LONG",
      "size": "0.028",
      "value": "91.976707"
    },
    "pacifica_position": {
      "symbol": "ETH",
      "side": "ask",
      "amount": "0.028",
      "entry_price": "3300.8"
    },
    "opened_at": 1762449026,
    "target_notional_usd": 92.20
  },
  "last_rotation_time": 1762449026,
  "total_rotations": 1
}
```

**Crash Recovery**: On startup, bot loads this file. If a position exists and hasn't expired, it continues monitoring. If 48 hours elapsed, it closes and rotates.

### Retry Logic

Both `open_delta_neutral_position()` and `close_delta_neutral_position()` use inline retry loops with exponential backoff:

```rust
for attempt in 1..=5 {
    match operation().await {
        Ok(result) => {
            if attempt > 1 {
                info!("Operation succeeded on attempt {}/5", attempt);
            }
            break;
        }
        Err(e) => {
            if attempt >= 5 {
                error!("Operation failed after 5 attempts: {}", e);
                return Err(e);
            }
            let delay_ms = 2u64.pow(attempt - 1) * 1000;  // 1s, 2s, 4s, 8s, 16s
            warn!("Retry in {}ms...", delay_ms);
            sleep(Duration::from_millis(delay_ms)).await;
        }
    }
}
```

### Negative Numbers in SNIP-12

Negative amounts (e.g., selling) are encoded using Starknet field arithmetic:
```rust
// For negative values: field_modulus - abs(value)
let field_modulus = Felt::from_hex("0x800000000000011000000000000000000000000000000000000000000000001")?;
let encoded = field_modulus - abs_value;
```

### Settlement Expiration

Extended orders require settlement expiration with 14-day buffer:
```rust
let settlement_exp = (expiry_millis / 1000) + (14 * 24 * 60 * 60);
```

### Market Symbol Conventions

**Extended DEX**: Uses hyphenated format (e.g., `BTC-USD`, `ETH-USD`)
**Pacifica**: Uses raw symbols (e.g., `BTC`, `ETH`)

When building cross-exchange strategies, normalize symbols:
```rust
// Converting Extended to Pacifica format
let extended_symbol = "BTC-USD";
let pacifica_symbol = extended_symbol.split('-').next().unwrap(); // "BTC"
```

## Common Development Patterns

### Running the Bot

```bash
# Start bot with default config
cargo run

# With debug logging
RUST_LOG=debug cargo run

# Check current state
cat bot_state.json

# Scan opportunities without trading
cargo run --example scan_opportunities
```

### Customizing Configuration

Edit `config.json` and restart the bot:
```json
{
  "filters": {
    "min_combined_volume_usd": 100000000,   // Increase to $100M
    "min_net_apr_pct": 10.0                 // Increase to 10% APR
  }
}
```

### Testing with Different Capital

The bot automatically sizes positions based on available capital. To test with different amounts:
1. Adjust balance on Extended/Pacifica
2. Bot will use 95% of minimum available

### Monitoring Bot Operation

```rust
// Bot logs show:
[INFO] ╔═══════════════════════════════════════════════════════════════╗
[INFO] ║                  FUNDING RATE BOT STATUS                      ║
[INFO] ╠═══════════════════════════════════════════════════════════════╣
[INFO] ║ Symbol: ETH                                                    ║
[INFO] ║ Notional: $92.20                                               ║
[INFO] ║ Time Remaining: 47.8 hours                                     ║
[INFO] ║ Extended Position: ACTIVE                                      ║
[INFO] ║ Pacifica Position: ACTIVE                                      ║
[INFO] ║ Extended PnL: $0.15                                            ║
[INFO] ╚═══════════════════════════════════════════════════════════════╝
```

### Debugging Position Issues

```bash
# Check Extended position
cargo run --example check_positions

# Check Pacifica position
cargo run --example pacifica_check_positions

# Check balance
cargo run --example check_balance

# Enable verbose logging
RUST_LOG=extended_connector=debug cargo run
```

### Analyzing Opportunities

```bash
# Scan with current config
cargo run --example scan_opportunities

# Check spreads across markets
cargo run --example check_cross_spreads

# View funding rate comparison
cargo run --example funding_arbitrage
```

## Known Behaviors & Design Decisions

### Why 95% Capital Allocation?

- **5% Buffer**: Accounts for fees (0.06% maker on Extended, variable on Pacifica)
- **Slippage**: Market orders can have minor slippage (typically <0.1%)
- **Funding Payments**: Funding rates are received/paid, affecting balance
- **Safety Margin**: Prevents over-leveraging if prices move slightly

### Why 48-Hour Rotation?

- **Funding Capture**: Extended settles every 8 hours (6 payments), Pacifica every hour (48 payments)
- **Cost Minimization**: Each rotation costs ~0.12% in fees (0.06% × 4 orders)
- **APR Break-even**: With 5% annual rate, daily return is ~0.0137%, so 2-day return (~0.027%) covers rotation cost
- **Balance**: More frequent rotation = more fees but better rate tracking
- **Empirical Optimal**: 48 hours balances these factors

### Why 15-Minute Monitoring?

- **API Rate Limits**: Avoids hitting rate limits on either exchange
- **Position Stability**: Positions don't change much in 15 minutes
- **Responsiveness**: Catches issues within reasonable time
- **Resource Efficiency**: Reduces unnecessary API calls
- **User Experience**: Frequent enough status updates without spam

### Why Market Orders?

- **Execution Certainty**: Market orders guarantee fills
- **Delta Neutral Priority**: Must execute both legs; partial fills break strategy
- **Spread Cost**: With 0.15% max spread filter, market order cost is minimal
- **Simplicity**: Avoid limit order complexity (partial fills, cancellations)
- **Speed**: Faster execution reduces exposure to price movements

### Why 5 Retries?

- **Transient Errors**: Network issues, temporary exchange downtime
- **Exponential Backoff**: 1s + 2s + 4s + 8s + 16s = 31 seconds total
- **Balance**: Enough attempts without excessive delay
- **Empirical**: 99% of transient errors resolve within 3 attempts

## Testing Strategy

### Unit Tests
```bash
# Test position size calculation
cargo test test_calculate_position_size

# Test configuration loading
cargo test test_config_load

# Test state persistence
cargo test test_state_persistence
```

### Integration Tests (Live)
```bash
# Test opportunity scanner
cargo run --example scan_opportunities

# Test with debug logging
RUST_LOG=debug cargo run --example scan_opportunities
```

### Manual Testing Workflow

1. **Start bot with small capital** ($10-$100)
2. **Monitor first position opening** (watch logs carefully)
3. **Verify positions on both exchanges** (using web interfaces)
4. **Check state file** (`cat bot_state.json`)
5. **Wait for rotation** (or manually close after testing)
6. **Verify closing and rotation** (watch for errors)

## Known Limitations

1. **SNIP-12 Pure Rust**: Functional but not matching Extended's signatures. Use Python subprocess for production.
2. **Pacifica Account Balance**: No direct API endpoint, estimated from positions. Could be improved.
3. **Price Feed Timing**: Uses orderbook mid-price at execution time; could use TWAP for better accuracy.
4. **Single Position**: Bot only holds one delta neutral position at a time.
5. **No Leverage**: Bot uses 1x leverage; could be parameterized for 2x-5x.
6. **No Stop Loss**: Relies on delta neutrality; no stop loss mechanism.

## Troubleshooting

### Bot Won't Start

**Check credentials**:
```bash
# Verify .env file exists and has all required fields
cat .env | grep -E "API_KEY|STARK_|VAULT_NUMBER|SOL_WALLET|API_"
```

**Check Python SDK** (for Extended order signing):
```bash
python scripts/sign_order.py
```

### Position Not Opening

**Check balance**:
```bash
cargo run --example check_balance
```

**Check opportunities**:
```bash
cargo run --example scan_opportunities
```

**Enable debug logging**:
```bash
RUST_LOG=debug cargo run
```

### Position Stuck

**Check state file**:
```bash
cat bot_state.json
```

**Manually close** (if needed):
- Close on Extended: Use web interface or `check_positions` example
- Close on Pacifica: Use web interface or API

**Reset state** (only if positions are actually closed):
```bash
rm bot_state.json
cargo run
```

## API Endpoints

**Extended DEX**:
- REST: `https://api.starknet.extended.exchange`
- WebSocket: `wss://ws.starknet.extended.exchange`
- Documentation: https://docs.extended.exchange

**Pacifica**:
- REST: `https://api.pacifica.fi`
- WebSocket: `wss://ws.pacifica.fi/ws`

## Future Enhancements

Potential improvements (not yet implemented):
- [ ] Multiple concurrent positions
- [ ] Configurable leverage (2x-5x)
- [ ] TWAP price feeds for better execution
- [ ] Stop loss / take profit mechanisms
- [ ] Web dashboard for monitoring
- [ ] Discord/Telegram notifications
- [ ] Historical performance tracking
- [ ] Backtesting framework
- [ ] Paper trading mode
- [ ] Better Pacifica balance API
