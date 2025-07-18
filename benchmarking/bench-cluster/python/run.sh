#!/bin/bash

if [ "$SDK_VARIANT" = "core" ]; then
  python bench-core.py
elif [ "$SDK_VARIANT" = "legacy" ]; then
  python bench-legacy.py
else
  echo "SDK_VARIANT must be either 'core' or 'legacy'"
  exit 1
fi