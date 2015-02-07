from version import __version__

# TODO: actually fill this (it's cut-n-pasted from the windows setup.py)
setup(
	name = 'atxpkg',
	version = __version__,
	executables = [
		Executable(
			script='atxpkg',
		),
	]
)
