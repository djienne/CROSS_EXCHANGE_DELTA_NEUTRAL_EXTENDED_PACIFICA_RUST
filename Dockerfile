# Multi-stage build for Rust trading bot
# Stage 1: Build the application
FROM rust:1.91-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies (cache layer)
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn lib() {}" > src/lib.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release || true

# Remove dummy files
RUN rm -rf src

# Copy actual source code
COPY src ./src
COPY examples ./examples
COPY scripts ./scripts

# Build the actual application
RUN cargo build --release

# Stage 2: Runtime image
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    python3 \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

# Install Python dependencies for order signing
RUN pip3 install --no-cache-dir \
    starknet-py==0.20.0 \
    cairo-lang==0.13.1 \
    --break-system-packages

# Copy compiled binary from builder
COPY --from=builder /app/target/release/extended_connector /app/extended_connector

# Copy Python signing script
COPY --from=builder /app/scripts /app/scripts

# Copy entrypoint script
COPY docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

# Copy configuration template (will be overridden by volume mount)
COPY config.json /app/config.json

# Create directory for state persistence
RUN mkdir -p /app/data

# Set environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Health check (checks if process is running)
HEALTHCHECK --interval=5m --timeout=10s --start-period=30s --retries=3 \
    CMD pgrep -f extended_connector || exit 1

# Run the bot
ENTRYPOINT ["/app/docker-entrypoint.sh"]
