#!/bin/bash

# Force Exit on Errors
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../shared_constants"

TARGET=x86_64-unknown-linux-gnu
DOCKER_IMAGE=x-comp/$TARGET

docker build . -t $DOCKER_IMAGE -f "$SCRIPT_DIR/Dockerfile.$TARGET"
echo -e "${GREEN}Compiler Setup Successfully${NC}\n"

docker run --rm -it -v "$SCRIPT_DIR/../../":/app --env EXTRAS="-p statsig_ffi" $DOCKER_IMAGE
# docker run --rm -it -v "$SCRIPT_DIR/../../":/app --env EXTRAS="-p statsig_ffi --release" $DOCKER_IMAGE
