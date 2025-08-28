#!/usr/bin/env bash
set -euo pipefail

: "${WORKDIR:=/app}"
cd "$WORKDIR"

export COMPOSER_ALLOW_SUPERUSER=1

if [ ! -f composer.lock ]; then
  composer update --no-interaction --no-progress
fi

PKG="${PHP_CORE_PKG:-statsig/statsig-php-core}"

TAG_RAW="${RELEASE_TAG:-stable}"
TAG="$(echo "$TAG_RAW" | tr '[:upper:]' '[:lower:]' | sed 's/^v//')"

composer config prefer-stable true

CHANNEL="stable"
if [[ "$TAG" == "rc" || "$TAG" =~ -rc(\.|$) ]]; then
  CHANNEL="rc"
elif [[ "$TAG" == "beta" || "$TAG" =~ -beta(\.|$) ]]; then
  CHANNEL="beta"
fi

pick_latest_with_suffix() {

  local pkg="$1" suf="$2"
  composer show "$pkg" --all --format=json \
    | jq -r --arg suf "$suf" '
        .versions[]
        | select(test("(?i)-\($suf)(\\.|$)"))
      ' \
    | head -n 1
}

if [[ "$CHANNEL" == "rc" ]]; then
  echo "Selecting strict latest RC for $PKG ..."
  v="$(pick_latest_with_suffix "$PKG" "RC")"
  if [[ -z "${v:-}" || "$v" == "null" ]]; then
    echo "No RC version found for $PKG"; exit 1
  fi
  echo "Requiring $PKG:$v"
  composer require "$PKG:$v" --no-interaction --no-progress

elif [[ "$CHANNEL" == "beta" ]]; then
  echo "Selecting strict latest BETA for $PKG ..."
  v="$(pick_latest_with_suffix "$PKG" "beta")"
  if [[ -z "${v:-}" || "$v" == "null" ]]; then
    echo "No beta version found for $PKG"; exit 1
  fi
  echo "Requiring $PKG:$v"
  composer require "$PKG:$v" --no-interaction --no-progress

else
  if [[ "$TAG" =~ ^[0-9][0-9A-Za-z\.\-\+]*$ ]]; then
    echo "Requiring $PKG:$TAG (exact)"
    composer require "$PKG:$TAG" --no-interaction --no-progress
  else
    echo "Requiring $PKG (stable)"
    composer require "$PKG" --no-interaction --no-progress
  fi
fi
