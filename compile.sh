#!/bin/bash
set -e -x

export name=atxpkg
export pkgname=${name}
export pkgrel=1

git submodule update --recursive --init

rm -rf build dist

#./compile_vagrant.sh
./compile_docker.sh

rm -rf pkg
mkdir pkg
mkdir -p pkg/${name}
cp -av dist/* pkg/${name}/

rm -rf build dist

if [ -d pkg_data ]; then
  cp -rv pkg_data/* pkg/
fi

if [ -f atxpkg_backup ]; then
  cp -av atxpkg_backup pkg/.atxpkg_backup
fi

if [ "$1" == "" ]; then
  export datetime=`gawk "BEGIN {print strftime(\"%Y%m%d%H%M%S\")}"`
  echo "devel version $datetime"
  export pkgname=${pkgname}.dev
  export version=$datetime
  export upload=scp://atxpkg@atxpkg-dev.asterix.cz:2224/atxpkg/
elif [ "$1" == "release" ]; then
  export version=`git describe --tags --abbrev=0`
  export version=${version:1}
  echo "release version $version"
  export upload=scp://atxpkg@atxpkg.asterix.cz:2224:/atxpkg/
else
  echo "unknown parameter!"
  exit
fi

#export sshopts='-i ./id_rsa_atxpkg -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null'
export sshopts='-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null'

export pkg_fn=${pkgname}-${version}-${pkgrel}.atxpkg.zip

rm -rf $pkg_fn

cd pkg
zip -r ../$pkg_fn .
cd ..

rm -rf pkg

scp -B ${sshopts} $pkg_fn $upload

echo "DONE: ${pkg_fn}"
