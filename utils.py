import sys
import logging
import datetime
import re
import os
import shutil
import json
import hashlib
import re
import glob
import tempfile
import subprocess
import urllib.request
from distutils.version import LooseVersion


if sys.platform == 'win32':
	BIN_7ZIP = '/atxpkg/atxpkg_7za.exe'
else:
	BIN_7ZIP = '7za'
#endif


# TODO: this is cut-n-pasted from atxutils - decouple from atx
def logging_setup(level, fn=None, print_=True):
	sys.excepthook = lambda type, value, traceback: logging.critical('unhandled exception', exc_info=(type, value, traceback))

	logger = logging.getLogger()
	logger.setLevel(logging.DEBUG)

	class AtxFormatter(logging.Formatter):
		def formatTime(self, record, datefmt=None):
			ct = datetime.datetime.fromtimestamp(record.created)
			if datefmt:
				s = ct.strftime(datefmt)
			else:
				s = ct.strftime('%Y-%m-%d %H:%M:%S.%f')
			#endif
			return s
		#enddef
	#endclass

	formatter = AtxFormatter('%(asctime)s: %(levelname)s: %(message)s')

	if print_:
		sh = logging.StreamHandler()
		sh.setLevel(level)
		sh.setFormatter(formatter)
		logger.addHandler(sh)
	else:
		nh = logging.NullHandler()
		logger.addHandler(nh)
	#endif

	if fn:
		fh = logging.FileHandler(fn)
		fh.setLevel(level)
		fh.setFormatter(formatter)
		logger.addHandler(fh)
	#endif
#enddef

def print_(str):
	print(str)
	logging.info(str)
#enddef

def get_repos(fn):
	ret = []

	for line in open(fn, 'r'):
		line = line.strip()
		if not line: continue
		if line.startswith('#'): continue
		if line.startswith(';'): continue
		ret.append(line)
	#endfor

	return ret
#enddef

def get_available_packages(repos):
	ret = {}

	for repo in repos:
		package_urls = get_repo_listing(repo)
		#logging.debug(str(package_urls))

		for package_url in package_urls:
			package_fn = get_package_fn(package_url)

			if not is_valid_package_fn(package_fn):
				logging.debug('%s not valid package filename' % package_fn)
				continue
			#endif

			package_name = get_package_name(package_fn)
			package_version = get_package_version(package_fn)
			#print(package_name, package_version)

			if package_name in ret:
				ret[package_name].append(package_url)
			else:
				ret[package_name] = [package_url, ]
			#endif
		#endfor
	#endfor

	return ret
#enddef

def parse_index_html(html):
	ret = re.findall('[\w\-\._:/]+\.atxpkg\.\w+', html)
	return ret
#enddef

def get_repo_listing(repo):
	logging.debug('getting repo listing from %s' % repo)

	if repo.startswith('http://'):
		try:
			r = urllib.request.urlopen(repo)
		except:
			logging.error('failed to get listing from %s' % repo)
			return []
		#endtry

		files = parse_index_html(r.read().decode())
		return ['%s/%s' % (repo, f) for f in files]
	else:
		lst = glob.glob('%s/*' % repo)
		lst = [i.replace('\\', '/') for i in lst]
		lst = [i for i in lst if '.atxpkg.' in i]  # TODO: convert to more exact match
		return lst
	#endif
#enddef

def download_package(url, cache_dir):
	if url.startswith('http://'):
		fn = '%s/%s' % (cache_dir, get_package_fn(url))

		if not os.path.isfile(fn):
			print_('downloading %s to %s' % (url, fn))
			urllib.request.urlretrieve(url, fn)
		else:
			print_('using cached %s' % fn)
		#endif

		return fn
	else:
		return url
	#endif
#enddef

