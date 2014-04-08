setlocal

set name=atxpkg
set PYTHONPATH=../pylib;../libsh

rd /s /q build
rd /s /q dist

python setup.py install --prefix=dist

rd /s /q pkg
md pkg
md pkg\%name%
cp -av dist/* pkg/%name%/
cp atxpkg_backup pkg/.atxpkg_backup

;rem rd /s /q build
;rem rd /s /q dist

set /p version=<.version
rm .version

awk "BEGIN {print strftime(\"%%Y%%m%%d%%H%%M%%S\")}" >.datetime
set /p datetime=<.datetime
rm .datetime

set pkg_fn=%name%-%version%a%datetime%.atxpkg.zip

rm %pkg_fn%

cd pkg
zip -r ../%pkg_fn% .
cd ..

;rem rd /s /q pkg

pscp %pkg_fn% radek@podgorny.cz:public_html/atxpkg/
