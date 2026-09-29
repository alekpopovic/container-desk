#!/bin/sh
# Lab-only client configuration discovers the optional plugin on the isolated target.
exec env DOCKER_CONFIG=/tmp/compose-enabled-config /usr/bin/docker "$@"
