# ============================================================
# Noor Framework - Multi-stage Dockerfile
# Dockerfile متعدد المراحل لإطار عمل نور
# ============================================================
# Optimized for both production and weak servers.
# Multi-stage build keeps the final image small (~25MB).
#
# محسن للإنتاج والسيرفرات الضعيفة.
# ============================================================

# ---------- Stage 1: Builder ----------
FROM rust:1.96-slim AS builder

# Install build dependencies including curl and xz-utils for Zig
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

# Install Zig compiler for performance modules (optional - falls back to pure Rust)
RUN curl -L https://ziglang.org/download/0.11.0/zig-linux-x86_64-0.11.0.tar.xz -o /tmp/zig.tar.xz && \
    tar -xJf /tmp/zig.tar.xz -C /usr/local && \
    ln -s /usr/local/zig-linux-x86_64-0.11.0/zig /usr/local/bin/zig && \
    rm /tmp/zig.tar.xz || echo "Zig installation skipped - using pure Rust fallback"

WORKDIR /app

# Copy Cargo files first (for better caching)
COPY Cargo.toml Cargo.lock* build.rs ./

# Copy source code
COPY src/ ./src/
COPY noor.toml ./

# Build the framework with optimizations
RUN cargo build --release --bin noor-server

# ---------- Stage 2: Runtime ----------
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies (minimal) + curl for health checks
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user for security
RUN useradd -m -u 1000 noor

# Create necessary directories
RUN mkdir -p /app/storage/cache /app/storage/logs /app/storage/uploads /app/public \
    && chown -R noor:noor /app

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/noor-server /app/noor-server
COPY --from=builder /app/noor.toml /app/noor.toml

# Copy public assets if they exist
COPY public/ /app/public/

# Switch to non-root user
USER noor

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the server
CMD ["/app/noor-server"]

# ---------- Stage 3: Weak Server Variant (optional) ----------
# Use this stage for very weak servers (256MB RAM)
# docker build --target weak-server -t noor:weak .
FROM debian:bookworm-slim AS weak-server

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

RUN useradd -m -u 1000 noor
RUN mkdir -p /app/storage/cache /app/storage/logs /app/storage/uploads /app/public \
    && chown -R noor:noor /app

WORKDIR /app

# Copy the binary (built with --profile weak-server)
COPY --from=builder /app/target/release/noor-server /app/noor-server
COPY --from=builder /app/noor.toml /app/noor.toml

USER noor

EXPOSE 8080

# Minimal health check (less frequent for weak servers)
HEALTHCHECK --interval=60s --timeout=5s --retries=2 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["/app/noor-server"]
