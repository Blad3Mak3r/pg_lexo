# Build stage: Compile the pg_lexo extension
FROM rust:1.83-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    clang \
    llvm \
    pkgconf \
    openssl-dev \
    postgresql17-dev \
    git

# Install cargo-pgrx
RUN cargo install cargo-pgrx --version "0.16.1" --locked

# Initialize pgrx for PostgreSQL 17
RUN cargo pgrx init --pg17=/usr/bin/pg_config

# Create working directory
WORKDIR /build

# Copy source code
COPY . .

# Build the extension for PostgreSQL 17
RUN cargo pgrx package --pg-config /usr/bin/pg_config

# Runtime stage: PostgreSQL 17 Alpine with the extension
FROM postgres:17-alpine

# Copy the built extension files from builder
COPY --from=builder /build/target/release/pg_lexo-pg17/usr/share/postgresql/extension/pg_lexo* /usr/local/share/postgresql/extension/
COPY --from=builder /build/target/release/pg_lexo-pg17/usr/lib/postgresql/pg_lexo.so /usr/local/lib/postgresql/

# The extension will be available for: CREATE EXTENSION pg_lexo;
