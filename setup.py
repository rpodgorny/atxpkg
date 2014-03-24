from cx_Freeze import setup, Executable

from version import __version__


setup(
	name = 'atxpkg',
	version = __version__,
	options = {
		'build_exe': {
			'create_shared_zip': False,
			'compressed': True,
			'include_msvcr': True,
			#'include_files': ['captions.cs', ]
		},
	},
	executables = [
		Executable(
			script='atxpkg',
			appendScriptToExe=True,
			appendScriptToLibrary=False,
			compress=True,
		)
	]
)
