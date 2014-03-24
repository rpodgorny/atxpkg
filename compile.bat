setlocal

rd /s /q build
rd /s /q dist

;rem del *.pyc

;rem python setup.py py2exe
;rem python setup.py bdist
python setup.py install --prefix=dist

;rem del *.pyc

;rem rd /s /q build
;rem del dist\w9xpopen.exe

;rem copy dist\*.exe .\

;rem rd /s /q dist