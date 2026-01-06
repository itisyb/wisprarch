FROM rust:1.83-slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libasound2-dev \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libasound2 \
    libssl3 \
    ca-certificates \
    pulseaudio-utils \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/whisperarch /usr/local/bin/

VOLUME ["/config", "/models"]

ENV WHISPERARCH_CONFIG_DIR=/config
ENV WHISPERARCH_MODELS_DIR=/models

EXPOSE 3737

CMD ["whisperarch"]
