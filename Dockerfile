FROM rust:1.90.0-alpine3.22 AS builder

RUN apk upgrade --no-cache && apk add --no-cache musl-dev

WORKDIR /usr/src/notifier-rs

COPY Cargo.lock Cargo.toml ./

COPY src ./src
COPY build.rs ./build.rs
COPY schemas ./schemas

RUN RUSTFLAGS="-C target-feature=-crt-static" cargo install --target x86_64-unknown-linux-musl --path .

FROM alpine:3.22

# libgcc required as runtime library by Rust; I think because of unwinding?
RUN apk upgrade --no-cache && apk --no-cache add libgcc

COPY --from=builder /usr/local/cargo/bin/notifier-rs /usr/local/bin/notifier-rs
WORKDIR /
CMD ["notifier-rs"]
