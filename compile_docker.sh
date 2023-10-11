#!/bin/sh
set -e -x

trap 'rm -rf "$TMPDIR"' EXIT
TMPDIR=$(mktemp -d -p /var/tmp) || exit 1

cp -av . ${TMPDIR}
podman run --rm -it -v ${TMPDIR}:/x docker.io/rpodgorny/winepybuilder:3.11.6-1 /bin/bash -c "cd /x; ./compile_win.sh"
cp -av ${TMPDIR}/dist ./
