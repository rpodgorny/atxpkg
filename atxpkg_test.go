package main

import (
	"fmt"
	"os"
	"slices"
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

func TestGetRecursiveListing(t *testing.T) {
	tmpDir := t.TempDir()
	if err := os.MkdirAll(tmpDir+"/some/directory", os.ModePerm); err != nil {
		t.Fatalf(fmt.Sprintf("%+v", err))
	}
	if _, err := os.Create(tmpDir+"/some/file1"); err != nil {
		t.Fatalf(fmt.Sprintf("%+v", err))
	}
	if _, err := os.Create(tmpDir+"/some/directory/file2"); err != nil {
		t.Fatalf(fmt.Sprintf("%+v", err))
	}
	dirs, files, err := GetRecursiveListing(tmpDir)
	if err != nil {
		t.Fatalf(fmt.Sprintf("%+v", err))
	}
	if !slices.Equal(dirs, []string{"some", "some/directory"}) {
		t.Errorf("wrong dirs: %v", dirs)
	}
	slices.Sort(files)
	slices.Reverse(files)
	if !slices.Equal(files, []string{"some/file1", "some/directory/file2"}) {
		t.Errorf("wrong files: %v", files)
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

func TestUpdatePackage(t *testing.T) {
	tmpDir := t.TempDir()
	installedPackage, err := UpdatePackage(
		"./test_data/atx300-base-6.3-1.atxpkg.zip",
		"atx300-base",
		InstalledPackage{
			Version: "3.3",
			Md5sums: map[string]string{},
			Backup: []string{},
		},
		tmpDir,
		false,
	)
	if err != nil {
		t.Fatalf(fmt.Sprintf("%+v", err))
	}
	// TODO: not really much testing going on here
	fmt.Println(installedPackage)
}
