import re


def get_package_fn(url):
	return url.split('/')[-1]
#enddef


def get_package_name(fn):
	fn = re.sub('\.atxpkg\..*', '', fn)
	return fn.split('-', 1)[0]
#enddef


def get_package_version(fn):
	fn = re.sub('\.atxpkg\..*', '', fn)

	if '-' in fn:
		return fn.rsplit('-', 1)[1]
	else:
		return None
	#endif
#enddef