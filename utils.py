import sys
import logging
import datetime
import re
from distutils.version import LooseVersion


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