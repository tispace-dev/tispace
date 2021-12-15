#!/usr/bin/env sh

set -eux

if [ ! -f /tmp/rootfs/usr ]; then
  set +e
  tar -cpzf /tmp/rootfs.tgz --warning=no-file-changed --exclude=./tmp --exclude=./init-rootfs.sh --one-file-system -C / .
  exitcode=$?
  if [ "$exitcode" != "0" ] && [ "$exitcode" != "1" ]; then
    exit "$exitcode"
  fi
  set -e
  tar -xzf /tmp/rootfs.tgz -C /tmp/rootfs
fi
