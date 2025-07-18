#!/bin/bash

if [ "$SDK_VARIANT" = "core" ]; then
  npx tsx bench-core.mts
elif [ "$SDK_VARIANT" = "legacy" ]; then
  npx tsx bench-legacy.ts
else
  echo "SDK_VARIANT must be either 'core' or 'legacy'"
  exit 1
fi
