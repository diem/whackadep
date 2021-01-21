FROM rust:1.49.0

WORKDIR /app
COPY . .
COPY ../metrics ../

RUN rustup default nightly
RUN cargo build

CMD ["cargo", "+nightly", "run"]
