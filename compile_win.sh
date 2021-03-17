#!/bin/sh
set -e -x

rm -rf build dist

#pip install --no-cache-dir -i http://10.0.2.2:3141/root/pypi/+simple/ --trusted-host 10.0.2.2 --upgrade pip  # otherwise editable install mode for atxpylib does not work
pip install --no-cache-dir -i http://10.0.2.2:3141/root/pypi/+simple/ --trusted-host 10.0.2.2 -r requirements.txt
#pip install --no-cache-dir -i http://10.0.2.2:3141/root/pypi/+simple/ --trusted-host 10.0.2.2 "pyinstaller<4" "setuptools<45"
pip install --no-cache-dir -i http://10.0.2.2:3141/root/pypi/+simple/ --trusted-host 10.0.2.2 cx-freeze
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

#rm -rf dist/PyQt5/Qt/qml
#rm -rf dist/PyQt5/Qt/bin/Qt5WebEngine*.* dist/PyQt5/Qt/bin dist/PyQt5/Qt/qml dist/PyQt5/Qt/resources dist/PyQt5/Qt/translations
#rm -rf dist/lib/PyQt5/Qt/bin/Qt5WebEngine*.* dist/lib/PyQt5/Qt/bin dist/lib/PyQt5/Qt/qml dist/lib/PyQt5/Qt/resources dist/lib/PyQt5/Qt/translations
#rm -rf dist/PySide2/*.exe dist/PySide2/Qt*WebEngine*.* dist/PySide2/Qt*Qml*.* dist/PySide2/Qt*3D*.* dist/PySide2/Qt*Quick*.* dist/PySide2/examples dist/PySide2/qml dist/PySide2/support dist/PySide2/translations

#cp -av c:/python37/lib/site-packages/dateutil dist/  # fix for pyinstaller/arrow/dateutil/whatever... bug
