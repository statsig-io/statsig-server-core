#!/bin/bash

IMAGE="amazonlinux2023-x86_64"
PLATFORM="linux/amd64"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
"$SCRIPT_DIR/build_amazonlinux_base.sh" "$IMAGE" "$PLATFORM" "$1"