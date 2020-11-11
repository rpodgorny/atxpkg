#!/bin/sh
set -e -x

export name=atxpkg
export pkgname=${name}
export pkgrel=1

git submodule update --recursive --init

rm -rf build dist

#pipenv --rm || true
#pipenv install --dev
#pipenv run python setup.py install --prefix=dist
./vagcompile.sh

rm -rf dist/PyQt5/Qt/bin/Qt5WebEngine*.* dist/PyQt5/Qt/bin dist/PyQt5/Qt/qml dist/PyQt5/Qt/resources dist/PyQt5/Qt/translations
rm -rf dist/lib/PyQt5/Qt/bin/Qt5WebEngine*.* dist/lib/PyQt5/Qt/bin dist/lib/PyQt5/Qt/qml dist/lib/PyQt5/Qt/resources dist/lib/PyQt5/Qt/translations
rm -rf dist/PySide2/*.exe dist/PySide2/Qt*WebEngine*.* dist/PySide2/Qt*Qml*.* dist/PySide2/Qt*3D*.* dist/PySide2/Qt*Quick*.* dist/PySide2/examples dist/PySide2/qml dist/PySide2/support dist/PySide2/translations

rm -rf pkg
mkdir pkg
mkdir -p pkg/${name}
cp -av dist/* pkg/${name}/

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
  export pkgname=${pkgname}.dev
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

export pkg_fn=${pkgname}-${version}-${pkgrel}.atxpkg.zip

rm -rf $pkg_fn

cd pkg
zip -r ../$pkg_fn .
cd ..

rm -rf pkg

rsync -avP $pkg_fn $upload

echo "DONE: ${pkg_fn}"
