#!/usr/bin/env bash
set -euo pipefail

# Set paths
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GO_DIR="$ROOT_DIR/statsig-go"
RUST_FFI_DIR="$ROOT_DIR/statsig-ffi"
BUILD_DIR="$GO_DIR/build"
DYLIB_PATH="$ROOT_DIR/target/aarch64-macos/release"
HEADER_PATH="$RUST_FFI_DIR/include"
BINARY_NAME="statsig-go-sdk"

# Clean and create build dir
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# Export necessary cgo variables
export CGO_CFLAGS="-I$HEADER_PATH"
export DYLD_LIBRARY_PATH="$DYLIB_PATH"

# Build Go binary
cd "$GO_DIR"
go build -o "$BUILD_DIR/$BINARY_NAME" main.go

cd "$BUILD_DIR"
tar -czf "${BINARY_NAME}.tar.gz" "$BINARY_NAME"

echo "✅ Built binary at $BUILD_DIR/$BINARY_NAME"
