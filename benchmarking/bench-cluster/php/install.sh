#!/usr/bin/env bash
set -euo pipefail

PKG="${PHP_CORE_PKG:-statsig/statsig-php-core}"

TAG="$(echo "${RELEASE_TAG:-stable}" | tr '[:upper:]' '[:lower:]')"

: "${WORKDIR:=/app}"
cd "$WORKDIR"

composer config prefer-stable true

case "$TAG" in
  rc)
    echo "Requiring $PKG (rc channel)…"
    composer require "$PKG:*@rc" --no-interaction --no-progress
    ;;
  beta)
    echo "Requiring $PKG (beta channel)…"
    composer require "$PKG:*@beta" --no-interaction --no-progress
    ;;
  *)
    if [[ "$TAG" =~ [0-9] ]]; then
      echo "Requiring $PKG:$TAG (exact)…"
      composer require "$PKG:$TAG" --no-interaction --no-progress
    else
      echo "Requiring $PKG (stable)…"
      composer require "$PKG" --no-interaction --no-progress
    fi
    ;;
esac
