#!/bin/sh
# Disposable identity-drift fault: real CLI list, then a changed identity in the next probe.
for arg in "$@"; do
    if [ "$arg" = ps ]; then touch /tmp/list-identity-changed; fi
    if [ "$arg" = info ] && [ -f /tmp/list-identity-changed ]; then
        printf '%s\n' '{"id":"changed-fixture-identity","os":"linux","security":[]}'
        exit 0
    fi
done
exec /usr/bin/docker "$@"
