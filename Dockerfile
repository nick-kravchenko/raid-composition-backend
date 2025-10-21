FROM rust:1.90.0-alpine as builder

LABEL authors="krava"

WORKDIR /app

COPY . .

RUN cargo build --release

FROM alpine:latest

WORKDIR /app

COPY --from=builder /app/target/release/raid-composition-backend .

CMD ["./raid-composition-backend"]
