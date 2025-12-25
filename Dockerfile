# ---------- Stage 1: Build ----------
FROM rust:1-trixie as builder

WORKDIR /app
COPY src ./src/
COPY data ./data/
COPY Cargo.toml .
COPY Cargo.lock .
RUN apt update && apt install -y lua5.4 liblua5.4-dev pkg-config build-essential
RUN cargo build --release

# ---------- Stage 2: Run ----------
FROM debian:13-slim

WORKDIR /app
RUN apt update && apt install -y git lua5.4 liblua5.4-dev--no-install-recommends && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/whois-server .

EXPOSE 43
EXPOSE 9999

CMD ["./whois-server"]
