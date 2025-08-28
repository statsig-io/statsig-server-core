#!/usr/bin/env bash
# install.sh — Install the highest compatible stable/rc/beta (or exact) for statsig/statsig-php-core
# Usage examples:
#   RELEASE_TAG=stable ./install.sh
#   RELEASE_TAG=rc     ./install.sh
#   RELEASE_TAG=beta   ./install.sh
#   RELEASE_TAG=0.8.3-beta.2508280233 ./install.sh   # exact version
#
# ENV:
#   WORKDIR=/app (default)
#   PHP_CORE_PKG=statsig/statsig-php-core (default)
#   RELEASE_TAG=stable (default)

set -euo pipefail

: "${WORKDIR:=/app}"
: "${PHP_CORE_PKG:=statsig/statsig-php-core}"
: "${RELEASE_TAG:=stable}"
export COMPOSER_ALLOW_SUPERUSER=1

log() { printf "%s\n" "$*" >&2; }

install_jq_if_missing() {
  if command -v jq >/dev/null 2>&1; then return 0; fi
  log "jq not found. Attempting to install (apk/apt/yum)..."
  if command -v apk >/dev/null 2>&1; then
    apk add --no-cache jq || true
  elif command -v apt-get >/dev/null 2>&1; then
    apt-get update -y && apt-get install -y jq || true
  elif command -v yum >/dev/null 2>&1; then
    yum install -y jq || true
  fi
  if ! command -v jq >/dev/null 2>&1; then
    log "Failed to install jq automatically. Please install jq and re-run."
    exit 1
  fi
}

ensure_composer_json() {
  if [ ! -f composer.json ]; then
    composer init -n --name temp/app >/dev/null
  fi
}

normalize_channel() {
  local tag="$1"
  tag="$(echo "$tag" | tr '[:upper:]' '[:lower:]' | sed 's/^v//')"
  case "$tag" in
    rc|*-rc|*-rc.*)   echo "rc" ;;
    beta|*-beta|*-beta.*) echo "beta" ;;
    stable|"")        echo "stable" ;;
    *)                echo "exact" ;;   # looks like an exact version or non-standard tag
  esac
}

# Returns a newline-separated list of versions (highest last)
list_versions_sorted() {
  local pkg="$1"
  composer show "$pkg" --all --format=json \
    | jq -r '.versions[]' \
    | grep -v '^dev-' \
    | sort -V
}

# Filter versions by channel
# - stable: pure semver X.Y.Z
# - rc:     contains "rc."
# - beta:   contains "beta."
filter_by_channel() {
  local channel="$1"
  case "$channel" in
    stable) grep -E '^[0-9]+\.[0-9]+\.[0-9]+$' || true ;;
    rc)     grep -E 'rc\.' || true ;;
    beta)   grep -E 'beta\.' || true ;;
    *)      cat ;;  # no filter
  esac
}

try_install_version() {
  local pkg="$1" ver="$2"
  log ""
  log ">>> Trying ${pkg}:${ver}"
  rm -f composer.lock
  if composer require -W --no-interaction --no-progress "${pkg}:${ver}"; then
    log "Installed ${pkg}@${ver}"
    return 0
  else
    log "Failed to install ${ver}. Diagnostics:"
    composer why-not "${pkg}" "${ver}" || true
    return 1
  fi
}

main() {
  cd "$WORKDIR" >/dev/null 2>&1 || { log "WORKDIR $WORKDIR not found"; exit 1; }

  install_jq_if_missing
  ensure_composer_json

  log ""
  log "-- Env Debug --"
  php -v || true
  composer --version || true
  log "WORKDIR:      $WORKDIR"
  log "PACKAGE:      $PHP_CORE_PKG"
  log "RELEASE_TAG:  $RELEASE_TAG"
  log "--------------"

  # If RELEASE_TAG looks like an exact version (starts with digits), install it directly.
  if [[ "$RELEASE_TAG" =~ ^[0-9]+\.[0-9]+\.[0-9]+ ]]; then
    log "Installing exact version: ${RELEASE_TAG}"
    try_install_version "$PHP_CORE_PKG" "$RELEASE_TAG"
    composer show "$PHP_CORE_PKG"
    exit 0
  fi

  local channel
  channel="$(normalize_channel "$RELEASE_TAG")"
  log "Resolved channel: ${channel}"

  if [ "$channel" = "exact" ]; then
    # Not a recognized channel; treat as exact string (e.g., "0.8.1-rc.XXXX" or custom)
    try_install_version "$PHP_CORE_PKG" "$RELEASE_TAG"
    composer show "$PHP_CORE_PKG"
    exit 0
  fi

  log ""
  log "-- Resolving versions from Composer (channel: ${channel}) --"
  versions="$(list_versions_sorted "$PHP_CORE_PKG")"
  if [ -z "${versions:-}" ]; then
    log "No versions found via composer show. Check network/packagist."
    exit 1
  fi

  candidates="$(printf "%s\n" "$versions" | filter_by_channel "$channel")"
  if [ -z "${candidates:-}" ]; then
    log "No ${channel} versions available for ${PHP_CORE_PKG}."
    exit 1
  fi

  log "Candidates (${channel}):"
  printf "%s\n" "$candidates" >&2

  # Try from newest to oldest
  success=0
  while read -r v; do
    [ -z "$v" ] && continue
    if try_install_version "$PHP_CORE_PKG" "$v"; then
      success=1
      break
    fi
  done < <(printf "%s\n" "$candidates" | tac)

  if [ $success -ne 1 ]; then
    log "Could not install any ${channel} version (platform constraints?)."
    exit 1
  fi

  log ""
  composer show "$PHP_CORE_PKG"
}

main "$@"
