#!/bin/sh
set -e -x

export name=atxpkg
export pkgname=${name}
export pkgrel=1

rm -rf venv
python -m venv venv
export VIRTUAL_ENV=venv
export PATH="venv/bin:venv/Scripts:$PATH"
pip install cx_freeze
#pip install -e atxpylib

rm -rf build dist

if [ -f requirements.txt ]; then
  pip install -r requirements.txt
fi

python setup.py install --prefix=dist
rm -rf dist/PyQt5/Qt/bin/Qt5WebEngine*.* dist/PyQt5/Qt/qml

# wtf?
cp -v c:/windows/system32/vcruntime140.dll dist/

rm -rf venv

rm -rf pkg
mkdir pkg
mkdir -p pkg/$name
cp -av dist/* pkg/$name/

if [ -d pkg_data ]; then
  cp -rv pkg_data/* pkg/
fi

if [ -f atxpkg_backup ]; then
  cp -av atxpkg_backup pkg/.atxpkg_backup
fi

rm -rf build dist

if [ "$1" == "" ]; then
  export datetime=`gawk "BEGIN {print strftime(\"%Y%m%d%H%M%S\")}"`
  echo "devel version $datetime"
  export name=${name}.dev
  export version=$datetime
  export upload=atxpkg@atxpkg-dev.asterix.cz:atxpkg/
elif [ "$1" == "release" ]; then
  export version=`git describe --tags --abbrev=0`
  export version=${version:1}
  echo "release version $version"
  export upload=atxpkg@atxpkg.asterix.cz:atxpkg/
else
  echo "unknown parameter!"
  exit
fi

export pkg_fn=${name}-${version}-${pkgrel}.atxpkg.zip

rm -rf $pkg_fn

cd pkg
zip -r ../$pkg_fn .
cd ..

rm -rf pkg

rsync -avP $pkg_fn $upload
