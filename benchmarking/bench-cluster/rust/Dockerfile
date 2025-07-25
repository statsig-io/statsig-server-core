FROM rust:1.88 AS builder

RUN apt-get update && apt-get install -y jq

WORKDIR /app

COPY Cargo.toml ./
COPY src ./src
COPY build.rs ./build.rs

# Install the latest statsig-rust version
RUN curl -s "https://crates.io/api/v1/crates/statsig-rust/versions" | jq '.versions | sort_by(.created_at) | reverse | .[0].num' > statsig-rust-version.txt
RUN cargo add statsig-rust@=$(cat statsig-rust-version.txt | tr -d '"')

RUN cargo build --release


# Running the benchmark
FROM debian:bookworm-slim
COPY --from=builder /app/target/release/rust-bench /usr/local/bin/rust-bench
CMD ["/usr/local/bin/rust-bench"]
