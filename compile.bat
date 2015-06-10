echo this is compile.bat v0.3

setlocal

set name=atxpkg
set pkgrel=2

rd /s /q build
rd /s /q dist

python setup_win.py install --prefix=dist

rd /s /q pkg
md pkg
md pkg\%name%
cp -av dist/* pkg/%name%/
cp atxpkg_backup pkg/.atxpkg_backup

cp add_to_path.bat pkg/%name%/
cp atxpkg_7za.exe pkg/%name%/
cp atxpkg_diff.exe pkg/%name%/
cp atxpkg_sdiff.exe pkg/%name%/
cp atxpkg_setx.exe pkg/%name%/
cp atxpkg_vim.exe pkg/%name%/
cp repos.txt pkg/%name%/

rd /s /q build
rd /s /q dist

;rem hg parents --template "{latesttag}" >.version
git describe --tags --abbrev=0 >.version
set /p version=<.version
rm .version
set version=%version:~1%

;rem no, i can't do this inside of the if for some reason - FUCK WINDOWS!
awk "BEGIN {print strftime(\"%%Y%%m%%d%%H%%M%%S\")}" >.datetime
set /p datetime=<.datetime
rm .datetime

if "%1" == "" (
	echo devel version %datetime%

	set name=%name%.dev
	set version=%datetime%
	set upload=atxpkg@atxpkg-dev.asterix.cz:atxpkg/
) else if "%1" == "release" (
	echo release version %version%
	set upload=atxpkg@atxpkg.asterix.cz:atxpkg/
) else (
	echo unknown parameter!
	goto end
)

set pkg_fn=%name%-%version%-%pkgrel%.atxpkg.zip

rm %pkg_fn%

cd pkg
zip -r ../%pkg_fn% .
cd ..

rd /s /q pkg

pscp %pkg_fn% %upload%

:end
