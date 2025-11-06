FROM rust:1.90.0-alpine3.22 AS builder

RUN apk upgrade --no-cache && apk add --no-cache musl-dev

WORKDIR /usr/src/notifier-rs

COPY schemas ./schemas
COPY build.rs ./build.rs
COPY Cargo.lock Cargo.toml ./
COPY src ./src

RUN cargo build --locked --release && \
    cp target/release/notifier-rs /usr/src/notifier-rs/notifier-rs  # Copy final binary to persistent path

FROM alpine:3.22

COPY --from=builder /usr/src/notifier-rs/notifier-rs /usr/local/bin/notifier-rs
WORKDIR /
CMD ["notifier-rs"]