# TODO: possibly use 'atxpkg_inuse.exe'
def try_delete(fn):
	if not os.path.isfile(fn): return

	del_fn = '%s.atxpkg_delete' % fn
	while os.path.isfile(del_fn):
		try:
			os.remove(del_fn)
			break
		except:
			pass
		#endtry

		del_fn += '_delete'
	#endwhile

	os.rename(fn, del_fn)

	try:
		os.remove(del_fn)
	except:
		pass
	#endtry
#enddef

def install_package(fn, prefix, force=False):
	name = get_package_name(get_package_fn(fn))
	version_new = get_package_version(get_package_fn(fn))

	print_('installing %s-%s' % (name, version_new))

	ret = {}
	ret['version'] = get_package_version(get_package_fn(fn))
	ret['md5sums'] = {}

	cwd = os.getcwd()
	try:
		tmpdir = tempfile.mkdtemp()
		logging.debug('tmpdir is %s' % tmpdir)

		os.chdir(tmpdir)
		unzip(fn)

		files_to_backup = []
		if os.path.isfile('.atxpkg_backup'):
			files_to_backup = getlines('.atxpkg_backup')
		#endif

		ret['backup'] = files_to_backup

		dirs, files = get_recursive_listing(tmpdir)

		if not force:
			for f in files:
				f = '%s/%s' % (prefix, f)
				if os.path.isfile(f):
					raise Exception('%s already exists!' % f)
				#endif
			#endfor
		#endif

		for d in dirs:
			try:
				os.makedirs('%s/%s' % (prefix, d))
			except: pass
		#endfor

		for f in files:
			if f.startswith('.atxpkg_'): continue

			if os.path.isfile('%s/%s' % (prefix, f)) and f in files_to_backup:
				# TODO: only backup when sum differs
				print_('saving untracked %s/%s as %s/%s.atxpkg_save' % (prefix, f, prefix, f))
				logging.debug('S %s/%s %s/%s.atxpkg_save' % (prefix, f, prefix, f))
				shutil.move('%s/%s' % (prefix, f), '%s/%s.atxpkg_save' % (prefix, f))
			#endif

			ret['md5sums'][f] = get_md5sum(f)
			try:
				os.makedirs(os.path.dirname('%s/%s' % (prefix, f)))
			except: pass

			logging.debug('I %s/%s' % (prefix, f))
			try_delete('%s/%s' % (prefix, f))
			#shutil.move(f, '%s/%s' % (prefix, f))
			shutil.copy(f, '%s/%s' % (prefix, f))
		#endfor
	finally:
		os.chdir(cwd)
		shutil.rmtree(tmpdir)
	#endtry

	return ret
#endif

