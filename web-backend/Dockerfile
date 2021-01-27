FROM rust:1.49.0

WORKDIR /app
COPY . .

RUN rustup default nightly
RUN cargo build

ENV RUST_LOG=info
CMD ["cargo", "run"]
