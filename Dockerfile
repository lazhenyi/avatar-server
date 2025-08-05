FROM rust:1.88 as builder
WORKDIR /app
RUN USER=root cargo new --bin avatar-server
WORKDIR /app/avatar-server
COPY Cargo.toml Cargo.lock* ./
RUN cargo build --release
RUN rm src/*.rs
COPY src/ ./src/
RUN touch src/main.rs
RUN cargo build --release
FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN groupadd -r avatar && useradd -r -g avatar avatar
WORKDIR /app
RUN mkdir -p /app/uploads && chown -R avatar:avatar /app
COPY --from=builder /app/avatar-server/target/release/avatar-server /app/
USER avatar
ENV AUTH_TOKEN=DEFAULT \
    UPLOAD_DIR=/app/uploads \
    HOST=0.0.0.0 \
    PORT=8080 \
    RUST_LOG=info
EXPOSE 8080
CMD ["./avatar-server"]
