#!/usr/bin/env bash
set -euo pipefail

PKG="statsig-python-core"
TAG="$RELEASE_TAG"  

pip install --no-cache-dir packaging

if [ "$TAG" = "rc" ]; then
  RC_VER=$(
    python3 - <<'PY'
import json, urllib.request
from packaging.version import Version

url = "https://pypi.org/pypi/statsig-python-core/json"
d = json.load(urllib.request.urlopen(url))
rels = d.get("releases") or {}
rc = [
    Version(s) for s, fs in rels.items()
    if fs and any(not f.get("yanked", False) for f in fs)
    and (lambda v: v.is_prerelease and v.pre and v.pre[0] == "rc")(Version(s))
]
print(max(rc) if rc else "")
PY
  )

  if [ -n "$RC_VER" ]; then
    echo "Installing $PKG==$RC_VER (rc)"
    pip install --no-cache-dir "$PKG==$RC_VER"
  else
    echo "No rc found; falling back to --pre"
    pip install --no-cache-dir --pre "$PKG"
  fi
else
  pip install --no-cache-dir --pre "$PKG"
fi
