FROM rust:1.99.1-alpine AS builder

LABEL authors="krava"

WORKDIR /app

COPY . .

RUN apk add --no-cache musl-dev ca-certificates

RUN cargo build --release --locked

FROM scratch

WORKDIR /app

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /app/target/release/raid-composition-backend .

CMD ["./raid-composition-backend"]
