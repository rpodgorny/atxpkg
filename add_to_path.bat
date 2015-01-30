set p=c:\atxpkg

Echo.%PATH% | findstr /C:"%p%">nul && (
	echo %p% already in path
) || (
	echo adding to %p% to path
	atxpkg_setx PATH "%PATH%;%p%" -m
)