def update_package(fn, name_old, installed_package, prefix, force=False):
	name = get_package_name(get_package_fn(fn))
	version_old = installed_package['version']
	version_new = get_package_version(get_package_fn(fn))

	print_('updating %s-%s -> %s-%s' % (name_old, version_old, name, version_new))

	ret = {}
	ret['version'] = version_new
	ret['md5sums'] = {}

	cwd = os.getcwd()
	try:
		tmpdir = tempfile.mkdtemp()

		os.chdir(tmpdir)
		unzip(fn)

		files_to_backup = []
		if os.path.isfile('.atxpkg_backup'):
			files_to_backup = getlines('.atxpkg_backup')
		#endif

		ret['backup'] = files_to_backup

		dirs, files = get_recursive_listing(tmpdir)

		if not force:
			for f in files:
				if os.path.isfile('%s/%s' % (prefix, f)) and not f in installed_package['md5sums']:
					raise Exception('%s/%s exists in filesystem but is not part of original package' % (prefix, f))
				#endif
			#endfor
		#endif

		for f in files:
			if f.startswith('.atxpkg_'): continue

			sum_new = get_md5sum(f)

			ret['md5sums'][f] = sum_new
			try:
				os.makedirs(os.path.dirname('%s/%s' % (prefix, f)))
			except: pass

			if os.path.isfile('%s/%s' % (prefix, f)):
				skip = False
				backup = False

				if f in files_to_backup:
					sum_current = get_md5sum('%s/%s' % (prefix, f))
					sum_original = installed_package['md5sums'][f]

					if sum_original == sum_new:
						skip = True
					elif sum_current == sum_new:
						pass
					else:
						backup = True
					#endif
				#endif

				if skip:
					logging.debug('S %s/%s' % (prefix, f))
				elif backup:
					print_('sum for file %s/%s changed, installing new version as %s/%s.atxpkg_new' % (prefix, f, prefix, f))
					logging.debug('I %s/%s.atxpkg_new' % (prefix, f))
					#shutil.move(f, '%s/%s.atxpkg_new' % (prefix, f))
					shutil.copy(f, '%s/%s.atxpkg_new' % (prefix, f))
				else:
					logging.debug('U %s/%s' % (prefix, f))
					try_delete('%s/%s' % (prefix, f))
					#shutil.move(f, '%s/%s' % (prefix, f))
					shutil.copy(f, '%s/%s' % (prefix, f))
				#endif
			else:
				logging.debug('I %s/%s' % (prefix, f))
				try_delete('%s/%s' % (prefix, f))
				#shutil.move(f, '%s/%s' % (prefix, f))
				shutil.copy(f, '%s/%s' % (prefix, f))
			#endif
		#endfor

		# remove files which are no longer in the new version

		if 'backup' in installed_package:
			files_to_backup_old = installed_package['backup']
		else:
			files_to_backup_old = []
		#endif

		for fn, md5sum in installed_package['md5sums'].items():
			if fn in ret['md5sums']: continue

			if not os.path.isfile('%s/%s' % (prefix, fn)):
				logging.warning('%s/%s does not exist!' % (prefix, fn))
				continue
			#endif

			backup = False
			if fn in files_to_backup_old:
				sum_current = get_md5sum('%s/%s' % (prefix, fn))
				sum_original = md5sum
				if sum_current != sum_original:
					backup = True
				#endif
			#endif

			if backup:
				print_('saving changed %s/%s as %s/%s.atxpkg_save' % (prefix, fn, prefix, fn))
				logging.debug('S %s/%s %s/%s.atxpkg_save' % (prefix, fn, prefix, fn))
				shutil.move('%s/%s' % (prefix, fn), '%s/%s.atxpkg_save' % (prefix, fn))
			else:
				logging.debug('D %s/%s' % (prefix, fn))
				os.remove('%s/%s' % (prefix, fn))
			#endif

			try:
				os.removedirs(os.path.dirname('%s/%s' % (prefix, fn)))
			except: pass
		#endfor
	finally:
		os.chdir(cwd)
		shutil.rmtree(tmpdir)
	#endtry

	return ret
#endif

def remove_package(package_name, installed_packages, prefix):
	version = installed_packages[package_name]['version']
	print_('removing package %s: %s' % (package_name, version))

	package_info = installed_packages[package_name]

	if 'backup' in package_info:
		files_to_backup_old = package_info['backup']
	else:
		files_to_backup_old = []
	#endif

	for fn, md5sum in package_info['md5sums'].items():
		if not os.path.isfile('%s/%s' % (prefix, fn)):
			logging.warning('%s/%s does not exist!' % (prefix, fn))
			continue
		#endif

		backup = False
		if fn in files_to_backup_old:
			current_sum = get_md5sum('%s/%s' % (prefix, fn))
			original_sum = md5sum
			if current_sum != original_sum:
				backup = True
			#endif
		#endif

		if backup:
			print_('%s/%s changed, keeping old backup' % (prefix, fn))
			logging.debug('M %s/%s %s/%s.atxpkg_backup' % (prefix, fn, prefix, fn))
			os.rename('%s/%s' % (prefix, fn), '%s/%s.atxpkg_backup' % (prefix, fn))
		else:
			logging.debug('D %s/%s' % (prefix, fn))
			try_delete('%s/%s' % (prefix, fn))
		#endif

		try:
			os.removedirs(os.path.dirname('%s/%s' % (prefix, fn)))
		except: pass
	#endfor
