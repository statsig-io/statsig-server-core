#!/usr/bin/env bash
# install.sh — strictly install latest -beta / -rc of statsig/statsig-php-core
set -euo pipefail

# ------------------------- Config -------------------------
: "${WORKDIR:=/app}"
: "${PHP_CORE_PKG:=statsig/statsig-php-core}"
: "${RELEASE_TAG:=stable}"
export COMPOSER_ALLOW_SUPERUSER=1

cd "$WORKDIR" >/dev/null 2>&1 || { echo "WORKDIR $WORKDIR not found"; exit 1; }

composer config prefer-stable true || true

if [ ! -f composer.lock ]; then
  echo "[init] composer.lock not found — running initial resolve..."
  composer update --no-interaction --no-progress
fi

TAG="$(echo "$RELEASE_TAG" | tr '[:upper:]' '[:lower:]' | sed 's/^v//')"

CHANNEL="stable"
if [[ "$TAG" == "rc" || "$TAG" =~ -rc(\.|$) ]]; then
  CHANNEL="rc"
elif [[ "$TAG" == "beta" || "$TAG" =~ -beta(\.|$) ]]; then
  CHANNEL="beta"
fi

# ------------------------- Helpers -------------------------
have_jq() { command -v jq >/dev/null 2>&1; }

refresh_metadata() {
  composer clear-cache || true
}

pick_latest_with_suffix_json() {
  local pkg="$1" suf="$2"
  have_jq || { echo ""; return; }
  composer show "$pkg" --all --format=json 2>/dev/null \
    | jq -r --arg suf "$suf" '
        (.versions // .releases // [])
        | map(select(type == "string"))
        | map(select(test("(?i)-\($suf)(\\.|$)")))
        | .[0] // empty
      '
}
