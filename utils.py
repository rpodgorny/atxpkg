import re
from distutils.version import LooseVersion

def get_max_version_url(urls):
	return sorted(urls, key=lambda x: LooseVersion(get_package_version(get_package_fn(x))))[-1]
#enddef

def get_package_fn(url):
	return url.split('/')[-1]
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