#enddef

def mergeconfig_package(package_name, installed_packages, prefix):
	package_info = installed_packages[package_name]

	if 'backup' in package_info:
		files_to_backup = package_info['backup']
	else:
		files_to_backup = []
	#endif

	for fn in files_to_backup:
		for suffix in ['atxpkg_backup', 'atxpkg_new', 'atxpkg_save']:
			fn_full = '%s/%s' % (prefix, fn)
			fn_from_full = '%s.%s' % (fn_full, suffix)

			if os.path.isfile(fn_from_full):
				logging.debug('found %s, running merge' % fn_from_full)

				if yes_no('found %s, merge?' % fn_from_full, 'y'):
					merge(fn_full, fn_from_full)

					# just prints the diff
					#diff(fn_full, fn_from_full)

					if yes_no('delete %s?' % fn_from_full):
						logging.debug('D %s' % fn_from_full)
						os.remove(fn_from_full)
					#endif
				#endif
			#endif
		#endfor
	#endfor
#enddef

def yes_no(s, default=None):
	if default == 'y':
		q = '%s [Y/n] ' % s
	elif default == 'n':
		q = '%s [y/N] ' % s
	else:
		q = '%s [y/n] ' % s
	#endif

	while 1:
		ans = input(q).lower()

		if ans == 'y':
			return True
		elif ans == 'n':
			return False
		elif ans == '' and default == 'y':
			return True
		elif ans == '' and default == 'n':
			return False
		#endif
	#endwhile
#enddef

def merge(fn1, fn2):
	cmd = 'vim -d %s %s' % (fn1, fn2)
	return subprocess.call(cmd, shell=True) == 0
#enddef

#def diff(fn1, fn2):
#	cmd = '/atxpkg/atxpkg_diff.exe -u %s %s' % (fn1, fn2)
#	return subprocess.call(cmd, shell=True)
#enddef

def get_md5sum(fn):
	with open(fn, 'rb') as f:
		return hashlib.md5(f.read()).hexdigest()
	#endwith
#enddef

def get_recursive_listing(path):
	ret_d = []
	ret_f = []

	for root, dirs, files in os.walk(path):
		root = root.replace('\\', '/')  # TODO: not very nice

		for d in dirs:
			ret_d.append('%s/%s' % (root, d))
		#endfor

		for f in files:
			ret_f.append('%s/%s' % (root, f))
		#endfor
	#endfor

	# cut the tempdir prefix
	ret_d = [i[len(path) + 1:] for i in ret_d]
	ret_f = [i[len(path) + 1:] for i in ret_f]

	return ret_d, ret_f
#enddef

def get_installed_packages(db_fn):
	if not os.path.isfile(db_fn):
		raise Exception('package database not found (%s)' % db_fn)
	#endif

	return json.load(open(db_fn, 'r'))
#enddef

def save_installed_packages(l, db_fn):
	json.dump(l, open(db_fn, 'w'), indent=4)
#enddef

def get_specific_version_url(urls, version):
	for url in urls:
		if get_package_version(get_package_fn(url)) == version:
			return url
		#endif
	#endfor

	return None
#enddef

def getlines(fn):
	with open(fn, 'r') as f:
		ret = f.readlines()
	#endwith

	ret = [i.strip() for i in ret]
	ret = [i for i in ret if i]
	return ret
#enddef

def clean_cache(cache_dir):
	for fn in os.listdir(cache_dir):
		logging.debug('D %s/%s' % (cache_dir, fn))
		os.remove('%s/%s' % (cache_dir, fn))
	#endfor
#enddef

def gen_fn_to_package_name_mapping(installed_packages, prefix):
	ret = {}

	for package_name, package_info in installed_packages.items():
		for fn in package_info['md5sums'].keys():
			ret['%s/%s' % (prefix, fn)] = package_name
		#endfor
	#endfor

	return ret
