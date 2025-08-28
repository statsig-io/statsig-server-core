#!/usr/bin/env bash
if [ ! -f composer.lock ]; then
  composer update --no-interaction --no-progress
fi

set -euo pipefail

PKG="${PHP_CORE_PKG:-statsig/statsig-php-core}"

TAG_RAW="${RELEASE_TAG:-stable}"
TAG="$(echo "$TAG_RAW" | tr '[:upper:]' '[:lower:]' | sed 's/^v//')"

: "${WORKDIR:=/app}"
cd "$WORKDIR"

export COMPOSER_ALLOW_SUPERUSER=1

composer config prefer-stable true

CHANNEL="stable"
if [[ "$TAG" == "rc" || "$TAG" =~ -rc(\.|$) ]]; then
  CHANNEL="rc"
elif [[ "$TAG" == "beta" || "$TAG" =~ -beta(\.|$) ]]; then
  CHANNEL="beta"
fi

if [[ "$CHANNEL" == "rc" ]]; then
  echo "Pin $PKG to RC channel (*@rc)"
  composer require "$PKG:*@rc" --no-interaction --no-progress --no-update
  composer update "$PKG" --with-dependencies --no-interaction --no-progress

elif [[ "$CHANNEL" == "beta" ]]; then
  echo "Pin $PKG to BETA channel (*@beta)"
  composer require "$PKG:*@beta" --no-interaction --no-progress --no-update
  composer update "$PKG" --with-dependencies --no-interaction --no-progress

else
  if [[ "$TAG" =~ ^[0-9][0-9a-zA-Z\.\-\+]*$ ]]; then
    echo "Pin $PKG to exact version: $TAG"
    composer require "$PKG:$TAG" --no-interaction --no-progress --no-update
    composer update "$PKG" --with-dependencies --no-interaction --no-progress
  else
    echo "Use stable for $PKG (leave constraint as *)"
    composer update "$PKG" --with-dependencies --no-interaction --no-progress
  fi
fi
