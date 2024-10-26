# TODO: Make this less messy

FROM rust:1.82.0-alpine3.20 AS builder

RUN apk upgrade --no-cache && apk add --no-cache musl-dev openssl openssl-dev pkgconfig libcrypto3 libgcc

WORKDIR /usr/src/notifier-rs

COPY Cargo.lock Cargo.toml ./

COPY src ./src
COPY build.rs ./build.rs
COPY schemas ./schemas

RUN RUSTFLAGS="-C target-feature=-crt-static $(pkg-config openssl --libs)" cargo install --target \
    x86_64-unknown-linux-musl --path .

FROM alpine:3.20.3
RUN apk upgrade --no-cache && apk add --no-cache openssl openssl-dev pkgconfig libcrypto3 libgcc

COPY --from=builder /usr/local/cargo/bin/notifier-rs /usr/local/bin/notifier-rs
WORKDIR /
CMD ["notifier-rs"]
