from cx_Freeze import setup, Executable

from version import __version__


setup(
	name = 'atxpkg',
	version = __version__,
	options = {
		'build_exe': {
			'include_msvcr': True,
		},
	},
	executables = [
		Executable(script='atxpkg'),
	]
)
