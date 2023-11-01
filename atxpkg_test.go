package main

import (
	"fmt"
	"testing"
)

func TestYesNo(t *testing.T) {
	tests := map[string]struct {
		msg  string
		def  string
		want bool
	}{
		"yes": {
			msg:  "ahoj",
			def:  "y",
			want: true,
		},
		"no": {
			msg:  "ahoj",
			def:  "n",
			want: false,
		},
		// TODO: this loops forever
		/*"nothing": {
			msg: "ahoj",
			def: "",
			want: false,
		},*/
	}
	for name, tt := range tests {
		t.Run(name, func(t *testing.T) {
			ans := YesNo(tt.msg, tt.def)
			if ans != tt.want {
				t.Errorf("got %v, want %v", ans, tt.want)
			}
		})
	}
}

func TestGetAvailablePackages(t *testing.T) {
	// TODO: make it work offline
	packages := GetAvailablePackages(
		[]string{
			"http://atxpkg.asterix.cz",
			"http://atxpkg-dev.asterix.cz",
			"./test_data",
		},
		false,
	)
	// TODO: this does not test anything
	fmt.Printf("%v", packages)
}

func TestInstallPackage(t *testing.T) {
	tmpDir := t.TempDir()
	installedPackage, err := InstallPackage(
		"./test_data/atx300-base-6.3-1.atxpkg.zip",
		tmpDir,
		false,
	)
	if err != nil {
		t.Fatalf(fmt.Sprintf("%+v", err))
	}
	// TODO: not really much testing going on here
	fmt.Println(installedPackage)
}