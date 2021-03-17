#!/bin/sh
set -e -x

rm -rf MERGE
mkdir MERGE
cd MERGE
for i in ../dist/*; do
  unzip -o $i/base_library.zip
done
zip -r ../___base_library.zip .
rm -rf ./*
for i in ../dist/*; do
  cp -av $i/* ./
done
cd -
rm -rf dist
mv ___base_library.zip MERGE/base_library.zip
mv MERGE dist
