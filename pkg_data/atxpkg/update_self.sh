#!/bin/sh
set -e -x

urlbase=https://atxpkg.asterix.cz
pkg=atxpkg
ver=4.0-1
fn=$pkg-$ver.atxpkg.zip
#urlbase=https://atxpkg-dev.asterix.cz
#pkg=atxpkg.dev
#fn=atxpkg.dev-20220203181342-1.atxpkg.zip

mkdir -p /cygdrive/c/tmp
cd /cygdrive/c/tmp

rm -rf atxpkg.tmp
mkdir atxpkg.tmp
cd atxpkg.tmp

wget --no-check-certificate $urlbase/$fn

wget --no-check-certificate https://atxpkg.asterix.cz/7za.exe

cp /cygdrive/c/atxpkg/installed.json ./ || true
chmod a+r installed.json || true

chmod a+x 7za.exe
./7za.exe x $fn
rm -rf /cygdrive/c/atxpkg/* || true
cp -rv atxpkg c:\\
#rm -rf atxpkg
mkdir -p /cygdrive/c/atxpkg/cache
cp $fn /cygdrive/c/atxpkg/cache/
#rm $fn
#rm 7za.exe

cp installed.json /cygdrive/c/atxpkg/ || true
#rm installed.json || true

cd -
rm -rf atxpkg.tmp

/cygdrive/c/atxpkg/atxpkg install $pkg-$ver --yes --force

cd /cygdrive/c/atxpkg
./add_to_path.bat
cd -
