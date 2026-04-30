FROM rust:1.90.0-alpine

LABEL authors="krava"

WORKDIR /app

RUN apk add --no-cache musl-dev

RUN cargo install cargo-watch
RUN cargo install sqlx-cli --version 0.8.6 --no-default-features --features postgres

COPY . .

CMD ["cargo", "watch", "-x", "run"]
