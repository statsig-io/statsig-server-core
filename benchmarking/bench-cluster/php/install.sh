#!/usr/bin/env bash
# install.sh — strictly pin statsig/statsig-php-core to latest rc/beta/exact/stable
set -euo pipefail

# ------------------------- Config -------------------------
: "${WORKDIR:=/app}"
: "${PHP_CORE_PKG:=statsig/statsig-php-core}"
: "${RELEASE_TAG:=stable}"
export COMPOSER_ALLOW_SUPERUSER=1

cd "$WORKDIR"
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
pick_latest_with_suffix() {
  local pkg="$1" suf="$2"
  composer show "$pkg" --all --format=json \
    | jq -r --arg suf "$suf" '
        (.versions // .releases // [])
        | map(select(type == "string"))
        | map(select(test("^\\d")))
        | map(select(test("(?i)-\($suf)(\\.|$)")))
        | .[0] // empty
      '
}

dump_versions_for_debug() {
  local pkg="$1"
  echo "---------- DEBUG: first 30 available versions ----------"
  composer show "$pkg" --all --format=json \
    | jq -r '(.versions // .releases // [])[:30]'
  echo "--------------------------------------------------------"
}

require_exact() {
  local pkg="$1" ver="$2"
  echo "[core] requiring ${pkg}:${ver}"
  composer require "${pkg}:${ver}" --no-interaction --no-progress
}

# ------------------------- Main -------------------------
if [[ "$CHANNEL" == "rc" ]]; then
  echo "[core] selecting strict latest RC…"
  v="$(pick_latest_with_suffix "$PHP_CORE_PKG" "RC")"
  if [[ -z "${v:-}" ]]; then
    echo "[error] No RC version found for $PHP_CORE_PKG"
    dump_versions_for_debug "$PHP_CORE_PKG"
    exit 1
  fi
  require_exact "$PHP_CORE_PKG" "$v"

elif [[ "$CHANNEL" == "beta" ]]; then
  echo "[core] selecting strict latest BETA…"
  v="$(pick_latest_with_suffix "$PHP_CORE_PKG" "beta")"
  if [[ -z "${v:-}" ]]; then
    echo "[error] No beta version found for $PHP_CORE_PKG"
    dump_versions_for_debug "$PHP_CORE_PKG"
    exit 1
  fi
  require_exact "$PHP_CORE_PKG" "$v"

else
  if [[ "$TAG" =~ ^[0-9][0-9A-Za-z\.\-\+]*$ ]]; then
    echo "[core] exact version requested: ${TAG}"
    require_exact "$PHP_CORE_PKG" "$TAG"
  else
    echo "[core] installing stable for ${PHP_CORE_PKG}"
    composer require "$PHP_CORE_PKG" --no-interaction --no-progress
  fi
fi

echo "[done] ${PHP_CORE_PKG} pinned based on RELEASE_TAG='${RELEASE_TAG}'"
