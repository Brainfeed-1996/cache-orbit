FROM rust:1.75-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs && cargo fetch
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/cache-orbit /usr/local/bin/cache-orbit
EXPOSE 6379 9090
ENTRYPOINT ["cache-orbit"]
CMD ["--port", "6379", "--metrics", "9090"]
