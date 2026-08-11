#!/bin/sh
# Shared development entrypoint for both dev images.
#
# Docker creates named volumes owned by root, but the processes inside run as
# the host user so that anything reaching the bind mount stays owned by you.
# Every volume mountpoint therefore has to be handed over before privileges are
# dropped, or the first write fails with EACCES — cargo cannot create
# /app/target, redb cannot open /app/data, and bun cannot populate node_modules
# or its install cache.
#
# Services declare what they need via DEV_CHOWN_PATHS (space-separated).
set -e

for path in ${DEV_CHOWN_PATHS:-}; do
    [ -e "$path" ] || continue
    # Skip the recursive chown when ownership is already correct — on a
    # populated node_modules that walk is tens of thousands of files on every
    # container start.
    if [ "$(stat -c '%u' "$path")" != "${DEV_UID}" ]; then
        chown -R "${DEV_UID}:${DEV_GID}" "$path" 2>/dev/null || true
    fi
done

exec gosu "${DEV_UID}:${DEV_GID}" "$@"
