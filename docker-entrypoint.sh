#!/bin/sh
# Shared development entrypoint for both dev images.
#
# Two ownership problems to solve before the real process starts, both because
# the container runs as the host user so that anything it writes into the bind
# mount stays owned by you:
#
#   1. Named volumes (target/, the cargo registry, node_modules, the bun cache)
#      are created root-owned. Without a chown the first write fails with
#      EACCES — cargo cannot create /app/target, bun cannot populate
#      node_modules.
#   2. The dev data directory lives inside the bind-mounted repo so it is
#      inspectable from the host. It has to be created here, as root, and then
#      handed over. If it were instead declared as a named volume mounted
#      inside the bind mount, Docker would create the host directory itself,
#      root-owned, before this script ever runs — and nothing could fix it.
#
# Services declare what they need via DEV_CHOWN_PATHS (space-separated).
set -e

for path in ${DEV_CHOWN_PATHS:-}; do
    # mkdir rather than skip-if-missing: the dev data directory is expected not
    # to exist on a fresh clone, and creating it here is what keeps it out of
    # Docker's hands.
    mkdir -p "$path" 2>/dev/null || true
    [ -e "$path" ] || continue
    # Skip the recursive chown when ownership is already correct — on a
    # populated node_modules that walk is tens of thousands of files on every
    # container start.
    if [ "$(stat -c '%u' "$path")" != "${DEV_UID}" ]; then
        chown -R "${DEV_UID}:${DEV_GID}" "$path" 2>/dev/null || true
    fi
done

exec gosu "${DEV_UID}:${DEV_GID}" "$@"
