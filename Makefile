all:
	go build

release:
	go build -a -v -trimpath -ldflags "-s -w"

windows_console:
	env GOOS=windows GOARCH=amd64 go build -a -v -trimpath -ldflags "-s -w"
