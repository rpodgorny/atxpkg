import re


def get_package_fn(url):
	return url.split('/')[-1]
#enddef

def get_package_name(fn):
	fn = re.sub('\.atxpkg\..*', '', fn)
	name, ver, rel = fn.rsplit('-', 2)
	return name
#enddef

def get_package_version(fn):
	fn = re.sub('\.atxpkg\..*', '', fn)
	name, ver, rel = fn.rsplit('-', 2)
	return '%s-%s' % (ver, rel)
#enddef

def is_valid_package_fn(fn):
	return re.match('[\w\-\.]+-[\d.]+-\d+\.atxpkg\.zip', fn) is not None
#enddef