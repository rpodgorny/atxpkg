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
#endclass


if __name__ == '__main__':
	unittest.main()
#endif
