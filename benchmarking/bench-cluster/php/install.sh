#!/usr/bin/env bash
# install.sh — always install latest -beta / -rc (or stable) of statsig/statsig-php-core
set -euo pipefail

: "${WORKDIR:=/app}"
: "${PHP_CORE_PKG:=statsig/statsig-php-core}"
: "${RELEASE_TAG:=stable}"
export COMPOSER_ALLOW_SUPERUSER=1

cd "$WORKDIR" >/dev/null 2>&1 || { echo "WORKDIR $WORKDIR not found"; exit 1; }

if [ ! -f composer.json ]; then
  composer init -n --name temp/app >/dev/null
fi

composer config prefer-stable true || true

TAG="$(echo "$RELEASE_TAG" | tr '[:upper:]' '[:lower:]' | sed 's/^v//')"

CHANNEL="stable"
if [[ "$TAG" == "rc" || "$TAG" =~ -rc(\.|$) ]]; then
  CHANNEL="rc"
elif [[ "$TAG" == "beta" || "$TAG" =~ -beta(\.|$) ]]; then
  CHANNEL="beta"
fi

REQ="*"
case "$CHANNEL" in
  rc)   REQ="*@RC" ;;
  beta) REQ="*@beta" ;;
  *)    REQ="*" ;;
esac

if composer show "$PHP_CORE_PKG" >/dev/null 2>&1; then
  composer require --no-interaction --no-progress -W "$PHP_CORE_PKG:$REQ"
  composer update   --no-interaction --no-progress "$PHP_CORE_PKG"
else
  composer require --no-interaction --no-progress -W "$PHP_CORE_PKG:$REQ"
fi

composer show "$PHP_CORE_PKG"
