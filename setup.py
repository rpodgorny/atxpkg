from cx_Freeze import setup, Executable

from version import __version__

# TODO: actually fill this (it's cut-n-pasted from the windows setup.py)
setup(
	name = 'atxpkg',
	version = __version__,
	options={
		'build_exe': {
			'include_msvcr': True,
		},
	},
	executables = [
		Executable(
			script='atxpkg',
		),
	]
)
