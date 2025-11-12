# Docker Deployment Guide

This guide covers running the Extended/Pacifica delta neutral trading bot using Docker.

## Prerequisites

- Docker Desktop (Windows/Mac) or Docker Engine (Linux)
- Docker Compose v2.0+
- Your `.env` file with API credentials (see main README)

## Quick Start

### 1. Build the Image

```bash
docker-compose build
```

This creates a multi-stage Docker image (~874MB) with:
- Rust 1.91 runtime
- Python 3.11 with starknet-py and cairo-lang
- Compiled trading bot binary
- Order signing scripts

### 2. Start the Bot

```bash
docker-compose up -d
```

The bot will:
- Load credentials from `.env`
- Mount `config.json` for filtering parameters
- Persist state to `bot_state.json`
- Run in detached mode with auto-restart

### 3. View Logs

```bash
# Stream logs
docker-compose logs -f trading-bot

# View last 100 lines
docker-compose logs --tail=100 trading-bot
```

### 4. Stop the Bot

```bash
# Graceful stop
docker-compose down

# Stop and remove volumes (careful!)
docker-compose down -v
```

## Configuration

### Environment Variables

All credentials are loaded from `.env` in the project root:

```bash
# Extended DEX
API_KEY=dc88...
STARK_PUBLIC=0x338...
STARK_PRIVATE=0x1...
VAULT_NUMBER=226109
EXTENDED_ENV=mainnet

# Pacifica
SOL_WALLET=H2rV...
API_PUBLIC=GXV6...
API_PRIVATE=4okN...

# Optional: Logging
RUST_LOG=info  # or debug, trace
```

### Volume Mounts

The container mounts two files from your host:

- `./config.json` → `/app/config.json` (read-only)
  - Filter parameters (min volume, spreads, APR)
- `./bot_state.json` → `/app/bot_state.json` (read-write)
  - Current position and rotation state
  - Persists across container restarts

### Resource Limits

Default limits (adjust in `docker-compose.yml`):

```yaml
limits:
  cpus: '1.0'
  memory: 512M
reservations:
  cpus: '0.5'
  memory: 256M
```

## Management Commands

### Check Container Status

```bash
docker-compose ps
```

### Check Resource Usage

```bash
docker stats extended-pacifica-bot
```

### Execute Commands in Container

```bash
# Get a shell
docker-compose exec trading-bot /bin/bash

# Check Python installation
docker-compose exec trading-bot python3 --version

# View state file
docker-compose exec trading-bot cat /app/bot_state.json
```

### Restart Bot

```bash
docker-compose restart trading-bot
```

### Update Configuration

1. Edit `config.json` on host
2. Restart container: `docker-compose restart trading-bot`
3. Configuration is remounted automatically

### View Health Status

```bash
docker inspect extended-pacifica-bot --format='{{.State.Health.Status}}'
```

Health check runs every 5 minutes and verifies the bot process is running.

## Troubleshooting

### Container Won't Start

**Check logs:**
```bash
docker-compose logs trading-bot
```

**Common issues:**
- Missing `.env` file
- Invalid credentials in `.env`
- `config.json` not found

### Bot Keeps Restarting

**View exit code:**
```bash
docker-compose ps
```

**Check for errors:**
```bash
docker-compose logs --tail=50 trading-bot
```

**Possible causes:**
- Insufficient balance on exchanges
- Network connectivity issues
- Invalid API keys

### Python Signing Issues

**Verify Python packages:**
```bash
docker-compose exec trading-bot pip3 list | grep -E "starknet|cairo"
```

Should show:
- `cairo-lang 0.13.1`
- `starknet-py 0.20.0`

**Test signing script:**
```bash
docker-compose exec trading-bot python3 /app/scripts/sign_order.py
```

### State File Corrupted

**Backup and reset:**
```bash
cp bot_state.json bot_state.json.backup
echo '{}' > bot_state.json
docker-compose restart trading-bot
```

## Development Workflow

### Rebuild After Code Changes

```bash
# Rebuild image
docker-compose build

# Restart with new image
docker-compose up -d
```

### Run with Debug Logging

Edit `docker-compose.yml`:
```yaml
environment:
  - RUST_LOG=debug  # or trace for verbose
```

Then restart:
```bash
docker-compose up -d
```

### Test Build Without Running

```bash
docker-compose build
docker-compose config
```

## Production Deployment

### Use Named Volumes

Edit `docker-compose.yml`:

```yaml
volumes:
  - ./config.json:/app/config.json:ro
  - bot-data:/app/bot_state.json  # Named volume

volumes:
  bot-data:
    driver: local
```

### Enable Auto-Restart

Already configured with `restart: unless-stopped`

### Set Resource Limits

Adjust based on your VPS:

```yaml
deploy:
  resources:
    limits:
      cpus: '2.0'      # 2 CPU cores
      memory: 1G       # 1GB RAM
```

### Monitor Logs

Use log aggregation:
```bash
docker-compose logs -f trading-bot | tee logs/bot.log
```

Or configure external logging:
```yaml
logging:
  driver: "syslog"
  options:
    syslog-address: "tcp://your-log-server:514"
```

### Backup State

Schedule regular backups:
```bash
# Cron job example (daily at 2 AM)
0 2 * * * cp /path/to/bot_state.json /path/to/backups/bot_state_$(date +\%Y\%m\%d).json
```

## Security Notes

1. **Never commit `.env`** - Contains private keys
2. **Use read-only mounts** - `config.json` is mounted read-only
3. **Limit network access** - Uses bridge network (isolated)
4. **No privileged mode** - Container runs without elevated privileges
5. **Monitor logs** - Watch for suspicious activity

## Performance Notes

- **Build time**: 5-10 minutes (first build)
- **Image size**: ~874MB
- **Memory usage**: ~200-300MB during operation
- **CPU usage**: Minimal when idle, spikes during trades

## Support

For issues specific to Docker deployment:
1. Check this guide first
2. Review container logs
3. Verify host configuration
4. Check Docker daemon status

For bot functionality issues, see main `CLAUDE.md` documentation.
