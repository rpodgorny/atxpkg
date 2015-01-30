#!/usr/bin/python3

import unittest

from utils import *

class MyTestCase(unittest.TestCase):
	def test_package_version(self):
		self.assertEqual(get_package_version('package-3.5.6-1.atxpkg.zip'), '3.5.6-1')
	#enddef

	def test_package_version_hyphen(self):
		self.assertEqual(get_package_version('package-with-hyphen-3.5.6-1.atxpkg.zip'), '3.5.6-1')
	#enddef

	def test_valid_package_fn(self):
		self.assertTrue(is_valid_package_fn('package-3.5.6-6.atxpkg.zip'))
		self.assertTrue(is_valid_package_fn('package-name-3.5.6-6.atxpkg.zip'))
		self.assertTrue(is_valid_package_fn('package_name-3.5.6-6.atxpkg.zip'))
		self.assertTrue(is_valid_package_fn('package-name.dev-3.5.6-6.atxpkg.zip'))
		self.assertTrue(is_valid_package_fn('package-name-3-6.atxpkg.zip'))

		self.assertFalse(is_valid_package_fn('package-xxx-6.atxpkg.zip'))
		self.assertFalse(is_valid_package_fn('package-name-xxx-xxx.atxpkg.zip'))
		self.assertFalse(is_valid_package_fn('package-name-3-6.xxx.zip'))
	#enddef
#endclass


if __name__ == '__main__':
	unittest.main()
#endif
