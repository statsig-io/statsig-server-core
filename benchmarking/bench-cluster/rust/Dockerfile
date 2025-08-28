FROM rust:1.88 AS builder

ARG RELEASE_TAG
ENV RELEASE_TAG=${RELEASE_TAG}

RUN apt-get update && apt-get install -y jq

WORKDIR /app

COPY Cargo.toml ./
COPY src ./src
COPY build.rs ./build.rs
COPY install.sh ./install.sh
RUN chmod +x ./install.sh && ./install.sh

ENV RUSTFLAGS="--cfg statsig_kong"
RUN cargo build --release


# Running the benchmark
FROM ubuntu:22.04
COPY --from=builder /app/target/release/rust-bench /usr/local/bin/rust-bench
CMD ["/usr/local/bin/rust-bench"]
