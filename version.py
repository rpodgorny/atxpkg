import time

__version__ = "4.2"


def main():
	v = __version__.split('.')
	while len(v) < 4:
		v.append('0')
	v[3] = str(int(time.time()))
	vstr = ','.join(v)

	author = ""
	company = "Asterix a.s."
	name = "atxpkg"
	fileName = f"{name}.exe"

	s = f"""VSVersionInfo(
	  ffi=FixedFileInfo(
		filevers=({vstr}),
		prodvers=({vstr}),
		mask=0x3f,
		flags=0x0,
		fileType=0x1,
		subtype=0x0,
		date=(0, 0)),
	  kids=[
		StringFileInfo([
		  StringTable(
			u'040904B0',
			[StringStruct(u'CompanyName', u'{company}'),
			 StringStruct(u'FileDescription', u''),
			 StringStruct(u'InternalName', u'{name}'),
			 StringStruct(u'LegalCopyright', u'{company}'),
			 StringStruct(u'OriginalFilename', u'{fileName}'),
			 StringStruct(u'ProductName', u'{name}'),
			 StringStruct(u'FileVersion', u'{vstr}'),
			 StringStruct(u'ProductVersion', u'{vstr}')])]),
		VarFileInfo([VarStruct(u'Translation', [1033, 1200])])])"""
	print(s)


if __name__ == '__main__':
	main()
