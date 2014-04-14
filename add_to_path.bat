set p=c:\atxpkg

Echo.%PATH% | findstr /C:"%p%">nul && (
	echo already in path
) || (
	echo adding to %p% to path
	setx PATH "%PATH%;%p%" -m
)