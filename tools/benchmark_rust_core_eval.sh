#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ITERATIONS="${STATSIG_BENCH_ITERATIONS:-10000}"

cd "$ROOT_DIR"

echo "Running Rust core evaluation benchmark with ${ITERATIONS} uncached users"
STATSIG_BENCH_ITERATIONS="$ITERATIONS" \
  cargo test -p statsig-rust --test core_eval_benchmark --release -- --ignored --nocapture
