#!/bin/sh
# Automatic linker selector for asusctl:
# Priority 1: mold
# Priority 2: lld
# Priority 3: standard ld (fallback)

if command -v mold >/dev/null 2>&1; then
    exec cc -fuse-ld=mold "$@"
elif command -v lld >/dev/null 2>&1 || command -v ld.lld >/dev/null 2>&1; then
    exec cc -fuse-ld=lld "$@"
else
    exec cc "$@"
fi
