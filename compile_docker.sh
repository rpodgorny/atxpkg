#!/bin/sh
set -e -x

trap 'rm -rf "$TMPDIR"' EXIT
TMPDIR=$(mktemp -d -p /var/tmp) || exit 1

cp -av . ${TMPDIR}
podman run --rm -it -v ${TMPDIR}:/x -v cargo_cache:/usr/local/cargo/registry docker.io/rpodgorny/rust:1.77.2-1 /bin/sh -c "cd /x; ./compile_win.sh"
cp -av ${TMPDIR}/target ./target_xxx
