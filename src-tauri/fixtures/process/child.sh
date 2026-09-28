#!/bin/sh
# Synthetic test program; production never invokes this fixture.
scenario=$1
fixture_dir=$2
printf '%s\n' "$$" > "$fixture_dir/pid"
case "$scenario" in
  noisy)
    printf partial
    i=0
    while [ "$i" -lt 2000 ]; do
      printf 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx' >&2
      i=$((i+1))
    done
    printf tail
    exit 7
    ;;
  sleep) exec /bin/sleep 30 ;;
  stdout_limit) while :; do printf XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX; done ;;
  stderr_limit) while :; do printf XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX >&2; done ;;
  empty)
    if [ -t 0 ] || [ -t 1 ] || [ -t 2 ]; then exit 9; fi
    if read -r ignored; then exit 8; fi
    exit 0
    ;;
  exit) exit 0 ;;
  *) exit 10 ;;
esac
