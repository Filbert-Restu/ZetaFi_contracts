FROM rust:slim AS builder

WORKDIR /usr/src/app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    libdbus-1-dev \
    && rm -rf /var/lib/apt/lists/*

# Add WebAssembly target
RUN rustup target add wasm32-unknown-unknown

# Install Soroban CLI
RUN cargo install --locked soroban-cli

# Copy contracts source
COPY . .

# Build the ledger contract
WORKDIR /usr/src/app/ledger
RUN cargo build --target wasm32-unknown-unknown --release

# Optimize the contract (optional but recommended)
RUN soroban contract optimize --wasm target/wasm32-unknown-unknown/release/zetafi_ledger.wasm

# Run tests
RUN cargo test

# --- Export Stage ---
# This stage just holds the built wasm files
FROM scratch AS exporter
COPY --from=builder /usr/src/app/ledger/target/wasm32-unknown-unknown/release/ledger.wasm /
COPY --from=builder /usr/src/app/ledger/target/wasm32-unknown-unknown/release/ledger.optimized.wasm /
