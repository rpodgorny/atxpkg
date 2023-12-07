package main

import (
	"fmt"
	"testing"
)

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
