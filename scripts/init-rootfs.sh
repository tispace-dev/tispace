#!/usr/bin/env sh

set -eux

# If rootfs-initing exists, it means the rootfs was incomplete.
# We need to clean the rootfs and try to initialize it again.
if [ -f /tmp/rootfs/rootfs-initing ]; then
  find /tmp/rootfs -mindepth 1 -not -path /tmp/rootfs/rootfs-initing -delete
  rm -f /tmp/rootfs/rootfs-initing
fi

if [ ! -d /tmp/rootfs/usr ]; then
  touch /tmp/rootfs/rootfs-initing
  set +e
  # tar may throw an error like "tar: file changed as we read it".
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
  rm -f /tmp/rootfs/rootfs-initing
fi
