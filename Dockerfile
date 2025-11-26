# Build stage: Compile the pg_lexo extension
FROM rust:1.91-alpine AS builder

ENV RUST_BACKTRACE=1
ENV PGRX_BUILD_VERBOSE=true
ENV CARGO_TERM_COLOR=always

# Install build dependencies; fetch postgresql18-dev from edge/community repository only for that package
RUN apk add --no-cache \
    musl-dev \
    clang \
    llvm \
    pkgconf \
    openssl-dev \
    git \
 && apk add --no-cache --repository https://dl-cdn.alpinelinux.org/alpine/edge/community postgresql18-dev

# Install cargo-pgrx
RUN cargo install cargo-pgrx --version "0.16.1" --locked

# Initialize pgrx for PostgreSQL 18
RUN cargo pgrx init --pg18=/usr/bin/pg_config

# Create working directory
WORKDIR /build

# Copy source code
COPY . .

# Build the extension for PostgreSQL 18 with verbose output
RUN cargo pgrx package --verbose --pg-config /usr/bin/pg_config

# Runtime stage: PostgreSQL 18.1 Alpine with the extension
FROM postgres:18.1-alpine

# Copy the built extension files from builder
COPY --from=builder /build/target/release/pg_lexo-pg18/usr/share/postgresql/extension/pg_lexo* /usr/local/share/postgresql/extension/
COPY --from=builder /build/target/release/pg_lexo-pg18/usr/lib/postgresql/pg_lexo.so /usr/local/lib/postgresql/

# The extension will be available for: CREATE EXTENSION pg_lexo;
