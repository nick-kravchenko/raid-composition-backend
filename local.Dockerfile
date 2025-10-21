FROM rust:1.90.0-alpine

LABEL authors="krava"

WORKDIR /app

RUN apk add --no-cache musl-dev

RUN cargo install cargo-watch

COPY . .

CMD ["cargo", "watch", "-x", "run"]
