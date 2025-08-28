#!/usr/bin/env bash
set -euo pipefail

CHANNEL="${RELEASE_TAG:-}"

case "$CHANNEL" in
  "" )
    echo "RELEASE_TAG empty -> using 'stable'"
    CHANNEL="stable"
    ;;
  "beta"|"rc")
    echo "Resolved channel: $CHANNEL"
    ;;
  * )
    echo "ERROR: invalid RELEASE_TAG='$CHANNEL'. Must be empty, 'beta', or 'rc'." >&2
    exit 1
    ;;
esac

fetch_version() {
  local ch="$1"
  curl -s https://crates.io/api/v1/crates/statsig-rust/versions \
  | jq -r --arg ch "$ch" '
      .versions
      | map(select(.yanked == false))
      | if $ch == "rc" then
          map(select(.num | test("-rc")))
        elif $ch == "beta" then
          map(select(.num | test("-beta")))
        else
          map(select(.num | test("-(rc|beta)") | not))
        end
      | sort_by(.created_at) | reverse
      | (.[0].num // empty)
    '
}

VER="$(fetch_version "$CHANNEL")"
if [ -z "$VER" ]; then
  echo "ERROR: no version found for channel '$CHANNEL'" >&2
  exit 1
fi

echo "Resolved channel=$CHANNEL -> statsig-rust@$VER"
echo "$VER" > statsig-rust-version.txt

cargo add "statsig-rust@=${VER}"

echo "----- VERIFY -----"
grep -n "statsig-rust" Cargo.toml || true
grep -n 'name = "statsig-rust"' Cargo.lock -n -A2 || true
