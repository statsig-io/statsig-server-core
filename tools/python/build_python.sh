#!/bin/bash

set -e # Force Exit on Errors
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../shared_constants"


# Get the directory of the script
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SRC_DIR="$SCRIPT_DIR/../../statsig-ffi/bindings/python"
DST_DIR="$SCRIPT_DIR/../../build/python"
EXTRA_ARGS="${@:1}"

ensure_empty_dir() {
    echo -e "${BOLD_TXT}-- Ensure Empty Build Dir --${NORMAL_TXT}"
    $SCRIPT_DIR/../ensure_empty_dir "$DST_DIR"
}

ensure_empty_dir
