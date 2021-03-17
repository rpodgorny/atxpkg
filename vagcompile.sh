#!/bin/sh
set -e -x
rm -rf Vagrantfile
###vagrant init rpodgorny/mywindows-10 --box-version 1809.0.1904.2
echo 'Vagrant.configure("2") do |config|
  config.vm.box = "rpodgorny/mywindows-10"
  config.vm.box_version = "1809.0.1904.3"
  config.vm.provider "virtualbox" do |vb|
    vb.memory = 4096
    vb.cpus = 4
  end
end' >Vagrantfile
vagrant up
echo "rm -rf c:/build; mkdir -p c:/build; cp -rv c:/vagrant/* c:/build/" | vagrant ssh -c /bin/bash
echo "cd c:/build; ./compile_win.sh" | vagrant ssh -c /bin/bash
#echo "cd c:/build; cd dist; ./tests.exe" | vagrant ssh -c /bin/bash
#echo 'cd c:/build
#rm Pipfile.lock
#pipenv --rm
#pipenv install --dev
#pipenv install cx-freeze
#pipenv run python setup.py install --prefix=dist
#' | vagrant ssh --no-tty
vagrant ssh -c "cd c:/build; cp -rv dist c:/vagrant/" --no-tty
vagrant halt
vagrant destroy --force
rm Vagrantfile
