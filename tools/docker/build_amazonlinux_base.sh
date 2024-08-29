#!/bin/bash

# Force Exit on Errors
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../shared_constants"

SKIP_DOCKER_IMAGE_BUILD=${CI:-false}
RELEASE_MODE=false

IMAGE=$1
PLATFORM=$2

if [[ "$3" == "--release-mode" ]]; then
    RELEASE_MODE=true
fi

echo -e "${BOLD_TXT}-- Setup Summary --${NORMAL_TXT}"
echo "Targeting: $IMAGE"
echo "Skip Docker Image Build: $SKIP_DOCKER_IMAGE_BUILD"
echo "Build in Release Mode: $RELEASE_MODE"
echo -e "\n"

build_docker_image() {
    echo -e "${BOLD_TXT}-- Building Docker Image [$IMAGE] --${NORMAL_TXT}"
    docker build . \
        --platform $PLATFORM \
        -t statsig/core-sdk-compiler:$IMAGE \
        -f "$SCRIPT_DIR/Dockerfile.$IMAGE"
}

exec_cargo_build() {
    echo -e "\n${BOLD_TXT}-- Exec Cargo Build [$IMAGE] --${NORMAL_TXT}"

    local extras="-p statsig_ffi"
    if [ "$RELEASE_MODE" = true ]; then
        extras="$extras --release"
    fi

    docker run --rm -it \
        --platform $PLATFORM \
        -v "$SCRIPT_DIR/../../":/app \
        --env EXTRAS="$extras" \
        statsig/core-sdk-compiler:$IMAGE
}

if [ "$SKIP_DOCKER_IMAGE_BUILD" = false ]; then
    build_docker_image 
fi

exec_cargo_build
