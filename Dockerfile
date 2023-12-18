# Build
FROM rust:1.74.0 AS builder
WORKDIR /usr/src/
RUN rustup target add x86_64-unknown-linux-gnu
RUN apt update && apt install libssl-dev

RUN USER=root cargo new receqif
WORKDIR /usr/src/receqif
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release --features "docker"

COPY src ./src
RUN cargo install --target x86_64-unknown-linux-gnu --features "docker" --path .

# Bundle
FROM gcr.io/distroless/cc-debian12
COPY --from=builder /usr/local/cargo/bin/receqif /bin/receqif
USER 1000
ARG TELEGRAM_TOKEN
ENV TELOXIDE_TOKEN=$TELEGRAM_TOKEN
VOLUME ["/etc/receqif"]
ENTRYPOINT ["/bin/receqif", "--telegram"]
