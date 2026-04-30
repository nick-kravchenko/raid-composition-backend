FROM rust:1.90.0-alpine AS builder

LABEL authors="krava"

WORKDIR /app

COPY . .

RUN apk add --no-cache musl-dev

RUN cargo build --release --locked

FROM scratch

WORKDIR /app

COPY --from=builder /app/target/release/raid-composition-backend .

CMD ["./raid-composition-backend"]
