#!/bin/sh
# Inert POSIX remote-shell test target, not a Docker executable.
printf '%s\0' "$@"