#enddef

def get_repos(fn):
	ret = []

	for line in open(fn, 'r'):
		line = line.strip()
		if not line: continue
		if line.startswith('#'): continue
		if line.startswith(';'): continue
		ret.append(line)
	#endfor

	return ret
#enddef

def parse_index_html(html):
	ret = re.findall('[\w\-\._:/]+\.atxpkg\.\w+', html)
	return ret
#enddef

def get_repo_listing(repo):
	logging.debug('getting repo listing from %s' % repo)

	if repo.startswith('http://'):
		try:
			r = urllib.request.urlopen(repo)
		except:
			logging.error('failed to get listing from %s' % repo)
			return []
		#endtry

		files = parse_index_html(r.read().decode())
		return ['%s/%s' % (repo, f) for f in files]
	else:
		lst = glob.glob('%s/*' % repo)
		lst = [i.replace('\\', '/') for i in lst]
		lst = [i for i in lst if '.atxpkg.' in i]  # TODO: convert to more exact match
		return lst
	#endif
#enddef

def download_package(url, cache_dir):
	if url.startswith('http://'):
		fn = '%s/%s' % (cache_dir, get_package_fn(url))

		if not os.path.isfile(fn):
			print_('downloading %s to %s' % (url, fn))
			urllib.request.urlretrieve(url, fn)
		else:
			print_('using cached %s' % fn)
		#endif

		return fn
	else:
		return url
	#endif
#enddef

def unzip(fn):
	print_('unzipping %s' % fn)

	#cmd = 'unzip -q %s' % (fn, )
	cmd = '%s x %s' % (BIN_7ZIP, fn)
	logging.debug(cmd)
	subprocess.check_call(cmd, shell=True, stdout=subprocess.DEVNULL)
#enddef

def get_available_packages(repos):
	ret = {}

	for repo in repos:
		package_urls = get_repo_listing(repo)
		#logging.debug(str(package_urls))

		for package_url in package_urls:
			package_fn = get_package_fn(package_url)

			if not is_valid_package_fn(package_fn):
				logging.debug('%s not valid package filename' % package_fn)
				continue
			#endif

			package_name = get_package_name(package_fn)
			package_version = get_package_version(package_fn)
			#print(package_name, package_version)

			if package_name in ret:
				ret[package_name].append(package_url)
			else:
				ret[package_name] = [package_url, ]
			#endif
		#endfor
	#endfor

	return ret
#enddef

def get_max_version_url(urls):
	map_ = {get_package_version(get_package_fn(url)): url for url in urls}
	# TODO: replacing '-' with '.' is a hack. looseversion is unable to handle it otherwise
	max_version = sorted(map_.keys(), key=lambda x: LooseVersion(x.replace('-', '.')))[-1]
	return map_[max_version]
#enddef

def get_package_fn(url):
	fn = url.split('/')[-1]
	return fn
#enddef

# TODO: fn is not really fn here
def get_package_name(fn):
	fn = re.sub('\.atxpkg\..*', '', fn)

	# TODO: this is actually not really nice
	try:
		name, ver, rel = fn.rsplit('-', 2)
		return name
	except:
		return fn
	#endtry
#enddef

# TODO: fn is not really fn here
def get_package_version(fn):
	fn = re.sub('\.atxpkg\..*', '', fn)

	# TODO: this is actually not really nice
	if has_version(fn):
		name, ver, rel = fn.rsplit('-', 2)
		return '%s-%s' % (ver, rel)
	else:
		return None
	#endtry
#enddef

def is_valid_package_fn(fn):
	return re.match('[\w\-\.]+-[\d.]+-\d+\.atxpkg\.zip', fn) is not None
#enddef

def has_version(pkg_spec):
	return re.match('[\w\-\.]+-[\d.]+-\d+', pkg_spec) is not None
#enddef
