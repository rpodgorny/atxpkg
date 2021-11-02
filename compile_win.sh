#!/bin/sh
set -e -x

PYPI_CACHE="172.17.0.1"
PYTHON_SITE_PACKAGES="/wine/drive_c/Python39/Lib/site-packages"

rm -rf build dist

pip install --no-cache-dir -i http://${PYPI_CACHE}:3141/root/pypi/+simple/ --trusted-host ${PYPI_CACHE} -r requirements.txt
pip install --no-cache-dir -i http://${PYPI_CACHE}:3141/root/pypi/+simple/ --trusted-host ${PYPI_CACHE} cx-freeze
#python atxmanager/version.py >__v.txt
#pyinstaller --noconfirm --clean --noupx --windowed -n 4to6serverw 4to6server.py
#pyinstaller --noconfirm --clean --noupx 4to6server.py
#pyinstaller --noconfirm --clean --noupx --windowed -n 4to6clientw 4to6client.py
#pyinstaller --noconfirm --clean --noupx 4to6client.py
#pyinstaller --noconfirm --clean --noupx --version-file __v.txt --add-data "atxmanager/templates;templates" --add-data "atxmanager/static;static" --add-data "captions.cs;." -n manager_nw manager.py
#pyinstaller --noconfirm --clean --noupx --add-data "test;test" tests.py
#rm -rf __v.txt
python setup.py install --prefix=dist

#./merge_dist.sh

#cp -av c:/python37/lib/site-packages/dateutil dist/  # fix for pyinstaller/arrow/dateutil/whatever... bug

chmod -R a+rwX .  # this is being run under root in docker - so make it possible to delete this dir from outside by regular user
