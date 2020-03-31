#!/usr/bin/python3

import unittest
import os
import shutil
import subprocess
import tempfile

#from atxpkg import *
from utils import *


class HighLevelTestCase(unittest.TestCase):
	def setUp(self):
		# TODO: create temporary directory
		#self.d = tempfile.mkdtemp()
		self.d = '/tmp/atxpkg'
		if os.path.isdir(self.d):
			shutil.rmtree(self.d)
		os.mkdir(self.d)

		if os.path.isdir('/tmp/atxpkg_dest'):
			shutil.rmtree('/tmp/atxpkg_dest')

		with open('%s/repos.txt' % self.d, 'w') as f:
			f.write('http://atxpkg.asterix.cz\n')
			f.write('http://atxpkg-dev.asterix.cz\n')

	def tearDown(self):
		if os.path.isdir(self.d):
			shutil.rmtree(self.d)
		if os.path.isdir('/tmp/atxpkg_dest'):
			shutil.rmtree('/tmp/atxpkg_dest')

	def test_install(self):
		subprocess.check_call('./atxpkg.sh install atxpkg --yes --prefix=/tmp/atxpkg_dest', shell=True)
		subprocess.check_call('./atxpkg.sh update atxpkg..router --yes --prefix=/tmp/atxpkg_dest', shell=True)
		subprocess.check_call('./atxpkg.sh remove router --yes --prefix=/tmp/atxpkg_dest', shell=True)


class TestCase(unittest.TestCase):
	def setUp(self):
		self.d = '/tmp/atxpkg'
		if os.path.isdir(self.d):
			shutil.rmtree(self.d)
		os.mkdir(self.d)

	def tearDown(self):
		if os.path.isdir(self.d):
			shutil.rmtree(self.d)
		if os.path.isdir('/tmp/atxpkg_dest'):
			shutil.rmtree('/tmp/atxpkg_dest')

	# TODO: this is hardly finished
	def test_install_update_remove(self):
		fn = os.path.abspath('test_data/atxpkg-1.5-3.atxpkg.zip')
		installed_package = install_package(fn, self.d)
		installed_packages = {}
		installed_packages[get_package_name(fn)] = installed_package
		update_package(fn, get_package_name(fn), installed_package, self.d)
		remove_package(get_package_name(fn), installed_packages, self.d)

	def test_install_empty_dirs(self):
		fn = os.path.abspath('test_data/atx300-base.dev-0-1.atxpkg.zip')
		installed_package = install_package(fn, self.d)
		installed_packages = {}
		installed_packages[get_package_name(fn)] = installed_package
		self.assertTrue(os.path.isdir('/tmp/atxpkg/atx300'))
		self.assertTrue(os.path.isdir('/tmp/atxpkg/atx300/comm/dis_man'))
		self.assertTrue(os.path.isdir('/tmp/atxpkg/atx300/comm/dis_man/archive'))
		self.assertTrue(os.listdir('/tmp/atxpkg/atx300/comm/dis_man/archive') == [])

	def test_install_update_empty_dirs(self):
		fn = os.path.abspath('test_data/atx300-base-6.3-1.atxpkg.zip')
		fn2 = os.path.abspath('test_data/atx300-base.dev-0-1.atxpkg.zip')
		installed_package = install_package(fn, self.d)
		installed_packages = {}
		installed_packages[get_package_name(fn)] = installed_package
		update_package(fn2, get_package_name(fn), installed_package, self.d)
		self.assertTrue(os.path.isdir('/tmp/atxpkg/atx300'))
		self.assertTrue(os.path.isdir('/tmp/atxpkg/atx300/comm/dis_man'))
		self.assertTrue(os.path.isdir('/tmp/atxpkg/atx300/comm/dis_man/archive'))
		self.assertTrue(os.listdir('/tmp/atxpkg/atx300/comm/dis_man/archive') == [])


class UtilsCase(unittest.TestCase):
	def test_package_name(self):
		self.assertEqual(get_package_name('package-3.5.6-1.atxpkg.zip'), 'package')
		self.assertEqual(get_package_name('package-3.5.6-1'), 'package')
		self.assertEqual(get_package_name('package'), 'package')

		self.assertEqual(get_package_name('package-name-3.5.6-1.atxpkg.zip'), 'package-name')
		self.assertEqual(get_package_name('package-name-3.5.6-1'), 'package-name')
		self.assertEqual(get_package_name('package-name'), 'package-name')

	def test_package_version(self):
		self.assertEqual(get_package_version('package-3.5.6-1.atxpkg.zip'), '3.5.6-1')
		self.assertEqual(get_package_version('package-with-hyphen-3.5.6-1.atxpkg.zip'), '3.5.6-1')

		self.assertEqual(get_package_version('package'), None)
		self.assertEqual(get_package_version('package-with-hyphen'), None)

	def test_valid_package_fn(self):
		self.assertTrue(is_valid_package_fn('package-3.5.6-6.atxpkg.zip'))
		self.assertTrue(is_valid_package_fn('package-name-3.5.6-6.atxpkg.zip'))
		self.assertTrue(is_valid_package_fn('package_name-3.5.6-6.atxpkg.zip'))
		self.assertTrue(is_valid_package_fn('package-name.dev-3.5.6-6.atxpkg.zip'))
		self.assertTrue(is_valid_package_fn('package-name-3-6.atxpkg.zip'))
		self.assertTrue(is_valid_package_fn('package.dev-20150101145536-6.atxpkg.zip'))
		self.assertTrue(is_valid_package_fn('package-name.dev-20150101145536-6.atxpkg.zip'))

		self.assertFalse(is_valid_package_fn('package-xxx-6.atxpkg.zip'))
		self.assertFalse(is_valid_package_fn('package-name-xxx-xxx.atxpkg.zip'))
		self.assertFalse(is_valid_package_fn('package-name-3-6.xxx.zip'))

	def test_max_version(self):
		urls = [
			'http://example.com/repo/package-2.2-3.atxpkg.zip',
			'http://example.com/repo/package-1.2-3.atxpkg.zip',
			'http://example.com/repo/package-1.2-4.atxpkg.zip',
			'http://example.com/repo/package-1.2222-4.atxpkg.zip',
			'http://example.com/repo/package-1.2222-44444.atxpkg.zip',
		]
		self.assertEqual(get_max_version_url(urls), 'http://example.com/repo/package-2.2-3.atxpkg.zip')


if __name__ == '__main__':
	unittest.main()
