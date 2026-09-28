#!/bin/sh
# Static synthetic executable; test copies it into a private directory for the marker.
: > "${0%/*}/spawned"
printf 'OpenSSH_fixture\n' >&2
