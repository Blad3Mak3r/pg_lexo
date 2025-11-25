# Build stage: Compile the pg_order extension
FROM rust:1.83-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    clang \
    llvm \
    pkgconf \
    openssl-dev \
    postgresql18-dev \
    git

# Install cargo-pgrx
RUN cargo install cargo-pgrx --version "0.16.1" --locked

# Initialize pgrx for PostgreSQL 18
RUN cargo pgrx init --pg18=/usr/bin/pg_config

# Create working directory
WORKDIR /build

# Copy source code
COPY . .

# Build the extension for PostgreSQL 18
RUN cargo pgrx package --pg-config /usr/bin/pg_config

# Runtime stage: PostgreSQL 18 Alpine with the extension
FROM postgres:18-alpine

# Copy the built extension files from builder
COPY --from=builder /build/target/release/pg_order-pg18/usr/share/postgresql/extension/pg_order* /usr/local/share/postgresql/extension/
COPY --from=builder /build/target/release/pg_order-pg18/usr/lib/postgresql/pg_order.so /usr/local/lib/postgresql/

# The extension will be available for: CREATE EXTENSION pg_order;
