#!/bin/sh
set -e -x

PYPI_CACHE="172.17.0.1"
PYTHON_SITE_PACKAGES="/wine/drive_c/Python39/Lib/site-packages"

rm -rf build dist

pip install --no-cache-dir -i http://${PYPI_CACHE}:3141/root/pypi/+simple/ --trusted-host ${PYPI_CACHE} "pyinstaller==4.10"  # 5.0.1 is being barfed at by windows defender as containing virus (as of 2022-05-15)
pip install --no-cache-dir -i http://${PYPI_CACHE}:3141/root/pypi/+simple/ --trusted-host ${PYPI_CACHE} -r requirements.txt

pyinstaller --noconfirm --clean --noupx atxpkg.py

./merge_dist.sh

#cp -av c:/python37/lib/site-packages/dateutil dist/  # fix for pyinstaller/arrow/dateutil/whatever... bug

cp -av api-ms-win-core-path-l1-1-0.dll dist/  # fix for python3.9 not being supported on windows7

chmod -R a+rwX .  # this is being run under root in docker - so make it possible to delete this dir from outside by regular user
