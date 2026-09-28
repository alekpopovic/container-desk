#!/bin/sh
# Synthetic fault process, used only inside the disposable SSH server.
case "$0" in
  *docker-flood) exec head -c 20000 /dev/zero ;;
  *docker-hang) exec sleep 30 ;;
  *) exit 1 ;;
esac
