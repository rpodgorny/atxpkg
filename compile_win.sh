#!/bin/sh
set -e -x

PYPI_CACHE="host.containers.internal"

rm -rf build dist

pip install --no-cache-dir -i http://${PYPI_CACHE}:3141/root/pypi/+simple/ --trusted-host ${PYPI_CACHE} "pyinstaller==5.8.0"
pip install --no-cache-dir -i http://${PYPI_CACHE}:3141/root/pypi/+simple/ --trusted-host ${PYPI_CACHE} -r requirements.txt

python version.py | tee __v.txt
pyinstaller --noconfirm --clean --noupx --version-file __v.txt atxpkg.py
rm -rf __v.txt

./merge_dist.sh

#cp -av api-ms-win-core-path-l1-1-0.dll dist/  # fix for python3.9 not being supported on windows7
