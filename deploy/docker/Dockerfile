# Multi-stage build for SUNDAY Rust Backend
#
# Replaces the legacy Python FastAPI server with the native Rust Axum server.

# ---------------------------------------------------------------------------
# Stage 1: Build frontend SPA
# ---------------------------------------------------------------------------
FROM node:22-slim AS frontend

WORKDIR /frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci --ignore-scripts 2>/dev/null || npm install
COPY frontend/ .
RUN npm run build

# ---------------------------------------------------------------------------
# Stage 2: Build Rust workspace
# ---------------------------------------------------------------------------
FROM rust:1.85-slim-bookworm AS rust-builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY rust/Cargo.toml rust/Cargo.lock ./rust/
COPY rust/crates/ ./rust/crates/
COPY configs/ ./configs/

# Build the release binary
RUN cd rust && cargo build --release -p sunday-server

# ---------------------------------------------------------------------------
# Stage 3: Runtime
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Rust binary
COPY --from=rust-builder /app/rust/target/release/sunday-server /usr/local/bin/sunday-server

# Copy frontend build for static serving
COPY --from=frontend /frontend/dist /app/frontend/dist

# Copy default config
COPY --from=rust-builder /app/configs/sunday/config.toml /app/configs/sunday/config.toml

ENV SUNDAY_HOST=0.0.0.0
ENV SUNDAY_PORT=8000

EXPOSE 8000

ENTRYPOINT ["sunday-server"]
