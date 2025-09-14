# Build stage
FROM rust:1.85-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    perl \
    make \
    build-essential \
    protobuf-compiler \
    clang \
    lld \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Configure Rust to use Clang for better ARM64 emulation stability
ENV CC=clang
ENV CXX=clang++
ENV AR=llvm-ar

# Use system OpenSSL instead of building from source (better for cross-compilation)
ENV OPENSSL_STATIC=0
ENV OPENSSL_DIR=/usr
ENV PKG_CONFIG_PATH=/usr/lib/pkgconfig
ENV PKG_CONFIG_ALLOW_CROSS=1

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./

# Copy proto files (needed for build.rs)
COPY proto/ proto/

# Copy source code
COPY src/ src/

# Build the application with Clang
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false appuser

# Create directories
RUN mkdir -p /app/certs/storage && chown -R appuser:appuser /app

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/cert-agent /app/cert-agent

# Copy configuration
COPY config/ config/

# Change ownership
RUN chown -R appuser:appuser /app

# Switch to non-root user
USER appuser

# Expose gRPC port
EXPOSE 50051

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD nc -z localhost 50051 || exit 1

# Run the application
CMD ["./cert-agent"]
