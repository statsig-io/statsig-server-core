FROM rust:1.88 AS builder

ARG RELEASE_TAG
ENV RELEASE_TAG=${RELEASE_TAG}

RUN apt-get update && apt-get install -y jq

WORKDIR /app

RUN PROTOC_ARCH=$(if [ "$(uname -m)" = "x86_64" ]; then echo "x86_64"; else echo "aarch_64"; fi) \
    && curl -LO https://github.com/protocolbuffers/protobuf/releases/download/v29.3/protoc-29.3-linux-${PROTOC_ARCH}.zip && \
    unzip protoc-29.3-linux-${PROTOC_ARCH}.zip -d /usr/local && \
    rm protoc-29.3-linux-${PROTOC_ARCH}.zip;

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
