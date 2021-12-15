#!/usr/bin/env sh

set -eux

if [ ! -f /tmp/rootfs/usr ]; then
  set +e
  # tar may throw a error like "tar: file changed as we read it".
  # This is most likely due to the new output package in tmp directory.
  # We ignore this error explicitly since we have excluded tmp directory.
  tar -cpzf /tmp/rootfs.tgz --warning=no-file-changed --exclude=./tmp --exclude=./init-rootfs.sh --one-file-system -C / .
  exitcode=$?
  # exitcode 1 means "Some files differ", ignore it.
  if [ "$exitcode" != "0" ] && [ "$exitcode" != "1" ]; then
    exit "$exitcode"
  fi
  set -e
  tar -xzf /tmp/rootfs.tgz -C /tmp/rootfs
fi
