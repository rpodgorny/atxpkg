#!/bin/sh
set -e -x
rm -rf Vagrantfile
vagrant init rpodgorny/mywindows-10 --box-version 1809.0.1904.2
vagrant up
vagrant ssh -c "rm -rf c:/build; mkdir -p c:/build; cp -rv c:/vagrant/* c:/build/" --no-tty
echo 'cd c:/build
rm Pipfile.lock
pipenv --rm
pipenv install --dev
pipenv install cx-freeze
pipenv run python setup.py install --prefix=dist
' | vagrant ssh --no-tty
vagrant ssh -c "cd c:/build; cp -rv dist c:/vagrant/" --no-tty
# TODO: hack jako svine
cp -av /lib/python3.8/site-packages/dateutil dist/
vagrant halt
#vagrant destroy --force
#rm Vagrantfile
