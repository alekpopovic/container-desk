#!/bin/sh
# Disposable cancellation fixture. Capability probes use the real isolated Engine.
for arg in "$@"; do
    if [ "$arg" = ps ]; then
        touch /tmp/list-inflight
        exec sleep 30
    fi
done
exec /usr/bin/docker "$@"
