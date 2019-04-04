#!/usr/bin/python3

'''
Asterix package manager.

Usage:
  atxpkg install [options] <package>...
  atxpkg update [options] [<package>...] [<old_package..new_package>...]
  atxpkg remove [options] <package>...
  atxpkg check [options] [<package>...]
  atxpkg merge_config [options] [<package>...]
  atxpkg list_available [options]
  atxpkg list_installed [options]
  atxpkg show_untracked [options] [--recursive] [<path>]
  atxpkg clean_cache [options]

Options:
  --debug            Enable debug mode.
  --force            Force operation (overwrite files etc.)
  -w,--downloadonly  Only download packages, don't install/update anything.
  -h,--help          This screen.
  --prefix=<path>    Path prefix.
  --recursive        Recurse to directories.
  -y,--yes           Automatically answer yes to all questions.
  -n,--no            Automatically answer no to all questions.

'''

from version import __version__

import sys
import docopt
import logging
import os
from utils import *


def main():
	args = docopt.docopt(__doc__, version=__version__)
	log_level = 'DEBUG' if args['--debug'] else 'INFO'
	if sys.platform == 'win32':
		log_fn = 'c:/atxpkg/atxpkg.log'
	else:
		log_fn = '/tmp/atxpkg/atxpkg.log'
	logging_setup(log_level, log_fn, print_=True)
	logging.info('*' * 40)
	logging.info('starting atxpkg v%s' % __version__)
	logging.debug('args: %s' % dict(args))
	if sys.platform == 'win32':
		logging.debug('detected win32')
		db_fn = 'c:/atxpkg/installed.json'
		repos_fn = 'c:/atxpkg/repos.txt'
		prefix = 'c:'
		cache_dir = 'c:/atxpkg/cache'
	else:
		logging.debug('detected non-win32')
		db_fn = '/tmp/atxpkg/installed.json'
		repos_fn = '/tmp/atxpkg/repos.txt'
		prefix = ''
		cache_dir = '/tmp/atxpkg/cache'
	repos = get_repos(repos_fn)
	repos.append(cache_dir)
	#logging.debug(str(args))
	prefix = args['--prefix'] if args['--prefix'] else ''
	if not os.path.isfile(db_fn):
		logging.info('%s not found, creating empty one' % db_fn)
		with open(db_fn, 'w') as f:
			f.write('{}')
	if not os.path.isdir(cache_dir):
		logging.info('%s not found, creating empty one' % cache_dir)
		os.makedirs(cache_dir)
	installed_packages = get_installed_packages(db_fn)
	force = args['--force']
	yes, no = args['--yes'], args['--no']
	if args['install']:
		available_packages = get_available_packages(repos)
		for package in args['<package>']:
			package_name = get_package_name(package)
			if not package_name in available_packages:
				raise Exception('unable to find package %s' % package_name)
			if package_name in installed_packages and not force:
				raise Exception('package %s already installed' % package_name)
		for package in args['<package>']:
			package_name = get_package_name(package)
			package_version = get_package_version(package)
			if package_version:
				url = get_specific_version_url(available_packages[package_name], package_version)
			else:
				url = get_max_version_url(available_packages[package_name])
			ver = get_package_version(get_package_fn(url))
			print('install %s-%s' % (package_name, ver))
		if no or not (yes or yes_no('continue?', default='y')):
			return
		for package in args['<package>']:
			package_name = get_package_name(package)
			package_version = get_package_version(package)
			if package_version:
				url = get_specific_version_url(available_packages[package_name], package_version)
			else:
				url = get_max_version_url(available_packages[package_name])
			local_fn = download_package(url, cache_dir)
			if not args['--downloadonly']:
				package_info = install_package(local_fn, prefix, force)
				installed_packages[package_name] = package_info
				save_installed_packages(installed_packages, db_fn)
	elif args['update']:
		available_packages = get_available_packages(repos)
		if args['<package>']:
			packages = args['<package>']
			for package in packages:
				if '..' in package:
					package_old, package_new = package.split('..')
					package_name_old = get_package_name(package_old)
					package_name_new = get_package_name(package_new)
				else:
					package_name_old = package_name_new = get_package_name(package)

				if not package_name_old in installed_packages:
					raise Exception('package %s not installed' % package_name_old)
		else:
			packages = installed_packages.keys()
		packages_to_update = set()
		for package in packages:
			if '..' in package:
				package_old, package_new = package.split('..')
				package_name_old = get_package_name(package_old)
				package_name_new = get_package_name(package_new)
				package_version = get_package_version(package_new)
			else:
				package_name_old = package_name_new = get_package_name(package)
				package_version = get_package_version(package)
			if not package_name_new in available_packages:
				logging.warning('%s not available in any repository' % package_name_new)
				continue
			if package_version:
				url = get_specific_version_url(available_packages[package_name_new], package_version)
			else:
				url = get_max_version_url(available_packages[package_name_new])
			ver_cur = installed_packages[package_name_old]['version']
			ver_avail = get_package_version(get_package_fn(url))
			if package_name_old != package_name_new or ver_avail != ver_cur or force:
				print('update %s-%s -> %s-%s' % (package_name_old, ver_cur, package_name_new, ver_avail))
				packages_to_update.add(package)
		if not packages_to_update:
			print('nothing to update')
			return
		if no or not (yes or yes_no('continue?', default='y')):
			return
		for package in packages_to_update:
			if '..' in package:
				package_old, package_new = package.split('..')
				package_name_old = get_package_name(package_old)
				package_name_new = get_package_name(package_new)
				package_version = get_package_version(package_new)
			else:
				package_name_old = package_name_new = get_package_name(package)
				package_version = get_package_version(package)
			if package_version:
				url = get_specific_version_url(available_packages[package_name_new], package_version)
			else:
				url = get_max_version_url(available_packages[package_name_new])
			ver_cur = installed_packages[package_name_old]['version']
			ver_avail = get_package_version(get_package_fn(url))
			if package_name_old != package_name_new or ver_avail != ver_cur or force:
				local_fn = download_package(url, cache_dir)
				if not args['--downloadonly']:
					package_info = update_package(local_fn, package_name_old, installed_packages[package_name_old], prefix, force)
					del installed_packages[package_name_old]
					installed_packages[package_name_new] = package_info
					save_installed_packages(installed_packages, db_fn)
	elif args['merge_config']:
		if args['<package>']:
			packages = args['<package>']
			for package in packages:
				package_name = get_package_name(package)
				if not package_name in installed_packages:
					raise Exception('package %s not installed' % package_name)
		else:
			packages = installed_packages.keys()
		for package in packages:
			package_name = get_package_name(package)
			if not package_name in installed_packages:
				raise Exception('package %s not installed' % package_name)
		for package in packages:
			mergeconfig_package(package, installed_packages, prefix)
	elif args['remove']:
		for package_name in args['<package>']:
			if not package_name in installed_packages:
				raise Exception('package %s not installed' % package_name)
		for package_name in args['<package>']:
			package_version = installed_packages[package_name]['version']
			print('remove %s-%s' % (package_name, package_version))
		if no or not (yes or yes_no('continue?', default='n')):
			return
		for package_name in args['<package>']:
			remove_package(package_name, installed_packages, prefix)
			del installed_packages[package_name]
			save_installed_packages(installed_packages, db_fn)
	elif args['list_available']:
		available_packages = get_available_packages(repos)
		for package_name in sorted(available_packages.keys()):
			print(package_name)
	elif args['list_installed']:
		for package_name, package_info in installed_packages.items():
			package_version = package_info['version']
			print('%s-%s' % (package_name, package_version))
	elif args['show_untracked']:
		recursive = args['--recursive']
		fn_to_package_name = gen_fn_to_package_name_mapping(installed_packages, prefix)
		if args['<path>']:
			paths = set([args['<path>'], ])
		else:
			paths = set()
			for fn in fn_to_package_name.keys():
				paths.add(os.path.dirname(fn))
		while paths:
			for path in paths.copy():
				for fn in os.listdir(path):
					if os.path.isdir('%s/%s' % (path, fn)) and not os.path.islink('%s/%s' % (path, fn)):
						if recursive:
							paths.add('%s/%s' % (path, fn))
						else:
							continue
					if '%s/%s' % (path, fn) in fn_to_package_name:
						continue
					print('%s/%s' % (path, fn))
				paths.remove(path)
	elif args['clean_cache']:
		clean_cache(cache_dir)
	logging.debug('exit')
	return 0


if __name__ == '__main__':
	try:
		sys.exit(main())
	except Exception as e:
		print('ERROR: %s' % str(e))
		raise