#!/bin/bash
set -e -x

export name=atxpkg
export pkgname=${name}
export pkgrel=1

git submodule update --recursive --init

podman run --rm -it -v go_cache:/go/pkg/mod -v .:/xxx docker.io/golang:1.21.3 /bin/sh -c "cd /xxx; make windows_console"

rm -rf __pkg
mkdir __pkg
mkdir -p __pkg/${name}
cp -av *.exe __pkg/${name}/

if [ -d pkg_data ]; then
  cp -rv pkg_data/* __pkg/
fi

if [ -f atxpkg_backup ]; then
  cp -av atxpkg_backup __pkg/.atxpkg_backup
fi

if [ "$1" == "" ]; then
  export datetime=`gawk "BEGIN {print strftime(\"%Y%m%d%H%M%S\")}"`
  echo "devel version $datetime"
  export pkgname=${pkgname}.dev
  export version=$datetime
  export upload=atxpkg@atxpkg-dev.asterix.cz:atxpkg/
  export upload=scp://atxpkg@atxpkg-dev.asterix.cz:2224/atxpkg/
elif [ "$1" == "release" ]; then
  export version=`git describe --tags --abbrev=0`
  export version=${version:1}
  echo "release version $version"
  export upload=atxpkg@atxpkg.asterix.cz:atxpkg/
  export upload=scp://atxpkg@atxpkg.asterix.cz:2225/atxpkg/
else
  echo "unknown parameter!"
  exit
fi

#export sshopts='-i ./id_rsa_atxpkg -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null'
export sshopts='-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null'

export pkg_fn=${pkgname}-${version}-${pkgrel}.atxpkg.zip

rm -rf $pkg_fn

cd __pkg
zip -r ../$pkg_fn .
cd ..

rm -rf __pkg

#rsync -avP $pkg_fn $upload
scp -B ${sshopts} ${pkg_fn} ${upload}

echo "DONE: ${pkg_fn}"
