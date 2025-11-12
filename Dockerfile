FROM rust:1.75 as builder

WORKDIR /app
COPY . .

RUN cargo build --release --workspace

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/timed /usr/local/bin/

EXPOSE 8080 8081

CMD ["timed", "start"]
