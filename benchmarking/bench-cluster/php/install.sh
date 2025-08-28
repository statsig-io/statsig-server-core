#!/usr/bin/env bash
set -euo pipefail

: "${WORKDIR:=/app}"
: "${PHP_CORE_PKG:=statsig/statsig-php-core}"
: "${RELEASE_TAG:=stable}"
: "${FFI_TARGET:=linux-gnu-x86_64-shared}"
export COMPOSER_ALLOW_SUPERUSER=1

log() { printf "%s\n" "$*" >&2; }

install_jq_if_missing() {
  command -v jq >/dev/null 2>&1 && return 0
  if command -v apk >/dev/null 2>&1; then apk add --no-cache jq || true
  elif command -v apt-get >/dev/null 2>&1; then apt-get update -y && apt-get install -y jq || true
  elif command -v yum >/dev/null 2>&1; then yum install -y jq || true
  fi
  command -v jq >/dev/null 2>&1 || { log "jq missing"; exit 1; }
}

ensure_composer_json() {
  [ -f composer.json ] || composer init -n --name temp/app >/dev/null
}

normalize_channel() {
  local tag; tag="$(echo "$1" | tr '[:upper:]' '[:lower:]' | sed 's/^v//')"
  case "$tag" in
    rc|*-rc|*-rc.*) echo rc ;;
    beta|*-beta|*-beta.*) echo beta ;;
    ""|stable) echo stable ;;
    *) echo exact ;;
  esac
}

list_versions_sorted() {
  composer show "$PHP_CORE_PKG" --all --format=json | jq -r '.versions[]' | grep -v '^dev-' | sort -V
}

filter_by_channel() {
  case "$1" in
    stable) grep -E '^[0-9]+\.[0-9]+\.[0-9]+$' || true ;;
    rc)     grep -E 'rc\.' || true ;;
    beta)   grep -E 'beta\.' || true ;;
    *)      cat ;;
  esac
}

try_install_version() {
  local ver="$1"
  log ""; log ">>> Trying ${PHP_CORE_PKG}:${ver}"
  rm -f composer.lock
  if composer require -W --no-interaction --no-progress "${PHP_CORE_PKG}:${ver}"; then
    echo "$ver" > .selected_php_core_version
    log "Installed ${PHP_CORE_PKG}@${ver}"
    return 0
  else
    log "Failed ${ver}"; composer why-not "${PHP_CORE_PKG}" "${ver}" || true
    return 1
  fi
}

force_sync_ffi() {
  local ver="$1"
  local pkg_dir="vendor/statsig/statsig-php-core"
  local res_dir="${pkg_dir}/resources"
  mkdir -p "$res_dir"

  local targets=("$FFI_TARGET" "linux-gnu-x86_64-shared" "linux-musl-x86_64-shared" "centos7-x86_64-unknown-linux-gnu-shared")
  local ok=0

  for tgt in "${targets[@]}"; do
    local zip="statsig-ffi-${ver}-${tgt}.zip"
    local url="https://github.com/statsig-io/statsig-php-core/releases/download/${ver}/${zip}"
    log "FFI try: ${url}"
    if curl -fsI "$url" >/dev/null 2>&1; then
      curl -fsSL -o "${res_dir}/${zip}" "$url"
      unzip -o "${res_dir}/${zip}" -d "${res_dir}" >/dev/null
      curl -fsSL -o "${res_dir}/statsig_ffi.h" "https://github.com/statsig-io/statsig-php-core/releases/download/${ver}/statsig_ffi.h"
      ok=1
      break
    fi
  done

  if [ "$ok" -ne 1 ]; then
    log "No FFI asset found for ${ver} on known targets. Keeping post-install fallback."
    return 0
  fi

  [ -f "${res_dir}/libstatsig_ffi.so" ] || [ -f "${res_dir}/libstatsig_ffi.dylib" ] || { log "FFI library missing after download"; exit 1; }
  [ -f "${res_dir}/statsig_ffi.h" ] || { log "FFI header missing after download"; exit 1; }

  log "FFI synced to ${ver} under ${res_dir}"
}

main() {
  cd "$WORKDIR" >/dev/null 2>&1 || { log "WORKDIR $WORKDIR not found"; exit 1; }
  install_jq_if_missing
  ensure_composer_json

  log ""; log "-- Env Debug --"
  php -v || true; composer --version || true
  log "WORKDIR: $WORKDIR"; log "PACKAGE: $PHP_CORE_PKG"; log "RELEASE_TAG: $RELEASE_TAG"; log "FFI_TARGET: $FFI_TARGET"
  log "--------------"

  local channel ver versions candidates success=0
  if [[ "$RELEASE_TAG" =~ ^[0-9]+\.[0-9]+\.[0-9]+ ]]; then
    ver="$RELEASE_TAG"
    try_install_version "$ver" || exit 1
    force_sync_ffi "$ver"
    composer show "$PHP_CORE_PKG"
    exit 0
  fi

  channel="$(normalize_channel "$RELEASE_TAG")"
  if [ "$channel" = exact ]; then
    ver="$RELEASE_TAG"
    try_install_version "$ver" || exit 1
    force_sync_ffi "$ver"
    composer show "$PHP_CORE_PKG"
    exit 0
  fi

  versions="$(list_versions_sorted)"
  [ -n "$versions" ] || { log "No versions from composer show"; exit 1; }
  candidates="$(printf "%s\n" "$versions" | filter_by_channel "$channel")"
  [ -n "$candidates" ] || { log "No ${channel} versions available"; exit 1; }

  log "Candidates (${channel}):"; printf "%s\n" "$candidates" >&2

  while read -r v; do
    [ -z "$v" ] && continue
    if try_install_version "$v"; then ver="$v"; success=1; break; fi
  done < <(printf "%s\n" "$candidates" | tac)

  [ "$success" -eq 1 ] || { log "Install failed for all ${channel} candidates"; exit 1; }

  force_sync_ffi "$ver"
  log ""; composer show "$PHP_CORE_PKG"
}

main "$@"
