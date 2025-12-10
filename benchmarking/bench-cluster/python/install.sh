#!/usr/bin/env bash
set -euo pipefail

PKG="statsig-python-core"
TAG="$RELEASE_TAG"  

pip install --no-cache-dir packaging

version_json=$( \
  curl -s https://pypi.org/pypi/statsig-python-core/json \
  | jq -r '.releases | to_entries | sort_by(.value[0].upload_time) | .[].key' \
)


latest_beta=$(echo "$version_json" | grep 'b' | tail -1)
latest_rc=$(echo "$version_json" | grep 'rc' | tail -1)
latest_prod=$(echo "$version_json" | grep -E '^[0-9]+\.[0-9]+\.[0-9]+$' | tail -1)

echo "== Found Versions =="
echo "latest_beta: $latest_beta"
echo "latest_rc: $latest_rc"
echo "latest_prod: $latest_prod"
echo "===================="


if [ "$TAG" == "rc" ]; then
  pip install --no-cache-dir "$PKG==$latest_rc"
elif [ "$TAG" == "beta" ]; then
  pip install --no-cache-dir "$PKG==$latest_beta"
else
  pip install --no-cache-dir "$PKG==$latest_prod"
fi


version=$(pip show statsig-python-core | grep Version | awk '{print $2}')
echo "Installed statsig-python-core version: $version"

