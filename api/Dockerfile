# ===================================================================
# ZK Private Lending API - Dockerfile
# ===================================================================
#
# Multi-stage build for optimal image size
#
# Interview Q&A:
#
# Q: 왜 Multi-stage build를 사용하는가?
# A: 최종 이미지 크기 최소화
#    - Build stage: 모든 빌드 도구, 의존성 (~2GB)
#    - Runtime stage: 실행 파일만 (~100MB)
#    - 보안: 소스코드, 빌드 도구 노출 안 됨
#
# Q: Alpine vs Debian?
# A: 여기서는 Debian slim 사용
#    - Halo2는 glibc 필요 (Alpine은 musl)
#    - 더 나은 호환성
#
# ===================================================================

# ============ Stage 1: Builder ============
FROM rust:1.75-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    build-essential \
    git \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy Cargo files first (for dependency caching)
COPY api/Cargo.toml api/Cargo.lock ./api/
COPY circuits/halo2/Cargo.toml ./circuits/halo2/

# Create dummy source files for dependency caching
RUN mkdir -p api/src circuits/halo2/src \
    && echo "fn main() {}" > api/src/main.rs \
    && echo "pub fn dummy() {}" > circuits/halo2/src/lib.rs

# Build dependencies only (cached layer)
WORKDIR /app/api
RUN cargo build --release 2>/dev/null || true

# Copy actual source code
WORKDIR /app
COPY api/ ./api/
COPY circuits/halo2/ ./circuits/halo2/

# Touch main.rs to trigger rebuild
RUN touch api/src/main.rs

# Build the actual application
WORKDIR /app/api
RUN cargo build --release

# ============ Stage 2: Runtime ============
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -s /bin/bash app
USER app

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/api/target/release/zk-lending-api /app/zk-lending-api

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Run the application
CMD ["./zk-lending-api"]
