#!/bin/bash

IMAGE="amazonlinux2-arm64"
PLATFORM="linux/arm64"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
"$SCRIPT_DIR/build_amazonlinux_base.sh" "$IMAGE" "$PLATFORM" "$1"