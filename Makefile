all:
	go build

release:
	go build -a -v -trimpath -ldflags "-s -w"

windows_console:
	env GOOS=windows GOARCH=amd64 go build -a -v -trimpath -ldflags "-s -w"

test:
	go run gotest.tools/gotestsum@latest --format=dots

wasm:
	env GOOS=js GOARCH=wasm go build -a -v -trimpath -ldflags "-s -w" -o atxpkg.wasm

clean:
	rm -rf atxpkg atxpkg.exe atxpkg.wasm *.atxpkg.zip

updeps:
	go get -u ./...
	go mod tidy
