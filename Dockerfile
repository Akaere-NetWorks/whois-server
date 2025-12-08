# ---------- Stage 1: Build ----------
FROM rust:1-trixie as builder

WORKDIR /app
RUN apt update && apt install -y python3-dev
COPY src .
RUN cargo build --release

# ---------- Stage 2: Run ----------
FROM debian:13-slim

WORKDIR /app
RUN apt update && apt install -y python3 python3-pip libpython3.13 git --no-install-recommends && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/whois-server .
COPY --from=builder /app/src/services/pixiv ./pixiv/
COPY --from=builder /app/requirements.txt .
RUN pip3 install --break-system-packages --no-cache-dir -r requirements.txt

EXPOSE 43
EXPOSE 9999

CMD ["./whois-server"]
