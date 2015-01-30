import re
from distutils.version import LooseVersion

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